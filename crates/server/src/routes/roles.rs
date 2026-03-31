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
use shared::events::ServerEvent;
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

/// Returns the highest (lowest position number) role position for a user.
/// Admin (position 0) is the highest.
async fn get_highest_position(
    db: &sqlx::SqlitePool,
    user_id: i64,
) -> Result<i64, StatusCode> {
    let roles = crate::db::roles::get_user_roles(db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(roles.iter().map(|r| r.position).min().unwrap_or(i64::MAX))
}

/// Check that the acting user can manage a role at the given position.
/// A user can only manage roles with a higher position number (= lower rank) than their own.
/// Admins bypass this check.
async fn require_role_hierarchy(
    db: &sqlx::SqlitePool,
    user_id: i64,
    target_position: i64,
) -> Result<(), StatusCode> {
    let perms = crate::db::roles::get_user_permissions(db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if permissions::has(perms, permissions::ADMINISTRATOR) {
        return Ok(());
    }
    let user_position = get_highest_position(db, user_id).await?;
    if user_position >= target_position {
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
    position: Option<i64>,
}

/// POST /api/roles
async fn create_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateRolePayload>,
) -> Result<Json<shared::models::Role>, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    let position = payload.position.unwrap_or(0);

    let id = crate::db::roles::create(
        &state.db,
        &payload.name,
        payload.permissions,
        payload.color.as_deref(),
        position,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let role = shared::models::Role {
        id,
        name: payload.name,
        permissions: payload.permissions,
        color: payload.color,
        position,
    };

    let _ = state.event_tx.send(ServerEvent::RoleCreate(role.clone()));

    Ok(Json(role))
}

/// PUT /api/roles/:id
async fn update_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<CreateRolePayload>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    let target = crate::db::roles::find_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    require_role_hierarchy(&state.db, auth.0, target.position).await?;

    // Admin role (id=1): allow name/color change but lock permissions
    let final_permissions = if id == 1 { target.permissions } else { payload.permissions };

    crate::db::roles::update(
        &state.db,
        id,
        &payload.name,
        final_permissions,
        payload.color.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = state.event_tx.send(ServerEvent::RoleUpdate(shared::models::Role {
        id,
        name: payload.name,
        permissions: final_permissions,
        color: payload.color,
        position: target.position,
    }));

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/roles/:id
async fn delete_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    let target = crate::db::roles::find_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Admin (id=1) and Membre (id=2) roles cannot be deleted
    if id <= 2 {
        return Err(StatusCode::FORBIDDEN);
    }

    require_role_hierarchy(&state.db, auth.0, target.position).await?;

    crate::db::roles::delete(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = state.event_tx.send(ServerEvent::RoleDelete { id });

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

    let target = crate::db::roles::find_by_id(&state.db, role_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    require_role_hierarchy(&state.db, auth.0, target.position).await?;

    crate::db::roles::assign_to_user(&state.db, payload.user_id, role_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let roles = crate::db::roles::get_user_roles(&state.db, payload.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _ = state.event_tx.send(ServerEvent::UserRoleUpdate {
        user_id: payload.user_id,
        role_ids: roles.iter().map(|r| r.id).collect(),
    });

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

    let target = crate::db::roles::find_by_id(&state.db, role_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    require_role_hierarchy(&state.db, auth.0, target.position).await?;

    crate::db::roles::remove_from_user(&state.db, payload.user_id, role_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let roles = crate::db::roles::get_user_roles(&state.db, payload.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let _ = state.event_tx.send(ServerEvent::UserRoleUpdate {
        user_id: payload.user_id,
        role_ids: roles.iter().map(|r| r.id).collect(),
    });

    Ok(StatusCode::OK)
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
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_ROLES).await?;

    crate::db::roles::reorder(&state.db, &payload.ids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Broadcast updated roles
    let roles = crate::db::roles::list_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    for role in roles {
        let _ = state.event_tx.send(ServerEvent::RoleUpdate(role));
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_roles).post(create_role))
        .route("/reorder", axum::routing::post(reorder_roles))
        .route("/:id", axum::routing::put(update_role).delete(delete_role))
        .route("/:id/assign", axum::routing::post(assign_role))
        .route("/:id/remove", axum::routing::post(remove_role))
}
