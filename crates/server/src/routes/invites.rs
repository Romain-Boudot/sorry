use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

use super::auth::extract_client_ip;

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
) -> Result<Json<crate::db::invites::Invite>, AppError> {
    crate::perms::require_permission(&state.db, auth.0, shared::permissions::CREATE_INVITE).await?;

    // Validate role hierarchy
    if let Some(role_id) = payload.role_id {
        let perms = crate::db::roles::get_user_permissions(&state.db, auth.0).await?;
        if !shared::permissions::has(perms, shared::permissions::ADMINISTRATOR) {
            let target_role = crate::db::roles::find_by_id(&state.db, role_id)
                .await?
                .ok_or(AppError::BadRequest("Role not found".into()))?;
            crate::perms::require_role_hierarchy(&state.db, auth.0, target_role.position).await?;
        }
    }

    use rand::Rng;
    let code: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();

    let invite = crate::db::invites::create(
        &state.db, &code, auth.0, payload.max_uses, payload.expires_at, payload.role_id, payload.guest,
    ).await.map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(invite))
}

/// GET /api/invites — list all invites
async fn list_invites(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<crate::db::invites::Invite>>, AppError> {
    crate::perms::require_permission(&state.db, auth.0, shared::permissions::CREATE_INVITE).await?;
    let invites = crate::db::invites::list_all(&state.db)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(invites))
}

/// DELETE /api/invites/:code — revoke an invite
async fn delete_invite(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(code): Path<String>,
) -> Result<StatusCode, AppError> {
    crate::perms::require_permission(&state.db, auth.0, shared::permissions::CREATE_INVITE).await?;
    crate::db::invites::delete(&state.db, &code)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/invites/check/:code — public, check invite without consuming
async fn check_invite(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Path(code): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let client_ip = extract_client_ip(&headers);
    if !state.check_invite_rate_limit(client_ip) {
        return Err(AppError::TooManyRequests);
    }
    let invite = crate::db::invites::find_by_code(&state.db, &code)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::NotFound)?;

    if let Some(expires) = invite.expires_at {
        if chrono::Utc::now().timestamp() > expires {
            return Err(AppError::Gone);
        }
    }

    if let Some(max) = invite.max_uses {
        if invite.uses >= max {
            return Err(AppError::Gone);
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
