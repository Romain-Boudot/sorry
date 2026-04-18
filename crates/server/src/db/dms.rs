use sqlx::SqlitePool;
use shared::models::{DmConversation, DmMessage, Reaction};

pub async fn create(
    db: &SqlitePool,
    sender_id: i64,
    recipient_id: i64,
    ciphertext: &str,
    nonce: &str,
    sender_key_fingerprint: &str,
    reply_to_id: Option<i64>,
) -> sqlx::Result<DmMessage> {
    let row = sqlx::query!(
        r#"INSERT INTO dm_messages (sender_id, recipient_id, ciphertext, nonce, sender_key_fingerprint, reply_to_id)
           VALUES (?, ?, ?, ?, ?, ?)
           RETURNING id as "id!: i64", CAST(created_at AS TEXT) as "created_at!: String""#,
        sender_id,
        recipient_id,
        ciphertext,
        nonce,
        sender_key_fingerprint,
        reply_to_id,
    )
    .fetch_one(db)
    .await?;

    Ok(DmMessage {
        id: row.id,
        sender_id,
        recipient_id,
        ciphertext: ciphertext.to_string(),
        nonce: nonce.to_string(),
        sender_key_fingerprint: sender_key_fingerprint.to_string(),
        created_at: row.created_at,
        reply_to_id,
        edited: false,
        reactions: vec![],
    })
}

struct DmRow {
    id: i64,
    sender_id: i64,
    recipient_id: i64,
    ciphertext: String,
    nonce: String,
    sender_key_fingerprint: String,
    created_at: String,
    reply_to_id: Option<i64>,
    edited: i64,
}

fn row_to_model(r: DmRow) -> DmMessage {
    DmMessage {
        id: r.id,
        sender_id: r.sender_id,
        recipient_id: r.recipient_id,
        ciphertext: r.ciphertext,
        nonce: r.nonce,
        sender_key_fingerprint: r.sender_key_fingerprint,
        created_at: r.created_at,
        reply_to_id: r.reply_to_id,
        edited: r.edited != 0,
        reactions: vec![],
    }
}

/// All messages exchanged between `me` and `other`, newest first, paginated via `before`.
pub async fn list_with(
    db: &SqlitePool,
    me: i64,
    other: i64,
    limit: i64,
    before: Option<i64>,
) -> sqlx::Result<Vec<DmMessage>> {
    let before_filter = before.unwrap_or(i64::MAX);
    let rows = sqlx::query_as!(
        DmRow,
        r#"SELECT id as "id!: i64",
                  sender_id as "sender_id!: i64",
                  recipient_id as "recipient_id!: i64",
                  ciphertext,
                  nonce,
                  sender_key_fingerprint,
                  CAST(created_at AS TEXT) as "created_at!: String",
                  reply_to_id as "reply_to_id?: i64",
                  edited as "edited!: i64"
           FROM dm_messages
           WHERE ((sender_id = ? AND recipient_id = ?) OR (sender_id = ? AND recipient_id = ?))
             AND id < ?
           ORDER BY id DESC
           LIMIT ?"#,
        me, other, other, me, before_filter, limit
    )
    .fetch_all(db)
    .await?;

    let mut messages: Vec<DmMessage> = rows.into_iter().map(row_to_model).collect();
    enrich_with_reactions(db, &mut messages).await?;
    Ok(messages)
}

pub async fn find_by_id(db: &SqlitePool, id: i64) -> sqlx::Result<Option<DmMessage>> {
    let row = sqlx::query_as!(
        DmRow,
        r#"SELECT id as "id!: i64",
                  sender_id as "sender_id!: i64",
                  recipient_id as "recipient_id!: i64",
                  ciphertext,
                  nonce,
                  sender_key_fingerprint,
                  CAST(created_at AS TEXT) as "created_at!: String",
                  reply_to_id as "reply_to_id?: i64",
                  edited as "edited!: i64"
           FROM dm_messages WHERE id = ?"#,
        id
    )
    .fetch_optional(db)
    .await?;

    let Some(r) = row else { return Ok(None) };
    let mut dm = row_to_model(r);
    enrich_with_reactions(db, std::slice::from_mut(&mut dm)).await?;
    Ok(Some(dm))
}

pub async fn update_ciphertext(
    db: &SqlitePool,
    id: i64,
    ciphertext: &str,
    nonce: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE dm_messages SET ciphertext = ?, nonce = ?, edited = 1 WHERE id = ?",
        ciphertext,
        nonce,
        id
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete(db: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM dm_messages WHERE id = ?", id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn toggle_reaction(
    db: &SqlitePool,
    dm_id: i64,
    user_id: i64,
    emoji: &str,
) -> sqlx::Result<bool> {
    let existing = sqlx::query!(
        r#"SELECT emoji FROM dm_reactions WHERE dm_id = ? AND user_id = ? AND emoji = ?"#,
        dm_id, user_id, emoji
    )
    .fetch_optional(db)
    .await?;

    if existing.is_some() {
        sqlx::query!(
            "DELETE FROM dm_reactions WHERE dm_id = ? AND user_id = ? AND emoji = ?",
            dm_id, user_id, emoji
        )
        .execute(db)
        .await?;
        Ok(false)
    } else {
        sqlx::query!(
            "INSERT INTO dm_reactions (dm_id, user_id, emoji) VALUES (?, ?, ?)",
            dm_id, user_id, emoji
        )
        .execute(db)
        .await?;
        Ok(true)
    }
}

async fn enrich_with_reactions(db: &SqlitePool, messages: &mut [DmMessage]) -> sqlx::Result<()> {
    if messages.is_empty() { return Ok(()); }
    use std::collections::HashMap;
    let ids: Vec<i64> = messages.iter().map(|m| m.id).collect();
    let placeholders = vec!["?"; ids.len()].join(",");
    let query_str = format!(
        "SELECT dm_id, user_id, emoji FROM dm_reactions WHERE dm_id IN ({})",
        placeholders
    );
    let mut q = sqlx::query_as::<_, (i64, i64, String)>(&query_str);
    for id in &ids { q = q.bind(id); }
    let rows = q.fetch_all(db).await?;

    let mut by_msg: HashMap<i64, HashMap<String, Vec<i64>>> = HashMap::new();
    for (dm_id, user_id, emoji) in rows {
        by_msg.entry(dm_id).or_default().entry(emoji).or_default().push(user_id);
    }
    for m in messages.iter_mut() {
        if let Some(emoji_map) = by_msg.remove(&m.id) {
            let mut reactions: Vec<Reaction> = emoji_map.into_iter().map(|(emoji, user_ids)| {
                let count = user_ids.len() as i64;
                Reaction { emoji, count, user_ids }
            }).collect();
            reactions.sort_by(|a, b| b.count.cmp(&a.count));
            m.reactions = reactions;
        }
    }
    Ok(())
}

/// One row per peer, with the id/timestamp of the most recent exchange.
pub async fn list_conversations(db: &SqlitePool, me: i64) -> sqlx::Result<Vec<DmConversation>> {
    let rows = sqlx::query!(
        r#"SELECT peer_id as "user_id!: i64",
                  MAX(id) as "last_message_id!: i64",
                  CAST(MAX(created_at) AS TEXT) as "last_message_at!: String"
           FROM (
               SELECT id, created_at,
                      CASE WHEN sender_id = ? THEN recipient_id ELSE sender_id END AS peer_id
               FROM dm_messages
               WHERE sender_id = ? OR recipient_id = ?
           )
           GROUP BY peer_id
           ORDER BY MAX(id) DESC"#,
        me, me, me,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| DmConversation {
            user_id: r.user_id,
            last_message_id: r.last_message_id,
            last_message_at: r.last_message_at,
        })
        .collect())
}
