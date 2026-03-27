use sqlx::SqlitePool;
use shared::models::{Channel, ChannelKind};

pub struct ChannelRow {
    pub id: Option<i64>,
    pub name: String,
    pub kind: String,
    pub position: i64,
    pub group_id: Option<i64>,
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
        group_id: row.group_id,
    }
}

pub async fn create(db: &SqlitePool, name: &str, kind: &str, group_id: Option<i64>) -> sqlx::Result<i64> {
    let row = sqlx::query!(
        "INSERT INTO channels (name, kind, group_id) VALUES (?, ?, ?) RETURNING id",
        name,
        kind,
        group_id
    )
    .fetch_one(db)
    .await?;
    Ok(row.id)
}

pub async fn list_all(db: &SqlitePool) -> sqlx::Result<Vec<Channel>> {
    let rows = sqlx::query_as!(
        ChannelRow,
        "SELECT id, name, kind, position, group_id FROM channels ORDER BY position"
    )
    .fetch_all(db)
    .await?;

    Ok(rows.iter().map(to_model).collect())
}

pub async fn find_by_id(db: &SqlitePool, id: i64) -> sqlx::Result<Option<ChannelRow>> {
    sqlx::query_as!(
        ChannelRow,
        "SELECT id, name, kind, position, group_id FROM channels WHERE id = ?",
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

pub async fn update_group(db: &SqlitePool, id: i64, group_id: Option<i64>) -> sqlx::Result<()> {
    sqlx::query!("UPDATE channels SET group_id = ? WHERE id = ?", group_id, id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn reorder(db: &SqlitePool, ids: &[i64]) -> sqlx::Result<()> {
    for (i, id) in ids.iter().enumerate() {
        let pos = i as i64;
        sqlx::query!(
            "UPDATE channels SET position = ? WHERE id = ?",
            pos,
            id
        )
        .execute(db)
        .await?;
    }
    Ok(())
}
