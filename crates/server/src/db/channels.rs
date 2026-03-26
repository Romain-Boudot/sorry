use sqlx::SqlitePool;
use shared::models::{Channel, ChannelKind};

pub struct ChannelRow {
    pub id: Option<i64>,
    pub name: String,
    pub kind: String,
    pub position: i64,
}

pub fn to_model(row: &ChannelRow) -> Channel {
    Channel {
        id: row.id.unwrap_or(0),
        name: row.name.clone(),
        kind: match row.kind.as_str() {
            "voice" => ChannelKind::Voice,
            _ => ChannelKind::Text,
        },
        position: row.position,
    }
}

pub async fn create(db: &SqlitePool, name: &str, kind: &str) -> sqlx::Result<i64> {
    let row = sqlx::query!(
        "INSERT INTO channels (name, kind) VALUES (?, ?) RETURNING id",
        name,
        kind
    )
    .fetch_one(db)
    .await?;
    Ok(row.id)
}

pub async fn list_all(db: &SqlitePool) -> sqlx::Result<Vec<Channel>> {
    let rows = sqlx::query_as!(
        ChannelRow,
        "SELECT id, name, kind, position FROM channels ORDER BY position"
    )
    .fetch_all(db)
    .await?;

    Ok(rows.iter().map(to_model).collect())
}

pub async fn find_by_id(db: &SqlitePool, id: i64) -> sqlx::Result<Option<ChannelRow>> {
    sqlx::query_as!(
        ChannelRow,
        "SELECT id, name, kind, position FROM channels WHERE id = ?",
        id
    )
    .fetch_optional(db)
    .await
}

pub async fn delete(db: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM channels WHERE id = ?", id)
        .execute(db)
        .await?;
    Ok(())
}
