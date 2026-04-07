mod messages;
mod voice;

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

    let mut rx = state.event_tx.subscribe();
    let mut last_snapshot_request = Instant::now();

    if let Err(e) = send_snapshot(&state, &mut socket, user_id).await {
        tracing::warn!("Failed to send snapshot to user {}: {}", user_id, e);
        return;
    }

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
        voice::remove_from_all_channels(state, user_id);
        state.broadcast(ServerEvent::UserOffline { user_id });
    }

    tracing::info!("User {} disconnected (last={})", user_id, is_last_connection);
}

async fn send_snapshot(
    state: &AppState,
    socket: &mut WebSocket,
    user_id: i64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
            if last_snapshot_request.elapsed() < std::time::Duration::from_secs(5) {
                tracing::warn!("User {} snapshot request rate-limited", user_id);
                return Ok(());
            }
            *last_snapshot_request = Instant::now();
            send_snapshot(state, socket, user_id).await?;
        }

        // ── Messages ──
        ClientEvent::SendMessage { channel_id, content, reply_to_id } => {
            messages::handle_send(state, user_id, channel_id, content, reply_to_id).await?;
        }
        ClientEvent::EditMessage { message_id, content } => {
            messages::handle_edit(state, user_id, message_id, content).await?;
        }
        ClientEvent::DeleteMessage { message_id } => {
            messages::handle_delete(state, user_id, message_id).await?;
        }

        // ── Voice ──
        ClientEvent::JoinVoice { channel_id } => {
            voice::handle_join(state, user_id, channel_id).await?;
        }
        ClientEvent::LeaveVoice { channel_id } => {
            voice::handle_leave(state, user_id, channel_id).await?;
        }
        ClientEvent::UpdateVoiceState { muted, deafened, screen_sharing, camera_on } => {
            voice::handle_update_state(state, user_id, muted, deafened, screen_sharing, camera_on).await?;
        }
        ClientEvent::ForceMute { user_id: target_id, muted } => {
            voice::handle_force_mute(state, user_id, target_id, muted).await?;
        }
        ClientEvent::ForceDeafen { user_id: target_id, deafened } => {
            voice::handle_force_deafen(state, user_id, target_id, deafened).await?;
        }
        ClientEvent::KickVoice { user_id: target_id } => {
            voice::handle_kick(state, user_id, target_id).await?;
        }
    }

    Ok(())
}
