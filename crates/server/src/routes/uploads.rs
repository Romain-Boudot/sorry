use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::state::AppState;

/// GET /uploads/:msg_id/:filename
/// Auth required. Serves images inline, everything else as attachment.
pub async fn serve_upload(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
    Path((msg_id, filename)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    // Sanitize: reject path traversal
    if msg_id.contains("..") || filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let file_path = std::path::PathBuf::from(&state.upload_dir)
        .join(&msg_id)
        .join(&filename);

    let data = tokio::fs::read(&file_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

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
            // Block scripts even if browser sniffs HTML
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
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
