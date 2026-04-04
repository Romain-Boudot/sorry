use sqlx::SqlitePool;

pub async fn get_secret(db: &SqlitePool, user_id: i64) -> Result<Option<(String, bool)>, sqlx::Error> {
    sqlx::query_as::<_, (String, bool)>(
        "SELECT secret, verified != 0 FROM user_totp WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_optional(db)
    .await
}

pub async fn save_secret(db: &SqlitePool, user_id: i64, secret: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO user_totp (user_id, secret, verified) VALUES (?, ?, 0)
         ON CONFLICT(user_id) DO UPDATE SET secret = excluded.secret, verified = 0"
    )
    .bind(user_id)
    .bind(secret)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn verify_enable(db: &SqlitePool, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE user_totp SET verified = 1 WHERE user_id = ?")
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn disable(db: &SqlitePool, user_id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM user_totp WHERE user_id = ?")
        .bind(user_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn is_enabled(db: &SqlitePool, user_id: i64) -> Result<bool, sqlx::Error> {
    let row = sqlx::query_scalar::<_, i32>(
        "SELECT verified FROM user_totp WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_optional(db)
    .await?;
    Ok(row == Some(1))
}
