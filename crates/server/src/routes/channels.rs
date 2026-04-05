use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::state::AppState;
use shared::permissions;

async fn require_permission(
    db: &sqlx::SqlitePool,
    user_id: i64,
    permission: i64,
) -> Result<(), StatusCode> {
    let perms = crate::db::roles::get_user_permissions(db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !permissions::has(perms, permission) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

async fn require_channel_permission(
    db: &sqlx::SqlitePool,
    user_id: i64,
    channel_id: i64,
    permission: i64,
) -> Result<(), StatusCode> {
    let role_perms = crate::db::roles::get_user_permissions(db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Admin bypass tout
    if permissions::has(role_perms, permissions::ADMINISTRATOR) {
        return Ok(());
    }

    let (allow, deny) = crate::db::roles::get_channel_overwrites(db, user_id, channel_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let final_perms = permissions::compute(&[role_perms], allow, deny);

    if !permissions::has(final_perms, permission) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct CreateChannelPayload {
    name: String,
    kind: String,
    group_id: Option<i64>,
}

#[derive(Deserialize)]
pub struct ListMessagesQuery {
    limit: Option<i64>,
    before: Option<i64>,
}

#[derive(Deserialize)]
pub struct SendMessagePayload {
    content: String,
    reply_to_id: Option<i64>,
}

/// GET /api/channels
async fn list_channels(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<shared::models::Channel>>, StatusCode> {
    require_permission(&state.db, auth.0, permissions::VIEW_CHANNELS).await?;

    let channels = crate::db::channels::list_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(channels))
}

/// POST /api/channels
async fn create_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateChannelPayload>,
) -> Result<Json<shared::models::Channel>, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    let kind = match payload.kind.as_str() {
        "text" | "voice" => payload.kind.as_str(),
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let id = crate::db::channels::create(&state.db, &payload.name, kind, payload.group_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(shared::models::Channel {
        id,
        name: payload.name,
        kind: match kind {
            "voice" => shared::models::ChannelKind::Voice,
            _ => shared::models::ChannelKind::Text,
        },
        position: 0,
        group_id: payload.group_id,
    }))
}

#[derive(Deserialize)]
pub struct UpdateChannelPayload {
    name: Option<String>,
}

/// PATCH /api/channels/:id
async fn update_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateChannelPayload>,
) -> Result<Json<shared::models::Channel>, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    let row = crate::db::channels::find_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if let Some(ref name) = payload.name {
        crate::db::channels::update_name(&state.db, id, name)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    let name = payload.name.unwrap_or(row.name);
    Ok(Json(shared::models::Channel {
        id,
        name,
        kind: match row.kind.as_str() {
            "voice" => shared::models::ChannelKind::Voice,
            _ => shared::models::ChannelKind::Text,
        },
        position: row.position,
        group_id: row.group_id,
    }))
}

/// DELETE /api/channels/:id
async fn delete_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    crate::db::channels::find_by_id(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Collect message IDs before cascade delete so we can clean up S3
    let message_ids = crate::db::messages::list_ids_by_channel(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    crate::db::channels::delete(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Clean up files for all deleted messages
    if !message_ids.is_empty() {
        let storage = state.storage.clone();
        tokio::spawn(async move {
            for msg_id in message_ids {
                if let Err(e) = crate::storage::delete_prefix(&storage, &format!("{}/", msg_id)).await {
                    tracing::error!("Failed to clean up files for message {}: {}", msg_id, e);
                }
            }
        });
    }

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/channels/:id/messages?limit=50&before=123
async fn list_messages(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<Vec<shared::models::Message>>, StatusCode> {
    require_channel_permission(&state.db, auth.0, channel_id, permissions::READ_MESSAGE_HISTORY)
        .await?;

    crate::db::channels::find_by_id(&state.db, channel_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut messages = crate::db::messages::list_by_channel(
        &state.db,
        channel_id,
        query.limit.unwrap_or(50).min(100),
        query.before,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    crate::db::messages::enrich_with_attachments(&state.db, &mut messages)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(messages))
}

const MAX_FILE_SIZE: usize = 25 * 1024 * 1024; // 25 MB
const MAX_FILES_PER_MESSAGE: usize = 10;

const ALLOWED_EXTENSIONS: &[&str] = &[
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

fn sanitize_filename(name: &str) -> String {
    let name: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
        .collect();
    // Limit length to 255
    if name.len() > 255 {
        name[..255].to_string()
    } else if name.is_empty() {
        "file".to_string()
    } else {
        name
    }
}

fn is_allowed_extension(filename: &str) -> bool {
    let ext = std::path::Path::new(filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    ALLOWED_EXTENSIONS.contains(&ext.as_str())
}

/// POST /api/channels/:id/messages (JSON — text only)
async fn send_message_json(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
    Json(payload): Json<SendMessagePayload>,
) -> Result<Json<shared::models::Message>, StatusCode> {
    require_channel_permission(&state.db, auth.0, channel_id, permissions::SEND_MESSAGES).await?;

    crate::db::channels::find_by_id(&state.db, channel_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let message = crate::db::messages::create(&state.db, channel_id, auth.0, &payload.content, payload.reply_to_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let _ = state.event_tx.send(shared::events::ServerEvent::MessageCreate(message.clone()));

    Ok(Json(message))
}

/// POST /api/channels/:id/upload (multipart — text + files)
async fn send_message_upload(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Json<shared::models::Message>, StatusCode> {
    require_channel_permission(&state.db, auth.0, channel_id, permissions::SEND_MESSAGES).await?;

    crate::db::channels::find_by_id(&state.db, channel_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let mut content = String::new();
    let mut reply_to_id: Option<i64> = None;
    // (sanitized_filename, content_type, data, ext)
    let mut files: Vec<(String, String, Vec<u8>, String)> = Vec::new();

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        if name == "content" {
            content = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        } else if name == "reply_to_id" {
            let val = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            reply_to_id = val.parse().ok();
        } else if name == "file" {
            if files.len() >= MAX_FILES_PER_MESSAGE {
                return Err(StatusCode::BAD_REQUEST);
            }
            let raw_filename = field.file_name().unwrap_or("file").to_string();
            let filename = sanitize_filename(&raw_filename);
            if !is_allowed_extension(&filename) {
                return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
            }
            let content_type = field.content_type().unwrap_or("application/octet-stream").to_string();
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            if data.is_empty() {
                return Err(StatusCode::BAD_REQUEST);
            }
            if data.len() > state.max_file_size {
                return Err(StatusCode::PAYLOAD_TOO_LARGE);
            }
            let ext = std::path::Path::new(&filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
                .to_lowercase();
            files.push((filename, content_type, data.to_vec(), ext));
        }
    }

    if content.is_empty() && files.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create message in DB first (need the ID for S3 keys)
    let mut message = crate::db::messages::create(&state.db, channel_id, auth.0, &content, reply_to_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Upload files and create attachment records
    // On failure: clean up already-uploaded files + delete the message
    if !files.is_empty() {
        let mut uploaded_keys: Vec<String> = Vec::new();

        for (filename, content_type, data, ext) in &files {
            let stored_name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
            let key = format!("{}/{}", message.id, stored_name);

            if let Err(e) = crate::storage::upload(&state.storage, &key, data, content_type).await {
                tracing::error!("File upload failed: {e}");
                for k in &uploaded_keys {
                    let _ = crate::storage::delete_file(&state.storage, k).await;
                }
                let _ = crate::db::messages::delete(&state.db, message.id).await;
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
            uploaded_keys.push(key);

            let att = crate::db::attachments::create(
                &state.db,
                message.id,
                filename,
                &stored_name,
                content_type,
                data.len() as i64,
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            message.attachments.push(att);
        }
    }

    let _ = state.event_tx.send(shared::events::ServerEvent::MessageCreate(message.clone()));

    Ok(Json(message))
}

// ── Channel Groups ──

/// GET /api/channels/groups
async fn list_groups(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
) -> Result<Json<Vec<shared::models::ChannelGroup>>, StatusCode> {
    let groups = crate::db::channel_groups::list_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(groups))
}

#[derive(Deserialize)]
pub struct CreateGroupPayload {
    name: String,
}

/// POST /api/channels/groups
async fn create_group(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateGroupPayload>,
) -> Result<Json<shared::models::ChannelGroup>, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    let id = crate::db::channel_groups::create(&state.db, &payload.name, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(shared::models::ChannelGroup {
        id,
        name: payload.name,
        position: 0,
    }))
}

#[derive(Deserialize)]
pub struct UpdateGroupPayload {
    name: String,
}

/// PATCH /api/channels/groups/:id
async fn update_group(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateGroupPayload>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    crate::db::channel_groups::update(&state.db, id, &payload.name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/channels/groups/:id
async fn delete_group(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    crate::db::channel_groups::delete(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ReorderPayload {
    ids: Vec<i64>,
}

/// POST /api/channels/reorder
async fn reorder_channels(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<ReorderPayload>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    crate::db::channels::reorder(&state.db, &payload.ids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/channels/groups/reorder
async fn reorder_groups(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<ReorderPayload>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    crate::db::channel_groups::reorder(&state.db, &payload.ids)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct MoveChannelPayload {
    group_id: Option<i64>,
}

/// PATCH /api/channels/:id/group
async fn move_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<MoveChannelPayload>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    crate::db::channels::update_group(&state.db, id, payload.group_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Channel Permission Overwrites ──

/// GET /api/channels/:id/overwrites
async fn list_overwrites(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
) -> Result<Json<Vec<shared::models::ChannelOverwrite>>, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    let overwrites = crate::db::roles::list_channel_overwrites(&state.db, channel_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(overwrites))
}

#[derive(Deserialize)]
pub struct SetOverwritePayload {
    role_id: i64,
    allow: i64,
    deny: i64,
}

/// PUT /api/channels/:id/overwrites
async fn set_overwrite(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
    Json(payload): Json<SetOverwritePayload>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    crate::db::roles::set_channel_overwrite(
        &state.db,
        channel_id,
        payload.role_id,
        payload.allow,
        payload.deny,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct DeleteOverwritePayload {
    role_id: i64,
}

/// DELETE /api/channels/:id/overwrites
async fn delete_overwrite(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
    Json(payload): Json<DeleteOverwritePayload>,
) -> Result<StatusCode, StatusCode> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    crate::db::roles::delete_channel_overwrite(&state.db, channel_id, payload.role_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_channels).post(create_channel))
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/:id", axum::routing::patch(update_group).delete(delete_group))
        .route("/groups/reorder", axum::routing::post(reorder_groups))
        .route("/reorder", axum::routing::post(reorder_channels))
        .route("/:id", get(|| async { "channel" }).patch(update_channel).delete(delete_channel))
        .route("/:id/group", axum::routing::patch(move_channel))
        .route("/:id/messages", get(list_messages).post(send_message_json))
        .route("/:id/upload", axum::routing::post(send_message_upload)
            .layer(DefaultBodyLimit::max(MAX_FILE_SIZE * MAX_FILES_PER_MESSAGE + 1024 * 64)))
        .route("/:id/overwrites", get(list_overwrites).put(set_overwrite).delete(delete_overwrite))
}
