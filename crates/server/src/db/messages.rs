use sqlx::SqlitePool;
use shared::models::{Message, ReplyPreview};

pub struct MessageRow {
    pub id: Option<i64>,
    pub channel_id: i64,
    pub author_id: i64,
    pub content: String,
    pub created_at: String,
    pub reply_to_id: Option<i64>,
    pub pinned: i64,
}

pub fn to_model(row: &MessageRow) -> Message {
    let mentions = shared::mentions::parse(&row.content);
    Message {
        id: row.id.unwrap_or(0),
        channel_id: row.channel_id,
        author_id: row.author_id,
        content: row.content.clone(),
        created_at: row.created_at.clone(),
        attachments: vec![],
        reply_to: None,
        mentions,
        reactions: vec![],
        pinned: row.pinned != 0,
    }
}

pub async fn enrich_with_attachments(
    db: &SqlitePool,
    messages: &mut [Message],
) -> sqlx::Result<()> {
    let ids: Vec<i64> = messages.iter().map(|m| m.id).collect();
    let map = super::attachments::list_by_message_ids(db, &ids).await?;
    for msg in messages.iter_mut() {
        if let Some(atts) = map.get(&msg.id) {
            msg.attachments = atts.clone();
        }
    }
    Ok(())
}

pub async fn enrich_with_replies(
    db: &SqlitePool,
    messages: &mut [Message],
    rows: &[MessageRow],
) -> sqlx::Result<()> {
    let reply_ids: Vec<i64> = rows.iter().filter_map(|r| r.reply_to_id).collect();
    if reply_ids.is_empty() {
        return Ok(());
    }

    // Fetch reply previews in one query
    let placeholders = reply_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT id, author_id, content FROM messages WHERE id IN ({})",
        placeholders
    );
    let mut q = sqlx::query_as::<_, (i64, i64, String)>(&query);
    for id in &reply_ids {
        q = q.bind(id);
    }
    let previews: Vec<(i64, i64, String)> = q.fetch_all(db).await?;

    let preview_map: std::collections::HashMap<i64, ReplyPreview> = previews
        .into_iter()
        .map(|(id, author_id, content)| {
            let truncated = if content.len() > 100 {
                format!("{}...", &content[..100])
            } else {
                content
            };
            (id, ReplyPreview { id, author_id, content: truncated })
        })
        .collect();

    for (msg, row) in messages.iter_mut().zip(rows.iter()) {
        if let Some(reply_id) = row.reply_to_id {
            msg.reply_to = preview_map.get(&reply_id).cloned();
        }
    }
    Ok(())
}

pub async fn create(
    db: &SqlitePool,
    channel_id: i64,
    author_id: i64,
    content: &str,
    reply_to_id: Option<i64>,
) -> sqlx::Result<Message> {
    let row = sqlx::query_as!(
        MessageRow,
        r#"INSERT INTO messages (channel_id, author_id, content, reply_to_id) VALUES (?, ?, ?, ?) RETURNING id, channel_id, author_id, content, created_at as "created_at: String", reply_to_id, pinned"#,
        channel_id,
        author_id,
        content,
        reply_to_id
    )
    .fetch_one(db)
    .await?;

    let mut msg = to_model(&row);

    // Enrich reply preview if replying
    if row.reply_to_id.is_some() {
        enrich_with_replies(db, std::slice::from_mut(&mut msg), std::slice::from_ref(&row)).await?;
    }

    Ok(msg)
}

pub async fn find_by_id(db: &SqlitePool, id: i64) -> sqlx::Result<Option<MessageRow>> {
    sqlx::query_as!(
        MessageRow,
        r#"SELECT id, channel_id, author_id, content, created_at as "created_at: String", reply_to_id, pinned FROM messages WHERE id = ?"#,
        id
    )
    .fetch_optional(db)
    .await
}

pub async fn set_pinned(db: &SqlitePool, id: i64, pinned: bool) -> sqlx::Result<()> {
    let val = if pinned { 1 } else { 0 };
    sqlx::query!("UPDATE messages SET pinned = ? WHERE id = ?", val, id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn list_pinned(
    db: &SqlitePool,
    channel_id: i64,
) -> sqlx::Result<Vec<Message>> {
    let rows = sqlx::query_as!(
        MessageRow,
        r#"SELECT id, channel_id, author_id, content, created_at as "created_at: String", reply_to_id, pinned
           FROM messages WHERE channel_id = ? AND pinned = 1
           ORDER BY id DESC"#,
        channel_id
    )
    .fetch_all(db)
    .await?;

    let mut messages: Vec<Message> = rows.iter().map(to_model).collect();
    enrich_with_replies(db, &mut messages, &rows).await?;
    Ok(messages)
}

pub async fn delete(db: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM messages WHERE id = ?", id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn update_content(db: &SqlitePool, id: i64, content: &str) -> sqlx::Result<()> {
    sqlx::query!("UPDATE messages SET content = ? WHERE id = ?", content, id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn list_ids_by_channel(db: &SqlitePool, channel_id: i64) -> sqlx::Result<Vec<i64>> {
    let rows = sqlx::query_scalar!("SELECT id FROM messages WHERE channel_id = ?", channel_id)
        .fetch_all(db)
        .await?;
    Ok(rows.into_iter().flatten().collect())
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
                r#"SELECT id, channel_id, author_id, content, created_at as "created_at: String", reply_to_id, pinned FROM messages
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
                r#"SELECT id, channel_id, author_id, content, created_at as "created_at: String", reply_to_id, pinned FROM messages
                 WHERE channel_id = ?
                 ORDER BY id DESC LIMIT ?"#,
                channel_id,
                limit
            )
            .fetch_all(db)
            .await?
        }
    };

    let mut messages: Vec<Message> = rows.iter().map(to_model).collect();
    enrich_with_replies(db, &mut messages, &rows).await?;
    Ok(messages)
}

// ── By author ──

pub async fn list_by_author(
    db: &SqlitePool,
    author_id: i64,
    limit: i64,
    before_id: Option<i64>,
) -> sqlx::Result<Vec<Message>> {
    let rows = match before_id {
        Some(before) => {
            sqlx::query_as!(
                MessageRow,
                r#"SELECT id, channel_id, author_id, content, created_at as "created_at: String", reply_to_id, pinned FROM messages
                 WHERE author_id = ? AND id < ?
                 ORDER BY id DESC LIMIT ?"#,
                author_id,
                before,
                limit
            )
            .fetch_all(db)
            .await?
        }
        None => {
            sqlx::query_as!(
                MessageRow,
                r#"SELECT id, channel_id, author_id, content, created_at as "created_at: String", reply_to_id, pinned FROM messages
                 WHERE author_id = ?
                 ORDER BY id DESC LIMIT ?"#,
                author_id,
                limit
            )
            .fetch_all(db)
            .await?
        }
    };

    let mut messages: Vec<Message> = rows.iter().map(to_model).collect();
    enrich_with_replies(db, &mut messages, &rows).await?;
    Ok(messages)
}

// ── Search ──

pub async fn search(
    db: &SqlitePool,
    channel_id: i64,
    query: &str,
    limit: i64,
) -> sqlx::Result<Vec<Message>> {
    let raw: Vec<(i64, i64, i64, String, String, Option<i64>, i64)> = sqlx::query_as(
        r#"SELECT m.id, m.channel_id, m.author_id, m.content, m.created_at, m.reply_to_id, m.pinned
           FROM messages_fts f
           JOIN messages m ON m.id = f.rowid
           WHERE f.content MATCH ? AND m.channel_id = ?
           ORDER BY rank
           LIMIT ?"#,
    )
    .bind(query)
    .bind(channel_id)
    .bind(limit)
    .fetch_all(db)
    .await?;

    let rows: Vec<MessageRow> = raw
        .into_iter()
        .map(|(id, channel_id, author_id, content, created_at, reply_to_id, pinned)| MessageRow {
            id: Some(id),
            channel_id,
            author_id,
            content,
            created_at,
            reply_to_id,
            pinned,
        })
        .collect();

    let mut messages: Vec<Message> = rows.iter().map(to_model).collect();
    enrich_with_replies(db, &mut messages, &rows).await?;
    Ok(messages)
}

// ── Reactions ──

/// Toggle a reaction: removes if already exists, adds otherwise.
/// Returns `true` if added, `false` if removed.
pub async fn toggle_reaction(
    db: &SqlitePool,
    message_id: i64,
    user_id: i64,
    emoji: &str,
) -> sqlx::Result<bool> {
    let deleted = sqlx::query(
        "DELETE FROM message_reactions WHERE message_id = ? AND user_id = ? AND emoji = ?",
    )
    .bind(message_id)
    .bind(user_id)
    .bind(emoji)
    .execute(db)
    .await?;

    if deleted.rows_affected() > 0 {
        return Ok(false);
    }

    sqlx::query(
        "INSERT INTO message_reactions (message_id, user_id, emoji) VALUES (?, ?, ?)",
    )
    .bind(message_id)
    .bind(user_id)
    .bind(emoji)
    .execute(db)
    .await?;

    Ok(true)
}

pub async fn enrich_with_reactions(
    db: &SqlitePool,
    messages: &mut [Message],
) -> sqlx::Result<()> {
    if messages.is_empty() {
        return Ok(());
    }
    let ids: Vec<i64> = messages.iter().map(|m| m.id).collect();
    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let query = format!(
        "SELECT message_id, emoji, user_id FROM message_reactions WHERE message_id IN ({}) ORDER BY created_at",
        placeholders
    );
    let mut q = sqlx::query_as::<_, (i64, String, i64)>(&query);
    for id in &ids {
        q = q.bind(id);
    }
    let rows: Vec<(i64, String, i64)> = q.fetch_all(db).await?;

    // Group by message_id → ordered list of (emoji, user_ids)
    // Preserve insertion order by tracking first-seen position per emoji
    let mut map: std::collections::HashMap<i64, Vec<(String, Vec<i64>)>> =
        std::collections::HashMap::new();
    for (mid, emoji, uid) in rows {
        let entries = map.entry(mid).or_default();
        if let Some(entry) = entries.iter_mut().find(|(e, _)| *e == emoji) {
            entry.1.push(uid);
        } else {
            entries.push((emoji, vec![uid]));
        }
    }

    for msg in messages.iter_mut() {
        if let Some(entries) = map.remove(&msg.id) {
            msg.reactions = entries
                .into_iter()
                .map(|(emoji, user_ids)| shared::models::Reaction {
                    count: user_ids.len() as i64,
                    emoji,
                    user_ids,
                })
                .collect();
        }
    }
    Ok(())
}
