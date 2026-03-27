use sqlx::SqlitePool;
use shared::models::ChannelGroup;

pub struct GroupRow {
    pub id: Option<i64>,
    pub name: String,
    pub position: i64,
}

pub fn to_model(row: &GroupRow) -> ChannelGroup {
    ChannelGroup {
        id: row.id.unwrap_or(0),
        name: row.name.clone(),
        position: row.position,
    }
}

pub async fn create(db: &SqlitePool, name: &str, position: i64) -> sqlx::Result<i64> {
    let row = sqlx::query!(
        "INSERT INTO channel_groups (name, position) VALUES (?, ?) RETURNING id",
        name,
        position
    )
    .fetch_one(db)
    .await?;
    Ok(row.id)
}

pub async fn list_all(db: &SqlitePool) -> sqlx::Result<Vec<ChannelGroup>> {
    let rows = sqlx::query_as!(
        GroupRow,
        "SELECT id, name, position FROM channel_groups ORDER BY position"
    )
    .fetch_all(db)
    .await?;
    Ok(rows.iter().map(to_model).collect())
}

pub async fn update(db: &SqlitePool, id: i64, name: &str) -> sqlx::Result<()> {
    sqlx::query!("UPDATE channel_groups SET name = ? WHERE id = ?", name, id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn delete(db: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM channel_groups WHERE id = ?", id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn reorder(db: &SqlitePool, ids: &[i64]) -> sqlx::Result<()> {
    for (i, id) in ids.iter().enumerate() {
        let pos = i as i64;
        sqlx::query!(
            "UPDATE channel_groups SET position = ? WHERE id = ?",
            pos,
            id
        )
        .execute(db)
        .await?;
    }
    Ok(())
}
