use sqlx::SqlitePool;
use shared::models::Role;

pub struct RoleRow {
    pub id: Option<i64>,
    pub name: String,
    pub permissions: i64,
    pub color: Option<String>,
    pub position: i64,
}

pub fn to_model(row: &RoleRow) -> Role {
    Role {
        id: row.id.unwrap_or(0),
        name: row.name.clone(),
        permissions: row.permissions,
        color: row.color.clone(),
        position: row.position,
    }
}

pub async fn create(
    db: &SqlitePool,
    name: &str,
    permissions: i64,
    color: Option<&str>,
    position: i64,
) -> sqlx::Result<i64> {
    let row = sqlx::query!(
        "INSERT INTO roles (name, permissions, color, position) VALUES (?, ?, ?, ?) RETURNING id",
        name,
        permissions,
        color,
        position
    )
    .fetch_one(db)
    .await?;
    Ok(row.id)
}

pub async fn list_all(db: &SqlitePool) -> sqlx::Result<Vec<Role>> {
    let rows = sqlx::query_as!(
        RoleRow,
        "SELECT id, name, permissions, color, position FROM roles ORDER BY position"
    )
    .fetch_all(db)
    .await?;
    Ok(rows.iter().map(to_model).collect())
}

pub async fn find_by_id(db: &SqlitePool, id: i64) -> sqlx::Result<Option<RoleRow>> {
    sqlx::query_as!(
        RoleRow,
        "SELECT id, name, permissions, color, position FROM roles WHERE id = ?",
        id
    )
    .fetch_optional(db)
    .await
}

pub async fn update(
    db: &SqlitePool,
    id: i64,
    name: &str,
    permissions: i64,
    color: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE roles SET name = ?, permissions = ?, color = ? WHERE id = ?",
        name,
        permissions,
        color,
        id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete(db: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM roles WHERE id = ?", id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn reorder(db: &SqlitePool, ids: &[i64]) -> sqlx::Result<()> {
    // Custom roles start at position 1 (position 0 is reserved for Owner role)
    for (i, id) in ids.iter().enumerate() {
        let pos = (i + 1) as i64;
        sqlx::query!("UPDATE roles SET position = ? WHERE id = ?", pos, id)
            .execute(&*db)
            .await?;
    }
    Ok(())
}

pub async fn assign_to_user(db: &SqlitePool, user_id: i64, role_id: i64) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT OR IGNORE INTO user_roles (user_id, role_id) VALUES (?, ?)",
        user_id,
        role_id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn remove_from_user(db: &SqlitePool, user_id: i64, role_id: i64) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM user_roles WHERE user_id = ? AND role_id = ?",
        user_id,
        role_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Récupère le mapping user_id → role_ids pour tous les users
pub async fn get_all_user_roles(db: &SqlitePool) -> sqlx::Result<std::collections::HashMap<i64, Vec<i64>>> {
    let rows = sqlx::query!("SELECT user_id, role_id FROM user_roles")
        .fetch_all(db)
        .await?;

    let mut map: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    for r in rows {
        map.entry(r.user_id).or_default().push(r.role_id);
    }
    Ok(map)
}

/// Récupère les rôles d'un user
pub async fn get_user_roles(db: &SqlitePool, user_id: i64) -> sqlx::Result<Vec<Role>> {
    let rows = sqlx::query_as!(
        RoleRow,
        "SELECT r.id, r.name, r.permissions, r.color, r.position
         FROM roles r
         INNER JOIN user_roles ur ON ur.role_id = r.id
         WHERE ur.user_id = ?
         ORDER BY r.position",
        user_id
    )
    .fetch_all(db)
    .await?;
    Ok(rows.iter().map(to_model).collect())
}

/// Récupère les permissions combinées (OR) de tous les rôles d'un user.
/// Le rôle everyone (ID=2) est implicite pour tous les users non-guest.
/// Le rôle Owner (ID=1) est implicite pour user ID 1.
pub async fn get_user_permissions(db: &SqlitePool, user_id: i64) -> sqlx::Result<i64> {
    // Custom roles from user_roles table
    let rows = sqlx::query!(
        "SELECT r.permissions FROM roles r
         INNER JOIN user_roles ur ON ur.role_id = r.id
         WHERE ur.user_id = ?",
        user_id
    )
    .fetch_all(db)
    .await?;

    let mut perms: i64 = 0;
    for r in rows {
        perms |= r.permissions;
    }

    // Implicit everyone role — skip for guest users
    let is_guest = crate::db::users::is_guest(db, user_id).await?;
    if !is_guest {
        if let Some(everyone) = find_by_id(db, shared::EVERYONE_ROLE_ID).await? {
            perms |= everyone.permissions;
        }
    }

    // Implicit Owner role for the owner user
    if user_id == shared::OWNER_USER_ID {
        if let Some(owner) = find_by_id(db, shared::OWNER_ROLE_ID).await? {
            perms |= owner.permissions;
        }
    }

    Ok(perms)
}

/// List all overwrites for a channel
pub async fn list_all_channel_overwrites(
    db: &SqlitePool,
) -> sqlx::Result<Vec<shared::models::ChannelOverwrite>> {
    let rows = sqlx::query!(
        "SELECT channel_id, role_id, allow, deny FROM channel_permission_overwrites"
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .iter()
        .map(|r| shared::models::ChannelOverwrite {
            channel_id: r.channel_id,
            role_id: r.role_id,
            allow: r.allow,
            deny: r.deny,
        })
        .collect())
}

pub async fn list_channel_overwrites(
    db: &SqlitePool,
    channel_id: i64,
) -> sqlx::Result<Vec<shared::models::ChannelOverwrite>> {
    let rows = sqlx::query!(
        "SELECT channel_id, role_id, allow, deny FROM channel_permission_overwrites WHERE channel_id = ?",
        channel_id
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .iter()
        .map(|r| shared::models::ChannelOverwrite {
            channel_id: r.channel_id,
            role_id: r.role_id,
            allow: r.allow,
            deny: r.deny,
        })
        .collect())
}

/// Set (upsert) a channel overwrite for a role
pub async fn set_channel_overwrite(
    db: &SqlitePool,
    channel_id: i64,
    role_id: i64,
    allow: i64,
    deny: i64,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO channel_permission_overwrites (channel_id, role_id, allow, deny) VALUES (?, ?, ?, ?)
         ON CONFLICT(channel_id, role_id) DO UPDATE SET allow = excluded.allow, deny = excluded.deny",
        channel_id,
        role_id,
        allow,
        deny
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Delete a channel overwrite for a role
pub async fn delete_channel_overwrite(
    db: &SqlitePool,
    channel_id: i64,
    role_id: i64,
) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM channel_permission_overwrites WHERE channel_id = ? AND role_id = ?",
        channel_id,
        role_id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Merge overwrites for a single implicit role into the allow/deny accumulators.
async fn merge_implicit_overwrite(
    db: &SqlitePool,
    role_id: i64,
    channel_id: i64,
    allow: &mut i64,
    deny: &mut i64,
) -> sqlx::Result<()> {
    if let Some(r) = sqlx::query!(
        "SELECT allow, deny FROM channel_permission_overwrites WHERE role_id = ? AND channel_id = ?",
        role_id,
        channel_id
    )
    .fetch_optional(db)
    .await? {
        *allow |= r.allow;
        *deny |= r.deny;
    }
    Ok(())
}

/// Récupère les channel overwrites pour les rôles d'un user.
/// Includes implicit Everyone role for non-guests and Owner role for user 1.
pub async fn get_channel_overwrites(
    db: &SqlitePool,
    user_id: i64,
    channel_id: i64,
) -> sqlx::Result<(i64, i64)> {
    // Overwrites from explicit custom roles
    let rows = sqlx::query!(
        "SELECT cpo.allow, cpo.deny
         FROM channel_permission_overwrites cpo
         INNER JOIN user_roles ur ON ur.role_id = cpo.role_id
         WHERE ur.user_id = ? AND cpo.channel_id = ?",
        user_id,
        channel_id
    )
    .fetch_all(db)
    .await?;

    let mut allow: i64 = 0;
    let mut deny: i64 = 0;
    for r in rows {
        allow |= r.allow;
        deny |= r.deny;
    }

    // Implicit everyone role overwrites — skip for guest users
    let is_guest = crate::db::users::is_guest(db, user_id).await?;
    if !is_guest {
        merge_implicit_overwrite(db, shared::EVERYONE_ROLE_ID, channel_id, &mut allow, &mut deny).await?;
    }

    // Implicit Owner role overwrites for the owner user
    if user_id == shared::OWNER_USER_ID {
        merge_implicit_overwrite(db, shared::OWNER_ROLE_ID, channel_id, &mut allow, &mut deny).await?;
    }

    Ok((allow, deny))
}
