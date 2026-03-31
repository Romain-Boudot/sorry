use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::state::AppState;

#[derive(Serialize)]
pub struct MeResponse {
    user: shared::models::User,
    permissions: i64,
    users: Vec<shared::models::User>,
    online_users: Vec<i64>,
    voice_state: HashMap<i64, HashMap<i64, shared::models::VoiceUserState>>,
    roles: Vec<shared::models::Role>,
    user_roles: HashMap<i64, Vec<i64>>,
    max_file_size: usize,
}

/// GET /api/users/me
async fn me(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<MeResponse>, StatusCode> {
    let user = crate::db::users::find_by_id(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let permissions = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let users = crate::db::users::list_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let roles = crate::db::roles::list_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_roles = crate::db::roles::get_all_user_roles(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let online: Vec<i64> = state.online_users.read().unwrap().keys().copied().collect();

    let voice: HashMap<i64, HashMap<i64, shared::models::VoiceUserState>> = state
        .voice_state
        .read()
        .unwrap()
        .iter()
        .map(|(k, v)| (*k, v.iter().map(|(uid, vs)| (*uid, vs.clone())).collect()))
        .collect();

    Ok(Json(MeResponse {
        user,
        permissions,
        users,
        online_users: online,
        voice_state: voice,
        roles,
        user_roles,
        max_file_size: state.max_file_size,
    }))
}

#[derive(Deserialize)]
pub struct UpdateMePayload {
    display_name: String,
}

/// PATCH /api/users/me
async fn update_me(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<UpdateMePayload>,
) -> Result<Json<shared::models::User>, StatusCode> {
    let trimmed = payload.display_name.trim();
    if trimmed.is_empty() || trimmed.len() > 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    crate::db::users::update_display_name(&state.db, auth.0, trimmed)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = crate::db::users::find_by_id(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(user))
}

/// GET /api/users/:id/roles
async fn user_roles(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Vec<shared::models::Role>>, StatusCode> {
    let roles = crate::db::roles::get_user_roles(&state.db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(roles))
}

/// DELETE /api/users/:id — kick (delete) a user
async fn kick_user(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    // Can't kick yourself
    if auth.0 == user_id {
        return Err(StatusCode::BAD_REQUEST);
    }
    // Can't kick user id=1 (server owner)
    if user_id == 1 {
        return Err(StatusCode::FORBIDDEN);
    }

    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !shared::permissions::has(perms, shared::permissions::KICK_MEMBERS) {
        return Err(StatusCode::FORBIDDEN);
    }

    crate::db::users::find_by_id(&state.db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    crate::db::users::delete(&state.db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Notify all clients
    let _ = state.event_tx.send(shared::events::ServerEvent::UserOffline { user_id });

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/me", get(me).patch(update_me))
        .route("/:id", axum::routing::delete(kick_user))
        .route("/:id/roles", get(user_roles))
}
