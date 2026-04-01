use sqlx::SqlitePool;
use shared::models::User;

/// Ligne DB interne — contient le hash, ne jamais exposer via l'API
pub struct UserRow {
    pub id: Option<i64>,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub avatar_url: Option<String>,
    pub banned_at: Option<i64>,
}

pub fn to_model(row: &UserRow) -> User {
    User {
        id: row.id.unwrap_or(0),
        display_name: row.display_name.clone(),
        avatar_url: row.avatar_url.clone(),
    }
}

pub async fn find_by_username(db: &SqlitePool, username: &str) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as!(
        UserRow,
        r#"SELECT id, username, display_name, password_hash, avatar_url as "avatar_url?", banned_at FROM users WHERE username = ?"#,
        username
    )
    .fetch_optional(db)
    .await
}

pub struct UserPublicRow {
    pub id: Option<i64>,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

pub async fn find_by_id_internal(db: &SqlitePool, id: i64) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as!(
        UserRow,
        r#"SELECT id, username, display_name, password_hash, avatar_url as "avatar_url?", banned_at FROM users WHERE id = ?"#,
        id
    )
    .fetch_optional(db)
    .await
}

pub async fn find_by_id(db: &SqlitePool, id: i64) -> sqlx::Result<Option<User>> {
    let row: Option<UserPublicRow> = sqlx::query_as!(
        UserPublicRow,
        r#"SELECT id, display_name, avatar_url as "avatar_url?" FROM users WHERE id = ?"#,
        id
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| User {
        id: r.id.unwrap_or(0),
        display_name: r.display_name,
        avatar_url: r.avatar_url,
    }))
}

pub async fn list_all(db: &SqlitePool) -> sqlx::Result<Vec<User>> {
    let rows: Vec<UserPublicRow> = sqlx::query_as!(
        UserPublicRow,
        r#"SELECT id, display_name, avatar_url as "avatar_url?" FROM users"#
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .iter()
        .map(|r| User {
            id: r.id.unwrap_or(0),
            display_name: r.display_name.clone(),
            avatar_url: r.avatar_url.clone(),
        })
        .collect())
}

pub async fn update_display_name(db: &SqlitePool, id: i64, display_name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE users SET display_name = ? WHERE id = ?",
        display_name,
        id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_avatar_url(db: &SqlitePool, id: i64, avatar_url: Option<&str>) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE users SET avatar_url = ? WHERE id = ?",
        avatar_url,
        id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_password(db: &SqlitePool, id: i64, password_hash: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE users SET password_hash = ? WHERE id = ?",
        password_hash,
        id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn update_username(db: &SqlitePool, id: i64, username: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE users SET username = ? WHERE id = ?",
        username,
        id
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Load all user IDs banned within the last `ttl_secs` seconds
pub async fn load_recent_bans(db: &SqlitePool, ttl_secs: i64) -> sqlx::Result<Vec<i64>> {
    let cutoff = chrono::Utc::now().timestamp() - ttl_secs;
    let rows = sqlx::query!(
        "SELECT id FROM users WHERE banned_at IS NOT NULL AND banned_at > ?",
        cutoff
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|r| r.id).collect())
}

pub async fn ban(db: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE users SET banned_at = unixepoch() WHERE id = ?",
        id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn unban(db: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE users SET banned_at = NULL WHERE id = ?",
        id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete(db: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM users WHERE id = ?", id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn create(
    db: &SqlitePool,
    username: &str,
    display_name: &str,
    password_hash: &str,
) -> sqlx::Result<i64> {
    let row = sqlx::query!(
        "INSERT INTO users (username, display_name, password_hash) VALUES (?, ?, ?) RETURNING id",
        username,
        display_name,
        password_hash
    )
    .fetch_one(db)
    .await?;

    Ok(row.id)
}
