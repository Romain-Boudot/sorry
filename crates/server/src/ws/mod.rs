use axum::{
    extract::{Query, State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    response::Response,
};
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

use crate::auth;
use crate::state::AppState;
use shared::events::{ClientEvent, ServerEvent, Snapshot};
use shared::models::VoiceUserState;
use shared::permissions;
use std::collections::HashMap;

#[derive(Deserialize)]
pub struct WsQuery {
    token: String,
}

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> Response {
    let claims = match auth::verify_token(&query.token, &state.jwt_secret) {
        Ok(c) => c,
        Err(_) => {
            return Response::builder()
                .status(401)
                .body("Unauthorized".into())
                .unwrap();
        }
    };

    let user_id = claims.sub;
    ws.on_upgrade(move |socket| handle_socket(socket, state, user_id))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>, user_id: i64) {
    tracing::info!("User {} connected via WebSocket", user_id);

    // Subscribe AVANT tout broadcast pour ne perdre aucun event
    let mut rx = state.event_tx.subscribe();
    let mut last_snapshot_request = Instant::now();

    // Envoyer le snapshot initial AVANT de marquer online
    // pour éviter un flash UserOnline → UserOffline si le snapshot échoue
    if let Err(e) = send_snapshot(&state, &mut socket, user_id).await {
        tracing::warn!("Failed to send snapshot to user {}: {}", user_id, e);
        return;
    }

    // Marquer online seulement après snapshot réussi
    let is_first_connection = {
        let mut online = state.online_users.write().unwrap();
        let count = online.entry(user_id).or_insert(0);
        *count += 1;
        *count == 1
    };
    if is_first_connection {
        if let Ok(Some(user)) = crate::db::users::find_by_id(&state.db, user_id).await {
            state.broadcast(ServerEvent::UserOnline { user });
        }
    }

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        tracing::debug!("User {} sent: {}", user_id, &text[..text.len().min(200)]);
                        if let Err(e) = handle_client_event(&state, &mut socket, user_id, &text, &mut last_snapshot_request).await {
                            tracing::warn!("Error handling client event: {}", e);
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(seq_event) => {
                        tracing::debug!("Sending seq={} to user {}", seq_event.seq, user_id);
                        if let Ok(json) = serde_json::to_string(&seq_event) {
                            if socket.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // Le client a perdu des events (buffer overflow) → renvoyer un snapshot
                        tracing::warn!("User {} lagged by {} events, sending snapshot", user_id, n);
                        if send_snapshot(&state, &mut socket, user_id).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }

    cleanup_user(&state, user_id).await;
}

/// Décrémenter le compteur de connexions et nettoyer l'état vocal si dernière connexion
async fn cleanup_user(state: &AppState, user_id: i64) {
    let is_last_connection = {
        let mut online = state.online_users.write().unwrap();
        if let Some(count) = online.get_mut(&user_id) {
            *count -= 1;
            if *count == 0 {
                online.remove(&user_id);
                true
            } else {
                false
            }
        } else {
            true
        }
    };

    if is_last_connection {
        {
            let mut voice = state.voice_state.write().unwrap();
            for (channel_id, users) in voice.iter_mut() {
                if users.remove(&user_id).is_some() {
                    state.broadcast(ServerEvent::UserLeftVoice {
                        user_id,
                        channel_id: *channel_id,
                    });
                }
            }
        }

        state.broadcast(ServerEvent::UserOffline { user_id });
    }

    tracing::info!("User {} disconnected (last={})", user_id, is_last_connection);
}

/// Construit et envoie un snapshot complet au client
async fn send_snapshot(
    state: &AppState,
    socket: &mut WebSocket,
    user_id: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Toutes les queries DB en parallèle pour minimiser la latence
    let (user_opt, all_users, channels, groups, roles, user_perms, all_user_roles, server_settings) = tokio::try_join!(
        crate::db::users::find_by_id(&state.db, user_id),
        crate::db::users::list_all(&state.db),
        crate::db::channels::list_all(&state.db),
        crate::db::channel_groups::list_all(&state.db),
        crate::db::roles::list_all(&state.db),
        crate::db::roles::get_user_permissions(&state.db, user_id),
        crate::db::roles::get_all_user_roles(&state.db),
        crate::db::servers::get_settings(&state.db),
    )?;
    let user = user_opt.ok_or("user not found")?;

    let online_users: Vec<i64> = {
        let online = state.online_users.read().unwrap();
        online.keys().copied().collect()
    };

    let voice_state: HashMap<i64, HashMap<i64, VoiceUserState>> = {
        let vs = state.voice_state.read().unwrap();
        vs.iter()
            .map(|(cid, users)| (*cid, users.iter().map(|(uid, s)| (*uid, s.clone())).collect()))
            .collect()
    };

    let snapshot = Snapshot {
        seq: state.current_seq(),
        user,
        permissions: user_perms,
        users: all_users,
        online_users,
        channels,
        groups,
        roles,
        user_roles: all_user_roles,
        voice_state,
        server_name: server_settings.0,
        server_description: server_settings.1,
        server_icon_url: server_settings.2,
        max_file_size: state.max_file_size,
    };

    let json = serde_json::to_string(&serde_json::json!({
        "type": "Snapshot",
        "data": snapshot,
    }))?;
    socket.send(Message::Text(json.into())).await?;
    Ok(())
}

async fn check_channel_permission(
    state: &AppState,
    user_id: i64,
    channel_id: i64,
    permission: i64,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let role_perms = crate::db::roles::get_user_permissions(&state.db, user_id).await?;
    if permissions::has(role_perms, permissions::ADMINISTRATOR) {
        return Ok(true);
    }
    let (allow, deny) = crate::db::roles::get_channel_overwrites(&state.db, user_id, channel_id).await?;
    let final_perms = permissions::compute(&[role_perms], allow, deny);
    Ok(permissions::has(final_perms, permission))
}

async fn handle_client_event(
    state: &AppState,
    socket: &mut WebSocket,
    user_id: i64,
    text: &str,
    last_snapshot_request: &mut Instant,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event: ClientEvent = serde_json::from_str(text)?;

    match event {
        ClientEvent::RequestSnapshot => {
            // Rate limit: max 1 snapshot toutes les 5 secondes par client
            if last_snapshot_request.elapsed() < std::time::Duration::from_secs(5) {
                tracing::warn!("User {} snapshot request rate-limited", user_id);
                return Ok(());
            }
            *last_snapshot_request = Instant::now();
            send_snapshot(state, socket, user_id).await?;
            return Ok(());
        }
        ClientEvent::SendMessage { channel_id, content, reply_to_id } => {
            if !check_channel_permission(state, user_id, channel_id, permissions::SEND_MESSAGES).await? {
                return Ok(());
            }
            let message = crate::db::messages::create(&state.db, channel_id, user_id, &content, reply_to_id)
                .await?;
            state.broadcast(ServerEvent::MessageCreate(message));
        }
        ClientEvent::EditMessage { message_id, content } => {
            let row = crate::db::messages::find_by_id(&state.db, message_id)
                .await?
                .ok_or("message not found")?;
            // Only the author can edit their own message
            if row.author_id != user_id {
                return Ok(());
            }
            crate::db::messages::update_content(&state.db, message_id, &content).await?;
            let mut updated = crate::db::messages::to_model(&crate::db::messages::find_by_id(&state.db, message_id)
                .await?
                .ok_or("message not found")?);
            let _ = crate::db::messages::enrich_with_attachments(&state.db, std::slice::from_mut(&mut updated)).await;
            state.broadcast(ServerEvent::MessageUpdate(updated));
        }
        ClientEvent::DeleteMessage { message_id } => {
            let row = crate::db::messages::find_by_id(&state.db, message_id)
                .await?
                .ok_or("message not found")?;
            // Author can delete own messages, or user with MANAGE_MESSAGES can delete any
            let perms = crate::db::roles::get_user_permissions(&state.db, user_id).await?;
            if row.author_id != user_id && !permissions::has(perms, permissions::MANAGE_MESSAGES) {
                return Ok(());
            }
            // Delete message (CASCADE deletes attachment rows)
            crate::db::messages::delete(&state.db, message_id).await?;
            // Clean up files from S3/MinIO
            let storage = state.storage.clone();
            tokio::spawn(async move {
                if let Err(e) = crate::storage::delete_prefix(&storage, &format!("{}/", message_id)).await {
                    tracing::error!("Failed to clean up files for message {}: {}", message_id, e);
                }
            });
            state.broadcast(ServerEvent::MessageDelete { id: message_id });
        }
        ClientEvent::JoinVoice { channel_id } => {
            if !check_channel_permission(state, user_id, channel_id, permissions::CONNECT).await? {
                return Ok(());
            }
            // Quitter l'ancien channel vocal si déjà dans un
            {
                let mut voice = state.voice_state.write().unwrap();
                for (cid, users) in voice.iter_mut() {
                    if users.remove(&user_id).is_some() {
                        state.broadcast(ServerEvent::UserLeftVoice {
                            user_id,
                            channel_id: *cid,
                        });
                    }
                }
                // Rejoindre le nouveau avec état par défaut
                voice.entry(channel_id).or_default().insert(user_id, VoiceUserState::default());
            }

            let user = crate::db::users::find_by_id(&state.db, user_id)
                .await?
                .ok_or("user not found")?;
            state.broadcast(ServerEvent::UserJoinedVoice {
                user,
                channel_id,
                voice_state: VoiceUserState::default(),
            });
        }
        ClientEvent::LeaveVoice { channel_id } => {
            {
                let mut voice = state.voice_state.write().unwrap();
                if let Some(users) = voice.get_mut(&channel_id) {
                    users.remove(&user_id);
                }
            }
            state.broadcast(ServerEvent::UserLeftVoice {
                user_id,
                channel_id,
            });
        }
        ClientEvent::UpdateVoiceState { muted, deafened } => {
            let mut channel_id = None;
            {
                let mut voice = state.voice_state.write().unwrap();
                for (cid, users) in voice.iter_mut() {
                    if let Some(vs) = users.get_mut(&user_id) {
                        vs.muted = muted || vs.force_muted;
                        vs.deafened = deafened || vs.force_deafened;
                        channel_id = Some((*cid, vs.clone()));
                        break;
                    }
                }
            }
            if let Some((cid, vs)) = channel_id {
                state.broadcast(ServerEvent::VoiceStateUpdate {
                    user_id,
                    channel_id: cid,
                    voice_state: vs,
                });
            }
        }
        ClientEvent::ForceMute { user_id: target_id, muted } => {
            // Vérifier la permission MUTE_MEMBERS
            let perms = crate::db::roles::get_user_permissions(&state.db, user_id).await?;
            if !permissions::has(perms, permissions::MUTE_MEMBERS) {
                return Ok(());
            }

            let mut result = None;
            {
                let mut voice = state.voice_state.write().unwrap();
                for (cid, users) in voice.iter_mut() {
                    if let Some(vs) = users.get_mut(&target_id) {
                        vs.force_muted = muted;
                        result = Some((*cid, vs.clone()));
                        break;
                    }
                }
            }
            if let Some((cid, vs)) = result {
                // Server-side mute via LiveKit API
                let room = format!("voice-{}", cid);
                let identity = format!("user-{}", target_id);
                let lk_url = state.livekit_internal_url.clone();
                let lk_key = state.livekit_api_key.clone();
                let lk_secret = state.livekit_api_secret.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::livekit::set_participant_muted(&lk_url, &lk_key, &lk_secret, &room, &identity, muted).await {
                        tracing::error!("LiveKit force mute failed: {}", e);
                    }
                });

                state.broadcast(ServerEvent::VoiceStateUpdate {
                    user_id: target_id,
                    channel_id: cid,
                    voice_state: vs,
                });
            }
        }
        ClientEvent::ForceDeafen { user_id: target_id, deafened } => {
            let perms = crate::db::roles::get_user_permissions(&state.db, user_id).await?;
            if !permissions::has(perms, permissions::DEAFEN_MEMBERS) {
                return Ok(());
            }

            let mut result = None;
            {
                let mut voice = state.voice_state.write().unwrap();
                for (cid, users) in voice.iter_mut() {
                    if let Some(vs) = users.get_mut(&target_id) {
                        vs.force_deafened = deafened;
                        if deafened {
                            vs.force_muted = true;
                        } else {
                            vs.force_muted = false;
                        }
                        result = Some((*cid, vs.clone()));
                        break;
                    }
                }
            }
            if let Some((cid, vs)) = result {
                // Server-side mute via LiveKit API (deafen = also mute audio)
                let room = format!("voice-{}", cid);
                let identity = format!("user-{}", target_id);
                let lk_url = state.livekit_internal_url.clone();
                let lk_key = state.livekit_api_key.clone();
                let lk_secret = state.livekit_api_secret.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::livekit::set_participant_muted(&lk_url, &lk_key, &lk_secret, &room, &identity, deafened).await {
                        tracing::error!("LiveKit force deafen failed: {}", e);
                    }
                });

                state.broadcast(ServerEvent::VoiceStateUpdate {
                    user_id: target_id,
                    channel_id: cid,
                    voice_state: vs,
                });
            }
        }
        ClientEvent::KickVoice { user_id: target_id } => {
            let perms = crate::db::roles::get_user_permissions(&state.db, user_id).await?;
            if !permissions::has(perms, permissions::MOVE_MEMBERS) {
                return Ok(());
            }

            let mut kicked_channel = None;
            {
                let mut voice = state.voice_state.write().unwrap();
                for (cid, users) in voice.iter_mut() {
                    if users.remove(&target_id).is_some() {
                        kicked_channel = Some(*cid);
                        break;
                    }
                }
            }
            if let Some(cid) = kicked_channel {
                state.broadcast(ServerEvent::UserLeftVoice {
                    user_id: target_id,
                    channel_id: cid,
                });

                // Remove from LiveKit room
                let room = format!("voice-{}", cid);
                let identity = format!("user-{}", target_id);
                let lk_url = state.livekit_internal_url.clone();
                let lk_key = state.livekit_api_key.clone();
                let lk_secret = state.livekit_api_secret.clone();
                tokio::spawn(async move {
                    if let Err(e) = crate::livekit::remove_participant(&lk_url, &lk_key, &lk_secret, &room, &identity).await {
                        tracing::error!("LiveKit kick failed: {}", e);
                    }
                });
            }
        }
    }

    Ok(())
}
