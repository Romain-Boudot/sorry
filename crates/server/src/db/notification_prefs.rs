use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPref {
    pub scope: String,       // "channel" or "server"
    pub target_id: i64,      // channel_id or 0
    pub level: String,       // "all", "mentions", "nothing"
    pub mute_until: Option<String>,
}

pub async fn get_all(db: &SqlitePool, user_id: i64) -> sqlx::Result<Vec<NotificationPref>> {
    let rows = sqlx::query_as!(
        NotificationPref,
        "SELECT scope, target_id, level, mute_until FROM notification_preferences WHERE user_id = ?",
        user_id
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

pub async fn upsert(
    db: &SqlitePool,
    user_id: i64,
    scope: &str,
    target_id: i64,
    level: &str,
    mute_until: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO notification_preferences (user_id, scope, target_id, level, mute_until)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(user_id, scope, target_id) DO UPDATE SET level = excluded.level, mute_until = excluded.mute_until",
        user_id,
        scope,
        target_id,
        level,
        mute_until
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete(
    db: &SqlitePool,
    user_id: i64,
    scope: &str,
    target_id: i64,
) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM notification_preferences WHERE user_id = ? AND scope = ? AND target_id = ?",
        user_id,
        scope,
        target_id
    )
    .execute(db)
    .await?;
    Ok(())
}
