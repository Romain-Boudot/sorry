use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    routing::{get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;
use crate::upload;

async fn broadcast_server_update(state: &AppState) {
    let name = crate::db::servers::get_setting(&state.db, "name")
        .await.ok().flatten()
        .unwrap_or_else(|| state.server_name.clone());
    let description = crate::db::servers::get_setting(&state.db, "description")
        .await.ok().flatten();
    let icon_url = crate::db::servers::get_setting(&state.db, "icon_url")
        .await.ok().flatten();
    state.broadcast(shared::events::ServerEvent::ServerUpdate { name, description, icon_url });
}

#[derive(Deserialize)]
pub struct UpdateServerPayload {
    name: Option<String>,
    description: Option<String>,
}

/// PATCH /api/server — update server name/description (MANAGE_SERVER)
async fn update_server(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<UpdateServerPayload>,
) -> Result<StatusCode, AppError> {
    crate::perms::require_permission(&state.db, auth.0, shared::permissions::MANAGE_SERVER).await?;

    if let Some(name) = &payload.name {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.len() > 64 {
            return Err(AppError::BadRequest("Name must be 1-64 chars".into()));
        }
        crate::db::servers::set_setting(&state.db, "name", trimmed).await
            .map_err(|e| AppError::Internal(e.to_string()))?;
    }

    if let Some(desc) = &payload.description {
        let trimmed = desc.trim();
        if trimmed.is_empty() {
            crate::db::servers::delete_setting(&state.db, "description").await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        } else if trimmed.len() > 256 {
            return Err(AppError::BadRequest("Description too long".into()));
        } else {
            crate::db::servers::set_setting(&state.db, "description", trimmed).await
                .map_err(|e| AppError::Internal(e.to_string()))?;
        }
    }

    broadcast_server_update(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/server/icon — upload server icon (MANAGE_SERVER)
async fn upload_icon(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<serde_json::Value>, AppError> {
    crate::perms::require_permission(&state.db, auth.0, shared::permissions::MANAGE_SERVER).await?;

    let file = upload::parse_single_image(&mut multipart, "icon", 5 * 1024 * 1024).await?;

    let _ = crate::storage::delete_prefix(&state.storage, "server-icon/").await;

    let stored_name = format!("{}.{}", uuid::Uuid::new_v4(), file.ext);
    let key = format!("server-icon/{}", stored_name);

    crate::storage::upload(&state.storage, &key, &file.data, &file.content_type)
        .await
        .map_err(|e| AppError::Internal(format!("Server icon upload failed: {e}")))?;

    let icon_url = format!("/server-icon/{}", stored_name);
    crate::db::servers::set_setting(&state.db, "icon_url", &icon_url)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    broadcast_server_update(&state).await;
    Ok(Json(serde_json::json!({ "icon_url": icon_url })))
}

/// DELETE /api/server/icon — remove server icon (MANAGE_SERVER)
async fn delete_icon(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<StatusCode, AppError> {
    crate::perms::require_permission(&state.db, auth.0, shared::permissions::MANAGE_SERVER).await?;

    let _ = crate::storage::delete_prefix(&state.storage, "server-icon/").await;
    let _ = crate::db::servers::delete_setting(&state.db, "icon_url").await;

    broadcast_server_update(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /server-icon/:filename — serve server icon (public)
pub async fn serve_icon(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Result<axum::response::Response, AppError> {
    use axum::http::header;
    use axum::response::IntoResponse;

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(AppError::BadRequest("Invalid path".into()));
    }

    let key = format!("server-icon/{}", filename);
    let data = crate::storage::download(&state.storage, &key)
        .await
        .map_err(|_| AppError::NotFound)?;

    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    let content_type = upload::content_type_from_ext(&ext);

    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable".to_string()),
        ],
        data,
    ).into_response())
}

#[derive(Serialize)]
struct ServerStats {
    version: &'static str,
    uptime_secs: u64,
    users_total: i64,
    users_online: usize,
    users_guests: i64,
    channels_text: i64,
    channels_voice: i64,
    messages_total: i64,
    messages_today: i64,
    files_total: i64,
    files_size_bytes: i64,
    bans_active: usize,
    invites_active: i64,
    db_size_bytes: u64,
    disk_free_bytes: u64,
    disk_total_bytes: u64,
}

/// GET /api/server/stats — server statistics (MANAGE_SERVER)
async fn stats(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<ServerStats>, AppError> {
    crate::perms::require_permission(&state.db, auth.0, shared::permissions::MANAGE_SERVER).await?;

    let users_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db).await.unwrap_or(0);
    let users_guests: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE guest = 1")
        .fetch_one(&state.db).await.unwrap_or(0);
    let channels_text: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channels WHERE kind = 'text'")
        .fetch_one(&state.db).await.unwrap_or(0);
    let channels_voice: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channels WHERE kind = 'voice'")
        .fetch_one(&state.db).await.unwrap_or(0);
    let messages_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(&state.db).await.unwrap_or(0);
    let messages_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM messages WHERE created_at >= date('now')"
    ).fetch_one(&state.db).await.unwrap_or(0);
    let files_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM attachments")
        .fetch_one(&state.db).await.unwrap_or(0);
    let files_size_bytes: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size), 0) FROM attachments")
        .fetch_one(&state.db).await.unwrap_or(0);
    let invites_active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM invites")
        .fetch_one(&state.db).await.unwrap_or(0);

    let users_online = state.online_users.read().unwrap().len();
    let bans_active = state.banned_users.read().unwrap().len();
    let uptime_secs = state.started_at.elapsed().as_secs();

    // DB file size
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data.db".to_string());
    let db_path = db_url.strip_prefix("sqlite:").unwrap_or("./data.db");
    let db_size_bytes = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);

    // Disk space
    let upload_dir = std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./data/uploads".to_string());
    let (disk_free_bytes, disk_total_bytes) = get_disk_space(&upload_dir);

    Ok(Json(ServerStats {
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs,
        users_total,
        users_online,
        users_guests,
        channels_text,
        channels_voice,
        messages_total,
        messages_today,
        files_total,
        files_size_bytes,
        bans_active,
        invites_active,
        db_size_bytes,
        disk_free_bytes,
        disk_total_bytes,
    }))
}

#[cfg(unix)]
fn get_disk_space(path: &str) -> (u64, u64) {
    use std::ffi::CString;
    unsafe {
        let c_path = CString::new(path).unwrap_or_else(|_| CString::new(".").unwrap());
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
            let free = stat.f_bavail as u64 * stat.f_frsize as u64;
            let total = stat.f_blocks as u64 * stat.f_frsize as u64;
            (free, total)
        } else {
            (0, 0)
        }
    }
}

#[cfg(not(unix))]
fn get_disk_space(_path: &str) -> (u64, u64) {
    (0, 0)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", patch(update_server))
        .route("/icon", post(upload_icon).delete(delete_icon))
        .route("/stats", get(stats))
}
