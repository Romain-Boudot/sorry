use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::perms::{require_permission, require_role_hierarchy, require_user_hierarchy};
use crate::state::AppState;
use shared::events::ServerEvent;
use shared::permissions;

// --- Handlers ---

/// GET /api/roles
async fn list_roles(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
) -> Result<Json<Vec<shared::models::Role>>, AppError> {
    let roles = crate::db::roles::list_all(&state.db).await?;
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
) -> Result<Json<shared::models::Role>, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    // Auto-assign position: after all existing custom roles
    let all_roles = crate::db::roles::list_all(&state.db).await?;
    let max_custom_pos = all_roles.iter()
        .filter(|r| r.id >= shared::MIN_CUSTOM_ROLE_ID)
        .map(|r| r.position)
        .max()
        .unwrap_or(-1);
    let position = max_custom_pos + 1;

    let id = crate::db::roles::create(
        &state.db,
        &payload.name,
        payload.permissions,
        payload.color.as_deref(),
        position,
    )
    .await?;

    let role = shared::models::Role {
        id,
        name: payload.name,
        permissions: payload.permissions,
        color: payload.color,
        position,
    };

    state.broadcast(ServerEvent::RoleCreate(role.clone()));

    Ok(Json(role))
}

/// PUT /api/roles/:id
async fn update_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<CreateRolePayload>,
) -> Result<axum::http::StatusCode, AppError> {
    // Owner role: completely immutable
    if id == shared::OWNER_ROLE_ID {
        return Err(AppError::Forbidden);
    }

    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    let target = crate::db::roles::find_by_id(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;

    require_role_hierarchy(&state.db, auth.0, target.position).await?;

    // Everyone role: only permissions can change, name/color stay fixed
    let final_name = if id == shared::EVERYONE_ROLE_ID { target.name.clone() } else { payload.name.clone() };
    let final_color = if id == shared::EVERYONE_ROLE_ID { target.color.clone() } else { payload.color.clone() };

    crate::db::roles::update(
        &state.db,
        id,
        &final_name,
        payload.permissions,
        final_color.as_deref(),
    )
    .await?;

    state.broadcast(ServerEvent::RoleUpdate(shared::models::Role {
        id,
        name: final_name,
        permissions: payload.permissions,
        color: final_color,
        position: target.position,
    }));

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// DELETE /api/roles/:id
async fn delete_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<axum::http::StatusCode, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    let target = crate::db::roles::find_by_id(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;

    if shared::is_system_role(id) {
        return Err(AppError::Forbidden);
    }

    require_role_hierarchy(&state.db, auth.0, target.position).await?;

    crate::db::roles::delete(&state.db, id).await?;

    state.broadcast(ServerEvent::RoleDelete { id });

    Ok(axum::http::StatusCode::NO_CONTENT)
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
) -> Result<axum::http::StatusCode, AppError> {
    if shared::is_system_role(role_id) {
        return Err(AppError::Forbidden);
    }

    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;
    require_user_hierarchy(&state.db, auth.0, payload.user_id).await?;

    let target = crate::db::roles::find_by_id(&state.db, role_id)
        .await?
        .ok_or(AppError::NotFound)?;

    require_role_hierarchy(&state.db, auth.0, target.position).await?;

    crate::db::roles::assign_to_user(&state.db, payload.user_id, role_id).await?;

    let roles = crate::db::roles::get_user_roles(&state.db, payload.user_id).await?;
    let user_perms = crate::db::roles::get_user_permissions(&state.db, payload.user_id).await?;
    state.broadcast(ServerEvent::UserRoleUpdate {
        user_id: payload.user_id,
        role_ids: roles.iter().map(|r| r.id).collect(),
        permissions: user_perms,
    });

    Ok(axum::http::StatusCode::OK)
}

/// POST /api/roles/:id/remove
async fn remove_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(role_id): Path<i64>,
    Json(payload): Json<AssignRolePayload>,
) -> Result<axum::http::StatusCode, AppError> {
    if shared::is_system_role(role_id) {
        return Err(AppError::Forbidden);
    }

    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;
    require_user_hierarchy(&state.db, auth.0, payload.user_id).await?;

    let target = crate::db::roles::find_by_id(&state.db, role_id)
        .await?
        .ok_or(AppError::NotFound)?;

    require_role_hierarchy(&state.db, auth.0, target.position).await?;

    crate::db::roles::remove_from_user(&state.db, payload.user_id, role_id).await?;

    let roles = crate::db::roles::get_user_roles(&state.db, payload.user_id).await?;
    let user_perms = crate::db::roles::get_user_permissions(&state.db, payload.user_id).await?;
    state.broadcast(ServerEvent::UserRoleUpdate {
        user_id: payload.user_id,
        role_ids: roles.iter().map(|r| r.id).collect(),
        permissions: user_perms,
    });

    Ok(axum::http::StatusCode::OK)
}

#[derive(Deserialize)]
pub struct ReorderPayload {
    ids: Vec<i64>,
}

/// POST /api/roles/reorder
async fn reorder_roles(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<ReorderPayload>,
) -> Result<axum::http::StatusCode, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    // Filter out system roles — their positions are fixed
    let custom_ids: Vec<i64> = payload.ids.iter()
        .filter(|&&id| !shared::is_system_role(id))
        .copied()
        .collect();

    crate::db::roles::reorder(&state.db, &custom_ids).await?;

    // Broadcast updated roles
    let roles = crate::db::roles::list_all(&state.db).await?;
    for role in roles {
        state.broadcast(ServerEvent::RoleUpdate(role));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_roles).post(create_role))
        .route("/reorder", axum::routing::post(reorder_roles))
        .route("/:id", axum::routing::put(update_role).delete(delete_role))
        .route("/:id/assign", axum::routing::post(assign_role))
        .route("/:id/remove", axum::routing::post(remove_role))
}
