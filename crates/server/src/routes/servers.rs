use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    routing::{patch, post},
    Json, Router,
};
use serde::Deserialize;
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

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", patch(update_server))
        .route("/icon", post(upload_icon).delete(delete_icon))
}
