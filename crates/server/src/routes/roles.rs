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
use shared::permissions;

/// Helper pour vérifier qu'un user a une permission
async fn require_permission(
    db: &sqlx::SqlitePool,
    user_id: i64,
    permission: i64,
) -> Result<(), StatusCode> {
    let perms = crate::db::roles::get_user_permissions(db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !permissions::has(perms, permission) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

// --- Handlers ---

/// GET /api/roles
async fn list_roles(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
) -> Result<Json<Vec<shared::models::Role>>, StatusCode> {
    let roles = crate::db::roles::list_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(roles))
}

#[derive(Deserialize)]
pub struct CreateRolePayload {
    name: String,
    permissions: i64,
    color: Option<String>,
}

/// POST /api/roles
async fn create_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateRolePayload>,
) -> Result<Json<shared::models::Role>, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    let id = crate::db::roles::create(
        &state.db,
        &payload.name,
        payload.permissions,
        payload.color.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(shared::models::Role {
        id,
        name: payload.name,
        permissions: payload.permissions,
        color: payload.color,
        position: 0,
    }))
}

/// PUT /api/roles/:id
async fn update_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<CreateRolePayload>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    crate::db::roles::find_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    crate::db::roles::update(
        &state.db,
        id,
        &payload.name,
        payload.permissions,
        payload.color.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/roles/:id
async fn delete_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    crate::db::roles::delete(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct AssignRolePayload {
    user_id: i64,
}

/// POST /api/roles/:id/assign
async fn assign_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(role_id): Path<i64>,
    Json(payload): Json<AssignRolePayload>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    crate::db::roles::assign_to_user(&state.db, payload.user_id, role_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// POST /api/roles/:id/remove
async fn remove_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(role_id): Path<i64>,
    Json(payload): Json<AssignRolePayload>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    crate::db::roles::remove_from_user(&state.db, payload.user_id, role_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_roles).post(create_role))
        .route("/:id", axum::routing::put(update_role).delete(delete_role))
        .route("/:id/assign", axum::routing::post(assign_role))
        .route("/:id/remove", axum::routing::post(remove_role))
}
