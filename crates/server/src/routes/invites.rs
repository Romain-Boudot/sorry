use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateInvitePayload {
    max_uses: Option<i64>,
    expires_at: Option<i64>,
}

/// POST /api/invites — create an invite
async fn create_invite(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateInvitePayload>,
) -> Result<Json<crate::db::invites::Invite>, StatusCode> {
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !shared::permissions::has(perms, shared::permissions::CREATE_INVITE) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Generate a short random code
    use rand::Rng;
    let code: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();

    let invite = crate::db::invites::create(
        &state.db,
        &code,
        auth.0,
        payload.max_uses,
        payload.expires_at,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(invite))
}

/// GET /api/invites — list all invites
async fn list_invites(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<crate::db::invites::Invite>>, StatusCode> {
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !shared::permissions::has(perms, shared::permissions::CREATE_INVITE) {
        return Err(StatusCode::FORBIDDEN);
    }

    let invites = crate::db::invites::list_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(invites))
}

/// DELETE /api/invites/:code — revoke an invite
async fn delete_invite(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(code): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !shared::permissions::has(perms, shared::permissions::CREATE_INVITE) {
        return Err(StatusCode::FORBIDDEN);
    }

    crate::db::invites::delete(&state.db, &code)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_invites).post(create_invite))
        .route("/:code", axum::routing::delete(delete_invite))
}
