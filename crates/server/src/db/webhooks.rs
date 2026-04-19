use sqlx::SqlitePool;
use shared::models::{Webhook, WebhookInfo};

pub async fn create(
    db: &SqlitePool,
    channel_id: i64,
    token: &str,
    name: &str,
    avatar_url: Option<&str>,
    created_by: i64,
) -> sqlx::Result<Webhook> {
    let row = sqlx::query!(
        r#"INSERT INTO webhooks (channel_id, token, name, avatar_url, created_by)
           VALUES (?, ?, ?, ?, ?)
           RETURNING id, channel_id, token as "token!", name as "name!", avatar_url, created_by, created_at as "created_at: String""#,
        channel_id, token, name, avatar_url, created_by,
    )
    .fetch_one(db)
    .await?;

    Ok(Webhook {
        id: row.id.unwrap_or_default(),
        channel_id: row.channel_id,
        token: row.token,
        name: row.name,
        avatar_url: row.avatar_url,
        created_by: row.created_by,
        created_at: row.created_at,
    })
}

pub async fn find_by_id(db: &SqlitePool, id: i64) -> sqlx::Result<Option<Webhook>> {
    let row = sqlx::query!(
        r#"SELECT id, channel_id, token as "token!", name as "name!", avatar_url, created_by, created_at as "created_at: String"
           FROM webhooks WHERE id = ?"#,
        id
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| Webhook {
        id: r.id,
        channel_id: r.channel_id,
        token: r.token,
        name: r.name,
        avatar_url: r.avatar_url,
        created_by: r.created_by,
        created_at: r.created_at,
    }))
}

pub async fn list_by_channel(db: &SqlitePool, channel_id: i64) -> sqlx::Result<Vec<Webhook>> {
    let rows = sqlx::query!(
        r#"SELECT id, channel_id, token as "token!", name as "name!", avatar_url, created_by, created_at as "created_at: String"
           FROM webhooks WHERE channel_id = ? ORDER BY id"#,
        channel_id
    )
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(|r| Webhook {
        id: r.id.unwrap_or(0),
        channel_id: r.channel_id,
        token: r.token,
        name: r.name,
        avatar_url: r.avatar_url,
        created_by: r.created_by,
        created_at: r.created_at,
    }).collect())
}

pub async fn list_all_info(db: &SqlitePool) -> sqlx::Result<Vec<WebhookInfo>> {
    let rows = sqlx::query!(
        r#"SELECT id, channel_id, name as "name!", avatar_url FROM webhooks ORDER BY id"#
    )
    .fetch_all(db)
    .await?;

    Ok(rows.into_iter().map(|r| WebhookInfo {
        id: r.id,
        channel_id: r.channel_id,
        name: r.name,
        avatar_url: r.avatar_url,
    }).collect())
}

pub async fn update(
    db: &SqlitePool,
    id: i64,
    name: &str,
    avatar_url: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE webhooks SET name = ?, avatar_url = ? WHERE id = ?",
        name, avatar_url, id,
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete(db: &SqlitePool, id: i64) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM webhooks WHERE id = ?", id)
        .execute(db)
        .await?;
    Ok(())
}
