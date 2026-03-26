use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::state::AppState;

#[derive(Serialize)]
pub struct MeResponse {
    user: shared::models::User,
    users: Vec<shared::models::User>,
    online_users: Vec<i64>,
    voice_state: HashMap<i64, Vec<i64>>,
}

/// GET /api/users/me — info du user + état complet du serveur
async fn me(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<MeResponse>, StatusCode> {
    let user = crate::db::users::find_by_id(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

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
        users,
        online_users: online,
        voice_state: voice,
    }))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/me", get(me))
}
