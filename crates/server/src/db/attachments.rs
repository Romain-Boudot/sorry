use sqlx::{FromRow, Row, SqlitePool};
use shared::models::Attachment;

#[derive(FromRow)]
struct AttachmentRow {
    id: i64,
    message_id: i64,
    filename: String,
    stored_name: String,
    content_type: String,
    size: i64,
}

fn to_model(row: &AttachmentRow) -> Attachment {
    Attachment {
        id: row.id,
        filename: row.filename.clone(),
        content_type: row.content_type.clone(),
        size: row.size,
        url: format!("/uploads/{}/{}", row.message_id, row.stored_name),
    }
}

pub async fn create(
    db: &SqlitePool,
    message_id: i64,
    filename: &str,
    stored_name: &str,
    content_type: &str,
    size: i64,
) -> sqlx::Result<Attachment> {
    let row = sqlx::query_as::<_, AttachmentRow>(
        "INSERT INTO attachments (message_id, filename, stored_name, content_type, size) VALUES (?, ?, ?, ?, ?) RETURNING id, message_id, filename, stored_name, content_type, size"
    )
    .bind(message_id)
    .bind(filename)
    .bind(stored_name)
    .bind(content_type)
    .bind(size)
    .fetch_one(db)
    .await?;

    Ok(to_model(&row))
}

pub async fn list_by_channel(
    db: &SqlitePool,
    channel_id: i64,
    limit: i64,
    before_id: Option<i64>,
) -> sqlx::Result<Vec<shared::models::ChannelAttachment>> {
    let rows: Vec<(i64, i64, i64, String, String, String, i64, String)> = match before_id {
        Some(before) => {
            sqlx::query_as(
                r#"SELECT a.id, a.message_id, m.author_id, a.filename, a.stored_name, a.content_type, a.size, a.created_at
                   FROM attachments a
                   JOIN messages m ON m.id = a.message_id
                   WHERE m.channel_id = ? AND a.id < ?
                   ORDER BY a.id DESC LIMIT ?"#,
            )
            .bind(channel_id)
            .bind(before)
            .bind(limit)
            .fetch_all(db)
            .await?
        }
        None => {
            sqlx::query_as(
                r#"SELECT a.id, a.message_id, m.author_id, a.filename, a.stored_name, a.content_type, a.size, a.created_at
                   FROM attachments a
                   JOIN messages m ON m.id = a.message_id
                   WHERE m.channel_id = ?
                   ORDER BY a.id DESC LIMIT ?"#,
            )
            .bind(channel_id)
            .bind(limit)
            .fetch_all(db)
            .await?
        }
    };

    Ok(rows
        .into_iter()
        .map(|(id, message_id, author_id, filename, stored_name, content_type, size, created_at)| {
            shared::models::ChannelAttachment {
                id,
                filename,
                content_type,
                size,
                url: format!("/uploads/{}/{}", message_id, stored_name),
                author_id,
                created_at,
            }
        })
        .collect())
}

pub async fn list_by_message_ids(
    db: &SqlitePool,
    message_ids: &[i64],
) -> sqlx::Result<std::collections::HashMap<i64, Vec<Attachment>>> {
    if message_ids.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let placeholders = message_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT id, message_id, filename, stored_name, content_type, size FROM attachments WHERE message_id IN ({}) ORDER BY id",
        placeholders
    );

    let mut q = sqlx::query(&query);
    for id in message_ids {
        q = q.bind(id);
    }

    let rows = q.fetch_all(db).await?;

    let mut map: std::collections::HashMap<i64, Vec<Attachment>> = std::collections::HashMap::new();
    for row in &rows {
        let att_row = AttachmentRow {
            id: row.get("id"),
            message_id: row.get("message_id"),
            filename: row.get("filename"),
            stored_name: row.get("stored_name"),
            content_type: row.get("content_type"),
            size: row.get("size"),
        };
        let msg_id = att_row.message_id;
        map.entry(msg_id).or_default().push(to_model(&att_row));
    }

    Ok(map)
}
