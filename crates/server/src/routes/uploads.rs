use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::state::AppState;

/// GET /uploads/:msg_id/:filename
/// Public — UUID filenames are unguessable.
pub async fn serve_upload(
    State(state): State<Arc<AppState>>,
    Path((msg_id, filename)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    // Sanitize: reject path traversal
    if msg_id.contains("..") || filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let key = format!("{}/{}", msg_id, filename);

    let data = crate::storage::download(&state.bucket, &key)
        .await
        .map_err(|e| {
            tracing::error!("Failed to download '{}': {}", key, e);
            StatusCode::NOT_FOUND
        })?;

    // Infer content type from extension
    let content_type = mime_from_ext(&filename);
    let is_safe = (content_type.starts_with("image/") && content_type != "image/svg+xml")
        || content_type.starts_with("video/")
        || content_type.starts_with("audio/");

    let disposition = if is_safe {
        format!("inline; filename=\"{}\"", filename)
    } else {
        format!("attachment; filename=\"{}\"", filename)
    };

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, disposition),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
            // Files are immutable (UUID names) — cache forever
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable".to_string()),
        ],
        data,
    )
        .into_response())
}

fn mime_from_ext(filename: &str) -> String {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "ogg" => "audio/ogg",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "txt" => "text/plain",
        "json" => "application/json",
        _ => "application/octet-stream",
    }
    .to_string()
}
