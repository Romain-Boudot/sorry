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

#[derive(Deserialize)]
pub struct WsQuery {
    token: String,
}

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<WsQuery>,
) -> Response {
    // Auth avant d'accepter la connexion
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

    // Marquer online
    {
        let mut online = state.online_users.write().unwrap();
        online.insert(user_id);
    }
    let _ = state.event_tx.send(ServerEvent::UserOnline { user_id });

    let mut rx = state.event_tx.subscribe();

    loop {
        tokio::select! {
            // Messages du client
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
            // Events broadcast → renvoyer au client
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

    // Marquer offline
    {
        let mut online = state.online_users.write().unwrap();
        online.remove(&user_id);
    }

    // Quitter tous les channels vocaux
    {
        let mut voice = state.voice_state.write().unwrap();
        for (channel_id, users) in voice.iter_mut() {
            if users.remove(&user_id) {
                let _ = state.event_tx.send(ServerEvent::UserLeftVoice {
                    user_id,
                    channel_id: *channel_id,
                });
            }
        }
    }

    let _ = state.event_tx.send(ServerEvent::UserOffline { user_id });
    tracing::info!("User {} disconnected", user_id);
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
        ClientEvent::JoinVoice { channel_id } => {
            // Quitter l'ancien channel vocal si déjà dans un
            {
                let mut voice = state.voice_state.write().unwrap();
                for (cid, users) in voice.iter_mut() {
                    if users.remove(&user_id) {
                        let _ = state.event_tx.send(ServerEvent::UserLeftVoice {
                            user_id,
                            channel_id: *cid,
                        });
                    }
                }
                // Rejoindre le nouveau
                voice.entry(channel_id).or_default().insert(user_id);
            }

            // Récupérer le user pour l'event
            let user = crate::db::users::find_by_id(&state.db, user_id)
                .await?
                .ok_or("user not found")?;
            let _ = state.event_tx.send(ServerEvent::UserJoinedVoice {
                user,
                channel_id,
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
    }

    Ok(())
}
