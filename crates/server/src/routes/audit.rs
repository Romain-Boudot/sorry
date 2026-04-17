use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;
use shared::permissions;

#[derive(Deserialize)]
pub struct AuditQuery {
    limit: Option<i64>,
    before: Option<i64>,
}

/// GET /api/audit?limit=50&before=123 — ADMINISTRATOR only
async fn list_audit(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<shared::models::AuditLog>>, AppError> {
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0).await?;
    if !permissions::has(perms, permissions::ADMINISTRATOR) {
        return Err(AppError::Forbidden);
    }

    let logs = crate::db::audit::list(
        &state.db,
        query.limit.unwrap_or(50).min(200),
        query.before,
    )
    .await?;
    Ok(Json(logs))
}

/// GET /api/server/logs?limit=200 — ADMINISTRATOR only, in-memory server logs
async fn list_server_logs(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Vec<shared::models::ServerLogEntry>>, AppError> {
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0).await?;
    if !permissions::has(perms, permissions::ADMINISTRATOR) {
        return Err(AppError::Forbidden);
    }

    let limit = query.limit.unwrap_or(200).min(1000) as usize;
    let buf = state.server_logs.read().unwrap();
    let logs: Vec<shared::models::ServerLogEntry> = buf.iter().rev().take(limit).cloned().collect();
    Ok(Json(logs))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_audit))
        .route("/server", get(list_server_logs))
}
