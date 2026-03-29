use axum::{
    extract::{Query, State, WebSocketUpgrade},
    extract::ws::{Message, WebSocket},
    response::Response,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth;
use crate::state::AppState;
use shared::events::{ClientEvent, ServerEvent};
use shared::models::VoiceUserState;
use shared::permissions;

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

    // Marquer online (incrémenter le compteur de connexions)
    let is_first_connection = {
        let mut online = state.online_users.write().unwrap();
        let count = online.entry(user_id).or_insert(0);
        *count += 1;
        *count == 1
    };
    if is_first_connection {
        if let Ok(Some(user)) = crate::db::users::find_by_id(&state.db, user_id).await {
            let _ = state.event_tx.send(ServerEvent::UserOnline { user });
        }
    }

    let mut rx = state.event_tx.subscribe();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Err(e) = handle_client_event(&state, user_id, &text).await {
                            tracing::warn!("Error handling client event: {}", e);
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            event = rx.recv() => {
                if let Ok(event) = event {
                    if let Ok(json) = serde_json::to_string(&event) {
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Décrémenter le compteur de connexions
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

    // Seulement si c'était la dernière connexion
    if is_last_connection {
        // Quitter tous les channels vocaux
        {
            let mut voice = state.voice_state.write().unwrap();
            for (channel_id, users) in voice.iter_mut() {
                if users.remove(&user_id).is_some() {
                    let _ = state.event_tx.send(ServerEvent::UserLeftVoice {
                        user_id,
                        channel_id: *channel_id,
                    });
                }
            }
        }

        let _ = state.event_tx.send(ServerEvent::UserOffline { user_id });
    }

    tracing::info!("User {} disconnected (last={})", user_id, is_last_connection);
}

async fn handle_client_event(
    state: &AppState,
    user_id: i64,
    text: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let event: ClientEvent = serde_json::from_str(text)?;

    match event {
        ClientEvent::SendMessage { channel_id, content } => {
            let message = crate::db::messages::create(&state.db, channel_id, user_id, &content)
                .await?;
            let _ = state.event_tx.send(ServerEvent::MessageCreate(message));
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
            let _ = state.event_tx.send(ServerEvent::MessageUpdate(updated));
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
            // Clean up files from disk
            let upload_dir = state.upload_dir.clone();
            tokio::spawn(async move {
                let dir = std::path::PathBuf::from(&upload_dir).join(message_id.to_string());
                let _ = tokio::fs::remove_dir_all(&dir).await;
            });
            let _ = state.event_tx.send(ServerEvent::MessageDelete { id: message_id });
        }
        ClientEvent::JoinVoice { channel_id } => {
            // Quitter l'ancien channel vocal si déjà dans un
            {
                let mut voice = state.voice_state.write().unwrap();
                for (cid, users) in voice.iter_mut() {
                    if users.remove(&user_id).is_some() {
                        let _ = state.event_tx.send(ServerEvent::UserLeftVoice {
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
            let _ = state.event_tx.send(ServerEvent::UserJoinedVoice {
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
            let _ = state.event_tx.send(ServerEvent::UserLeftVoice {
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
                let _ = state.event_tx.send(ServerEvent::VoiceStateUpdate {
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
                        if muted { vs.muted = true; }
                        result = Some((*cid, vs.clone()));
                        break;
                    }
                }
            }
            if let Some((cid, vs)) = result {
                let _ = state.event_tx.send(ServerEvent::VoiceStateUpdate {
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
                            vs.deafened = true;
                            vs.force_muted = true;
                            vs.muted = true;
                        }
                        result = Some((*cid, vs.clone()));
                        break;
                    }
                }
            }
            if let Some((cid, vs)) = result {
                let _ = state.event_tx.send(ServerEvent::VoiceStateUpdate {
                    user_id: target_id,
                    channel_id: cid,
                    voice_state: vs,
                });
            }
        }
    }

    Ok(())
}
