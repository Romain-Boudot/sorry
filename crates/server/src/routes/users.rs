use axum::{
    extract::State,
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
    voice_state: HashMap<i64, Vec<i64>>,
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

    let online: Vec<i64> = state.online_users.read().unwrap().iter().copied().collect();

    let voice: HashMap<i64, Vec<i64>> = state
        .voice_state
        .read()
        .unwrap()
        .iter()
        .map(|(k, v)| (*k, v.iter().copied().collect()))
        .collect();

    Ok(Json(MeResponse {
        user,
        permissions,
        users,
        online_users: online,
        voice_state: voice,
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

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/me", get(me).patch(update_me))
}
