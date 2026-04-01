use sqlx::SqlitePool;

struct SettingRow {
    value: String,
}

pub async fn get_setting(db: &SqlitePool, key: &str) -> sqlx::Result<Option<String>> {
    let row = sqlx::query_as!(
        SettingRow,
        "SELECT value FROM server_settings WHERE key = ?",
        key
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| r.value))
}

pub async fn set_setting(db: &SqlitePool, key: &str, value: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO server_settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        key,
        value
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn delete_setting(db: &SqlitePool, key: &str) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM server_settings WHERE key = ?", key)
        .execute(db)
        .await?;
    Ok(())
}
