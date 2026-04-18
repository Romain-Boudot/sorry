//! Shared multipart upload parsing and validation.

use axum::extract::Multipart;
use crate::error::AppError;

// ── Allowed extensions ──

pub const ALLOWED_IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];

pub const ALLOWED_FILE_EXTENSIONS: &[&str] = &[
    // Images
    "png", "jpg", "jpeg", "gif", "webp", "svg",
    // Video
    "mp4", "webm", "mov",
    // Audio
    "mp3", "ogg", "wav", "flac",
    // Documents
    "pdf", "txt", "json", "csv",
    // Archives
    "zip", "tar", "gz", "7z", "rar",
];

pub const MAX_FILES_PER_MESSAGE: usize = 10;

// ── Helpers ──

pub fn sanitize_filename(name: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if name.len() > 255 {
        name[..255].to_string()
    } else if name.is_empty() {
        "file".to_string()
    } else {
        name
    }
}

pub fn file_extension(filename: &str) -> String {
    std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

pub fn is_allowed_extension(filename: &str, allowed: &[&str]) -> bool {
    let ext = file_extension(filename);
    allowed.contains(&ext.as_str())
}

// ── Parsed file ──

pub struct ParsedFile {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
    pub ext: String,
}

/// Parse a single image from a multipart form (avatar, icon).
/// Accepts fields named `file` or `alt_name`.
pub async fn parse_single_image(
    multipart: &mut Multipart,
    alt_name: &str,
    max_size: usize,
) -> Result<ParsedFile, AppError> {
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == alt_name {
            let raw_filename = field.file_name().unwrap_or("image.png").to_string();
            let ext = file_extension(&raw_filename);
            if !ALLOWED_IMAGE_EXTENSIONS.contains(&ext.as_str()) {
                return Err(AppError::UnsupportedMediaType);
            }
            let content_type = field.content_type().unwrap_or("image/png").to_string();
            let data = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            if data.is_empty() || data.len() > max_size {
                return Err(AppError::PayloadTooLarge);
            }
            return Ok(ParsedFile {
                filename: raw_filename,
                content_type,
                data: data.to_vec(),
                ext,
            });
        }
    }
    Err(AppError::BadRequest("No file field found".into()))
}

/// Parse a message upload: content + optional reply_to_id + multiple files.
pub struct ParsedMessageUpload {
    pub content: String,
    pub reply_to_id: Option<i64>,
    pub nonce: Option<String>,
    pub files: Vec<ParsedFile>,
}

pub async fn parse_message_upload(
    multipart: &mut Multipart,
    max_file_size: usize,
) -> Result<ParsedMessageUpload, AppError> {
    let mut content = String::new();
    let mut reply_to_id: Option<i64> = None;
    let mut nonce: Option<String> = None;
    let mut files: Vec<ParsedFile> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "content" => {
                content = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            }
            "reply_to_id" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                reply_to_id = val.parse().ok();
            }
            "nonce" => {
                let val = field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                if !val.is_empty() {
                    nonce = Some(val);
                }
            }
            "file" => {
                if files.len() >= MAX_FILES_PER_MESSAGE {
                    return Err(AppError::BadRequest("Too many files".into()));
                }
                let raw_filename = field.file_name().unwrap_or("file").to_string();
                let filename = sanitize_filename(&raw_filename);
                if !is_allowed_extension(&filename, ALLOWED_FILE_EXTENSIONS) {
                    return Err(AppError::UnsupportedMediaType);
                }
                let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
                let data = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                if data.is_empty() {
                    return Err(AppError::BadRequest("Empty file".into()));
                }
                if data.len() > max_file_size {
                    return Err(AppError::PayloadTooLarge);
                }
                let ext = file_extension(&filename);
                files.push(ParsedFile {
                    filename,
                    content_type,
                    data: data.to_vec(),
                    ext,
                });
            }
            _ => {}
        }
    }

    if content.is_empty() && files.is_empty() {
        return Err(AppError::BadRequest("Empty message".into()));
    }

    Ok(ParsedMessageUpload { content, reply_to_id, nonce, files })
}

/// Determine content-type from file extension (for serving).
pub fn content_type_from_ext(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
