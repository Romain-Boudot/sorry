//! Unified permission checking — single source of truth for the whole server.

use shared::permissions;
use crate::error::AppError;

/// Check that a user has a given global permission (from their roles).
pub async fn require_permission(
    db: &sqlx::SqlitePool,
    user_id: i64,
    permission: i64,
) -> Result<(), AppError> {
    let perms = crate::db::roles::get_user_permissions(db, user_id).await?;
    if !permissions::has(perms, permission) {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Check that a user has a given permission in a specific channel,
/// taking into account channel overwrites. Admins bypass everything.
pub async fn require_channel_permission(
    db: &sqlx::SqlitePool,
    user_id: i64,
    channel_id: i64,
    permission: i64,
) -> Result<(), AppError> {
    let role_perms = crate::db::roles::get_user_permissions(db, user_id).await?;

    if permissions::has(role_perms, permissions::ADMINISTRATOR) {
        return Ok(());
    }

    let (allow, deny) = crate::db::roles::get_channel_overwrites(db, user_id, channel_id).await?;
    let final_perms = permissions::compute(&[role_perms], allow, deny);

    if !permissions::has(final_perms, permission) {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Same as `require_channel_permission` but returns bool instead of error.
/// Useful in the WS handler where we silently ignore unauthorized events.
pub async fn check_channel_permission(
    db: &sqlx::SqlitePool,
    user_id: i64,
    channel_id: i64,
    permission: i64,
) -> Result<bool, AppError> {
    let role_perms = crate::db::roles::get_user_permissions(db, user_id).await?;
    if permissions::has(role_perms, permissions::ADMINISTRATOR) {
        return Ok(true);
    }
    let (allow, deny) = crate::db::roles::get_channel_overwrites(db, user_id, channel_id).await?;
    let final_perms = permissions::compute(&[role_perms], allow, deny);
    Ok(permissions::has(final_perms, permission))
}

/// Check that the acting user's highest role outranks a given role position.
/// Owner (user ID 1) always bypasses.
pub async fn require_role_hierarchy(
    db: &sqlx::SqlitePool,
    user_id: i64,
    target_position: i64,
) -> Result<(), AppError> {
    if user_id == shared::OWNER_USER_ID {
        return Ok(());
    }
    let user_pos = get_highest_position(db, user_id).await?;
    if user_pos >= target_position {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Check that the acting user outranks a target user.
/// Owner always bypasses; owner can never be targeted.
pub async fn require_user_hierarchy(
    db: &sqlx::SqlitePool,
    actor_id: i64,
    target_user_id: i64,
) -> Result<(), AppError> {
    if actor_id == shared::OWNER_USER_ID {
        return Ok(());
    }
    if actor_id == target_user_id {
        return Err(AppError::Forbidden);
    }
    if target_user_id == shared::OWNER_USER_ID {
        return Err(AppError::Forbidden);
    }
    let actor_pos = get_highest_position(db, actor_id).await?;
    let target_pos = get_highest_position(db, target_user_id).await?;
    if actor_pos >= target_pos {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// Returns the highest (lowest position number) role position for a user.
async fn get_highest_position(db: &sqlx::SqlitePool, user_id: i64) -> Result<i64, AppError> {
    let roles = crate::db::roles::get_user_roles(db, user_id).await?;
    Ok(roles.iter().map(|r| r.position).min().unwrap_or(i64::MAX))
}
