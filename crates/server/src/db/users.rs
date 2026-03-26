use sqlx::SqlitePool;
use shared::models::User;

/// Ligne DB interne — contient le hash, ne jamais exposer via l'API
pub struct UserRow {
    pub id: Option<i64>,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
}

pub fn to_model(row: &UserRow) -> User {
    User {
        id: row.id.unwrap_or(0),
        username: row.username.clone(),
        display_name: row.display_name.clone(),
    }
}

pub async fn find_by_username(db: &SqlitePool, username: &str) -> sqlx::Result<Option<UserRow>> {
    sqlx::query_as!(
        UserRow,
        "SELECT id, username, display_name, password_hash FROM users WHERE username = ?",
        username
    )
    .fetch_optional(db)
    .await
}

pub struct UserPublicRow {
    pub id: Option<i64>,
    pub username: String,
    pub display_name: String,
}

pub async fn find_by_id(db: &SqlitePool, id: i64) -> sqlx::Result<Option<User>> {
    let row = sqlx::query_as!(
        UserPublicRow,
        "SELECT id, username, display_name FROM users WHERE id = ?",
        id
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| User {
        id: r.id.unwrap_or(0),
        username: r.username,
        display_name: r.display_name,
    }))
}

pub async fn list_all(db: &SqlitePool) -> sqlx::Result<Vec<User>> {
    let rows = sqlx::query_as!(
        UserPublicRow,
        "SELECT id, username, display_name FROM users"
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .iter()
        .map(|r| User {
            id: r.id.unwrap_or(0),
            username: r.username.clone(),
            display_name: r.display_name.clone(),
        })
        .collect())
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
