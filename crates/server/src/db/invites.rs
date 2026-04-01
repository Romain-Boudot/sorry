use sqlx::SqlitePool;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Invite {
    pub code: String,
    pub created_by: i64,
    pub max_uses: Option<i64>,
    pub uses: i64,
    pub expires_at: Option<i64>,
    pub created_at: i64,
}

pub async fn create(
    db: &SqlitePool,
    code: &str,
    created_by: i64,
    max_uses: Option<i64>,
    expires_at: Option<i64>,
) -> sqlx::Result<Invite> {
    sqlx::query!(
        "INSERT INTO invites (code, created_by, max_uses, expires_at) VALUES (?, ?, ?, ?)",
        code,
        created_by,
        max_uses,
        expires_at
    )
    .execute(db)
    .await?;

    find_by_code(db, code).await.map(|o| o.unwrap())
}

pub async fn find_by_code(db: &SqlitePool, code: &str) -> sqlx::Result<Option<Invite>> {
    let row = sqlx::query!(
        r#"SELECT code, created_by, max_uses, uses, expires_at, created_at FROM invites WHERE code = ?"#,
        code
    )
    .fetch_optional(db)
    .await?;
    Ok(row.map(|r| Invite {
        code: r.code.unwrap_or_default(),
        created_by: r.created_by,
        max_uses: r.max_uses,
        uses: r.uses,
        expires_at: r.expires_at,
        created_at: r.created_at,
    }))
}

pub async fn list_all(db: &SqlitePool) -> sqlx::Result<Vec<Invite>> {
    let rows = sqlx::query!(
        r#"SELECT code, created_by, max_uses, uses, expires_at, created_at FROM invites ORDER BY created_at DESC"#
    )
    .fetch_all(db)
    .await?;
    Ok(rows.into_iter().map(|r| Invite {
        code: r.code.unwrap_or_default(),
        created_by: r.created_by,
        max_uses: r.max_uses,
        uses: r.uses,
        expires_at: r.expires_at,
        created_at: r.created_at,
    }).collect())
}

/// Validate and consume an invite. Returns error if invalid/expired/used up.
pub async fn use_invite(db: &SqlitePool, code: &str) -> Result<(), String> {
    let invite = find_by_code(db, code)
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or_else(|| "Invite not found".to_string())?;

    // Check expiration
    if let Some(expires) = invite.expires_at {
        let now = chrono::Utc::now().timestamp();
        if now > expires {
            return Err("Invite expired".to_string());
        }
    }

    // Check max uses
    if let Some(max) = invite.max_uses {
        if invite.uses >= max {
            return Err("Invite has reached max uses".to_string());
        }
    }

    // Increment uses
    sqlx::query!("UPDATE invites SET uses = uses + 1 WHERE code = ?", code)
        .execute(db)
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    Ok(())
}

pub async fn delete(db: &SqlitePool, code: &str) -> sqlx::Result<()> {
    sqlx::query!("DELETE FROM invites WHERE code = ?", code)
        .execute(db)
        .await?;
    Ok(())
}
