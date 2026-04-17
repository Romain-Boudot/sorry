use sqlx::SqlitePool;
use shared::models::AuditLog;

pub struct AuditRow {
    pub id: Option<i64>,
    pub actor_id: i64,
    pub action: String,
    pub target_user_id: Option<i64>,
    pub target_channel_id: Option<i64>,
    pub target_role_id: Option<i64>,
    pub details: Option<String>,
    pub created_at: String,
}

fn to_model(row: &AuditRow) -> AuditLog {
    AuditLog {
        id: row.id.unwrap_or(0),
        actor_id: row.actor_id,
        action: row.action.clone(),
        target_user_id: row.target_user_id,
        target_channel_id: row.target_channel_id,
        target_role_id: row.target_role_id,
        details: row.details.clone(),
        created_at: row.created_at.clone(),
    }
}

pub async fn log(
    db: &SqlitePool,
    actor_id: i64,
    action: &str,
    target_user_id: Option<i64>,
    target_channel_id: Option<i64>,
    target_role_id: Option<i64>,
    details: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO audit_log (actor_id, action, target_user_id, target_channel_id, target_role_id, details)
         VALUES (?, ?, ?, ?, ?, ?)",
        actor_id, action, target_user_id, target_channel_id, target_role_id, details
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn list(
    db: &SqlitePool,
    limit: i64,
    before_id: Option<i64>,
) -> sqlx::Result<Vec<AuditLog>> {
    let rows = match before_id {
        Some(b) => sqlx::query_as!(
            AuditRow,
            r#"SELECT id, actor_id, action, target_user_id, target_channel_id, target_role_id, details, created_at as "created_at!: String"
               FROM audit_log WHERE id < ? ORDER BY id DESC LIMIT ?"#,
            b, limit
        ).fetch_all(db).await?,
        None => sqlx::query_as!(
            AuditRow,
            r#"SELECT id, actor_id, action, target_user_id, target_channel_id, target_role_id, details, created_at as "created_at!: String"
               FROM audit_log ORDER BY id DESC LIMIT ?"#,
            limit
        ).fetch_all(db).await?,
    };
    Ok(rows.iter().map(to_model).collect())
}

pub async fn list_by_actor(
    db: &SqlitePool,
    actor_id: i64,
    limit: i64,
    before_id: Option<i64>,
) -> sqlx::Result<Vec<AuditLog>> {
    let rows = match before_id {
        Some(b) => sqlx::query_as!(
            AuditRow,
            r#"SELECT id, actor_id, action, target_user_id, target_channel_id, target_role_id, details, created_at as "created_at!: String"
               FROM audit_log WHERE actor_id = ? AND id < ? ORDER BY id DESC LIMIT ?"#,
            actor_id, b, limit
        ).fetch_all(db).await?,
        None => sqlx::query_as!(
            AuditRow,
            r#"SELECT id, actor_id, action, target_user_id, target_channel_id, target_role_id, details, created_at as "created_at!: String"
               FROM audit_log WHERE actor_id = ? ORDER BY id DESC LIMIT ?"#,
            actor_id, limit
        ).fetch_all(db).await?,
    };
    Ok(rows.iter().map(to_model).collect())
}
