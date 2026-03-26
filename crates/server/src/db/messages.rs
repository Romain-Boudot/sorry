use sqlx::SqlitePool;
use shared::models::Message;

pub struct MessageRow {
    pub id: Option<i64>,
    pub channel_id: i64,
    pub author_id: i64,
    pub content: String,
    pub created_at: String,
}

pub fn to_model(row: &MessageRow) -> Message {
    Message {
        id: row.id.unwrap_or(0),
        channel_id: row.channel_id,
        author_id: row.author_id,
        content: row.content.clone(),
        created_at: row.created_at.clone(),
    }
}

pub async fn create(
    db: &SqlitePool,
    channel_id: i64,
    author_id: i64,
    content: &str,
) -> sqlx::Result<Message> {
    let row = sqlx::query_as!(
        MessageRow,
        r#"INSERT INTO messages (channel_id, author_id, content) VALUES (?, ?, ?) RETURNING id, channel_id, author_id, content, created_at as "created_at: String""#,
        channel_id,
        author_id,
        content
    )
    .fetch_one(db)
    .await?;

    Ok(to_model(&row))
}

pub async fn list_by_channel(
    db: &SqlitePool,
    channel_id: i64,
    limit: i64,
    before_id: Option<i64>,
) -> sqlx::Result<Vec<Message>> {
    let rows = match before_id {
        Some(before) => {
            sqlx::query_as!(
                MessageRow,
                r#"SELECT id, channel_id, author_id, content, created_at as "created_at: String" FROM messages
                 WHERE channel_id = ? AND id < ?
                 ORDER BY id DESC LIMIT ?"#,
                channel_id,
                before,
                limit
            )
            .fetch_all(db)
            .await?
        }
        None => {
            sqlx::query_as!(
                MessageRow,
                r#"SELECT id, channel_id, author_id, content, created_at as "created_at: String" FROM messages
                 WHERE channel_id = ?
                 ORDER BY id DESC LIMIT ?"#,
                channel_id,
                limit
            )
            .fetch_all(db)
            .await?
        }
    };

    Ok(rows.iter().map(to_model).collect())
}
