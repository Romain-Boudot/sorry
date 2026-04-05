use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::state::AppState;

fn extract_client_ip(headers: &HeaderMap) -> std::net::IpAddr {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or_else(|| std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST))
}

#[derive(Deserialize)]
pub struct CreateInvitePayload {
    max_uses: Option<i64>,
    expires_at: Option<i64>,
    role_id: Option<i64>,
    #[serde(default)]
    guest: bool,
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

    // Validate role hierarchy: can only assign roles below your highest role
    if let Some(role_id) = payload.role_id {
        // Admins bypass hierarchy check
        if !shared::permissions::has(perms, shared::permissions::ADMINISTRATOR) {
            let user_roles = crate::db::roles::get_user_roles(&state.db, auth.0)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let user_highest = user_roles.iter().map(|r| r.position).min().unwrap_or(i64::MAX);
            let target_role = crate::db::roles::find_by_id(&state.db, role_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::BAD_REQUEST)?;
            // Lower position = higher rank. Can only assign roles ranked below yours
            if target_role.position <= user_highest {
                return Err(StatusCode::FORBIDDEN);
            }
        }
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
        payload.role_id,
        payload.guest,
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

/// GET /api/invites/check/:code — public, check invite without consuming
async fn check_invite(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client_ip = extract_client_ip(&headers);
    if !state.check_invite_rate_limit(client_ip) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }
    let invite = crate::db::invites::find_by_code(&state.db, &code)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Check expiration
    if let Some(expires) = invite.expires_at {
        if chrono::Utc::now().timestamp() > expires {
            return Err(StatusCode::GONE);
        }
    }

    // Check max uses
    if let Some(max) = invite.max_uses {
        if invite.uses >= max {
            return Err(StatusCode::GONE);
        }
    }

    Ok(Json(serde_json::json!({
        "valid": true,
        "guest": invite.guest,
    })))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_invites).post(create_invite))
        .route("/check/:code", axum::routing::get(check_invite))
        .route("/:code", axum::routing::delete(delete_invite))
}
