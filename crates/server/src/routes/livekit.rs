use axum::{
    extract::State,
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::state::AppState;
use shared::permissions;

#[derive(Deserialize)]
pub struct JoinVoicePayload {
    channel_id: i64,
}

#[derive(Serialize)]
pub struct JoinVoiceResponse {
    token: String,
    url: String,
}

/// POST /api/livekit/token — génère un token pour rejoindre un channel vocal
async fn get_token(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<JoinVoicePayload>,
) -> Result<Json<JoinVoiceResponse>, StatusCode> {
    // Vérifier permission CONNECT
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !permissions::has(perms, permissions::CONNECT) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Vérifier que le channel existe et est vocal
    let channel = crate::db::channels::find_by_id(&state.db, payload.channel_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if channel.kind != "voice" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Check user limit (ADMINISTRATOR and MOVE_MEMBERS bypass)
    let bypass = permissions::has(perms, permissions::ADMINISTRATOR)
        || permissions::has(perms, permissions::MOVE_MEMBERS);
    if !bypass {
        if let Some(limit) = channel.user_limit {
            let voice = state.voice_state.read().unwrap();
            let count = voice.get(&payload.channel_id).map(|u| u.len()).unwrap_or(0) as i64;
            if count >= limit {
                return Err(StatusCode::FORBIDDEN);
            }
        }
    }

    // Récupérer le user pour le display name
    let user = crate::db::users::find_by_id(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let room_name = state.room_name(payload.channel_id);
    let identity = format!("user-{}", auth.0);

    let can_speak = permissions::has(perms, permissions::SPEAK);
    let can_stream = permissions::has(perms, permissions::STREAM);

    let token = crate::livekit::generate_token(
        &state.livekit_api_key,
        &state.livekit_api_secret,
        &room_name,
        &identity,
        &user.display_name,
        can_speak,
        can_stream,
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(JoinVoiceResponse {
        token,
        url: state.livekit_url.clone(),
    }))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/token", post(get_token))
}
