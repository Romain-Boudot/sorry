use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    routing::{patch, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::state::AppState;

const ALLOWED_ICON_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];

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
) -> Result<StatusCode, StatusCode> {
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !shared::permissions::has(perms, shared::permissions::MANAGE_SERVER) {
        return Err(StatusCode::FORBIDDEN);
    }

    if let Some(name) = &payload.name {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.len() > 64 {
            return Err(StatusCode::BAD_REQUEST);
        }
        crate::db::servers::set_setting(&state.db, "name", trimmed)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if let Some(desc) = &payload.description {
        let trimmed = desc.trim();
        if trimmed.is_empty() {
            crate::db::servers::delete_setting(&state.db, "description")
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        } else if trimmed.len() > 256 {
            return Err(StatusCode::BAD_REQUEST);
        } else {
            crate::db::servers::set_setting(&state.db, "description", trimmed)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
) -> Result<Json<serde_json::Value>, StatusCode> {
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !shared::permissions::has(perms, shared::permissions::MANAGE_SERVER) {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut file_data: Option<(Vec<u8>, String, String)> = None;

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "icon" {
            let raw_filename = field.file_name().unwrap_or("icon.png").to_string();
            let ext = std::path::Path::new(&raw_filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !ALLOWED_ICON_EXTENSIONS.contains(&ext.as_str()) {
                return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
            }
            let content_type = field.content_type().unwrap_or("image/png").to_string();
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            if data.is_empty() || data.len() > 5 * 1024 * 1024 {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            file_data = Some((data.to_vec(), content_type, ext));
            break;
        }
    }

    let (data, content_type, ext) = file_data.ok_or(StatusCode::BAD_REQUEST)?;

    // Delete old icon
    let _ = crate::storage::delete_prefix(&state.storage, "server-icon/").await;

    // Upload new icon
    let stored_name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
    let key = format!("server-icon/{}", stored_name);

    crate::storage::upload(&state.storage, &key, &data, &content_type)
        .await
        .map_err(|e| {
            tracing::error!("Server icon upload failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let icon_url = format!("/server-icon/{}", stored_name);
    crate::db::servers::set_setting(&state.db, "icon_url", &icon_url)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    broadcast_server_update(&state).await;
    Ok(Json(serde_json::json!({ "icon_url": icon_url })))
}

/// DELETE /api/server/icon — remove server icon (MANAGE_SERVER)
async fn delete_icon(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<StatusCode, StatusCode> {
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !shared::permissions::has(perms, shared::permissions::MANAGE_SERVER) {
        return Err(StatusCode::FORBIDDEN);
    }

    let _ = crate::storage::delete_prefix(&state.storage, "server-icon/").await;
    let _ = crate::db::servers::delete_setting(&state.db, "icon_url").await;

    broadcast_server_update(&state).await;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /server-icon/:filename — serve server icon (public)
pub async fn serve_icon(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Result<axum::response::Response, StatusCode> {
    use axum::http::header;
    use axum::response::IntoResponse;

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let key = format!("server-icon/{}", filename);
    let data = crate::storage::download(&state.storage, &key)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    let content_type = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    };

    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable".to_string()),
        ],
        data,
    ).into_response())
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", patch(update_server))
        .route("/icon", post(upload_icon).delete(delete_icon))
}
