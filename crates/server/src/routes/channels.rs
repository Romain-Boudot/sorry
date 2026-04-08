use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::perms::{require_permission, require_channel_permission};
use crate::state::AppState;
use crate::upload;
use shared::permissions;

#[derive(Deserialize)]
pub struct CreateChannelPayload {
    name: String,
    kind: String,
    group_id: Option<i64>,
    description: Option<String>,
}

#[derive(Deserialize)]
pub struct ListMessagesQuery {
    limit: Option<i64>,
    before: Option<i64>,
}

#[derive(Deserialize)]
pub struct SearchMessagesQuery {
    q: String,
    limit: Option<i64>,
}

#[derive(Deserialize)]
pub struct ListAttachmentsQuery {
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
) -> Result<Json<Vec<shared::models::Channel>>, AppError> {
    require_permission(&state.db, auth.0, permissions::VIEW_CHANNELS).await?;
    let channels = crate::db::channels::list_all(&state.db).await?;
    Ok(Json(channels))
}

/// POST /api/channels
async fn create_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<CreateChannelPayload>,
) -> Result<Json<shared::models::Channel>, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    let kind = match payload.kind.as_str() {
        "text" | "voice" => payload.kind.as_str(),
        _ => return Err(AppError::BadRequest("Invalid channel kind".into())),
    };

    let id = crate::db::channels::create(&state.db, &payload.name, kind, payload.group_id).await?;

    if let Some(ref desc) = payload.description {
        let desc = if desc.trim().is_empty() { None } else { Some(desc.as_str()) };
        crate::db::channels::update_description(&state.db, id, desc).await?;
    }

    let channel = shared::models::Channel {
        id,
        name: payload.name,
        kind: match kind {
            "voice" => shared::models::ChannelKind::Voice,
            _ => shared::models::ChannelKind::Text,
        },
        position: 0,
        group_id: payload.group_id,
        description: payload.description.filter(|d| !d.trim().is_empty()),
    };

    state.broadcast(shared::events::ServerEvent::ChannelCreate(channel.clone()));
    Ok(Json(channel))
}

#[derive(Deserialize)]
pub struct UpdateChannelPayload {
    name: Option<String>,
    description: Option<String>,
}

/// PATCH /api/channels/:id
async fn update_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateChannelPayload>,
) -> Result<Json<shared::models::Channel>, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    let row = crate::db::channels::find_by_id(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;

    if let Some(ref name) = payload.name {
        crate::db::channels::update_name(&state.db, id, name).await?;
    }

    if let Some(ref desc) = payload.description {
        let desc = if desc.trim().is_empty() { None } else { Some(desc.as_str()) };
        crate::db::channels::update_description(&state.db, id, desc).await?;
    }

    let name = payload.name.unwrap_or(row.name);
    let description = if payload.description.is_some() {
        payload.description.filter(|d| !d.trim().is_empty())
    } else {
        row.description
    };
    let channel = shared::models::Channel {
        id,
        name,
        kind: match row.kind.as_str() {
            "voice" => shared::models::ChannelKind::Voice,
            _ => shared::models::ChannelKind::Text,
        },
        position: row.position,
        group_id: row.group_id,
        description,
    };

    state.broadcast(shared::events::ServerEvent::ChannelUpdate(channel.clone()));
    Ok(Json(channel))
}

/// DELETE /api/channels/:id
async fn delete_channel(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;

    crate::db::channels::find_by_id(&state.db, id)
        .await?
        .ok_or(AppError::NotFound)?;

    let message_ids = crate::db::messages::list_ids_by_channel(&state.db, id).await?;
    crate::db::channels::delete(&state.db, id).await?;

    state.broadcast(shared::events::ServerEvent::ChannelDelete { id });

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
) -> Result<Json<Vec<shared::models::Message>>, AppError> {
    require_channel_permission(&state.db, auth.0, channel_id, permissions::READ_MESSAGE_HISTORY)
        .await?;

    crate::db::channels::find_by_id(&state.db, channel_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut messages = crate::db::messages::list_by_channel(
        &state.db,
        channel_id,
        query.limit.unwrap_or(50).min(100),
        query.before,
    )
    .await?;

    crate::db::messages::enrich_with_attachments(&state.db, &mut messages).await?;
    crate::db::messages::enrich_with_reactions(&state.db, &mut messages).await?;

    Ok(Json(messages))
}

const MAX_FILE_SIZE: usize = 25 * 1024 * 1024; // 25 MB

/// POST /api/channels/:id/messages (JSON — text only)
async fn send_message_json(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
    Json(payload): Json<SendMessagePayload>,
) -> Result<Json<shared::models::Message>, AppError> {
    require_channel_permission(&state.db, auth.0, channel_id, permissions::SEND_MESSAGES).await?;

    crate::db::channels::find_by_id(&state.db, channel_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let message = crate::db::messages::create(&state.db, channel_id, auth.0, &payload.content, payload.reply_to_id)
        .await?;

    state.broadcast(shared::events::ServerEvent::MessageCreate(message.clone()));
    Ok(Json(message))
}

/// POST /api/channels/:id/upload (multipart — text + files)
async fn send_message_upload(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
    mut multipart: Multipart,
) -> Result<Json<shared::models::Message>, AppError> {
    require_channel_permission(&state.db, auth.0, channel_id, permissions::SEND_MESSAGES).await?;

    crate::db::channels::find_by_id(&state.db, channel_id)
        .await?
        .ok_or(AppError::NotFound)?;

    let parsed = upload::parse_message_upload(&mut multipart, state.max_file_size).await?;

    // Create message in DB first (need the ID for storage keys)
    let mut message = crate::db::messages::create(
        &state.db, channel_id, auth.0, &parsed.content, parsed.reply_to_id,
    ).await?;

    // Upload files and create attachment records
    if !parsed.files.is_empty() {
        let mut uploaded_keys: Vec<String> = Vec::new();

        for file in &parsed.files {
            let stored_name = format!("{}.{}", uuid::Uuid::new_v4(), file.ext);
            let key = format!("{}/{}", message.id, stored_name);

            if let Err(e) = crate::storage::upload(&state.storage, &key, &file.data, &file.content_type).await {
                tracing::error!("File upload failed: {e}");
                for k in &uploaded_keys {
                    let _ = crate::storage::delete_file(&state.storage, k).await;
                }
                let _ = crate::db::messages::delete(&state.db, message.id).await;
                return Err(AppError::Internal("File upload failed".into()));
            }
            uploaded_keys.push(key);

            let att = crate::db::attachments::create(
                &state.db,
                message.id,
                &file.filename,
                &stored_name,
                &file.content_type,
                file.data.len() as i64,
            )
            .await?;

            message.attachments.push(att);
        }
    }

    state.broadcast(shared::events::ServerEvent::MessageCreate(message.clone()));
    Ok(Json(message))
}

// ── Channel Groups ──

/// GET /api/channels/groups
async fn list_groups(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
) -> Result<Json<Vec<shared::models::ChannelGroup>>, AppError> {
    let groups = crate::db::channel_groups::list_all(&state.db).await?;
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
) -> Result<Json<shared::models::ChannelGroup>, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;
    let id = crate::db::channel_groups::create(&state.db, &payload.name, 0).await?;
    let group = shared::models::ChannelGroup { id, name: payload.name, position: 0 };
    state.broadcast(shared::events::ServerEvent::GroupCreate(group.clone()));
    Ok(Json(group))
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
) -> Result<StatusCode, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;
    crate::db::channel_groups::update(&state.db, id, &payload.name).await?;
    state.broadcast(shared::events::ServerEvent::GroupUpdate(shared::models::ChannelGroup { id, name: payload.name, position: 0 }));
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/channels/groups/:id
async fn delete_group(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<StatusCode, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;
    crate::db::channel_groups::delete(&state.db, id).await?;
    state.broadcast(shared::events::ServerEvent::GroupDelete { id });
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
) -> Result<StatusCode, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;
    crate::db::channels::reorder(&state.db, &payload.ids).await?;
    let channels = crate::db::channels::list_all(&state.db).await?;
    state.broadcast(shared::events::ServerEvent::ChannelListUpdate { channels });
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/channels/groups/reorder
async fn reorder_groups(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<ReorderPayload>,
) -> Result<StatusCode, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;
    crate::db::channel_groups::reorder(&state.db, &payload.ids).await?;
    let groups = crate::db::channel_groups::list_all(&state.db).await?;
    state.broadcast(shared::events::ServerEvent::GroupListUpdate { groups });
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
) -> Result<StatusCode, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;
    crate::db::channels::update_group(&state.db, id, payload.group_id).await?;
    let channels = crate::db::channels::list_all(&state.db).await?;
    state.broadcast(shared::events::ServerEvent::ChannelListUpdate { channels });
    Ok(StatusCode::NO_CONTENT)
}

// ── Channel Permission Overwrites ──

/// GET /api/channels/:id/overwrites
async fn list_overwrites(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
) -> Result<Json<Vec<shared::models::ChannelOverwrite>>, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;
    let overwrites = crate::db::roles::list_channel_overwrites(&state.db, channel_id).await?;
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
) -> Result<StatusCode, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;
    crate::db::roles::set_channel_overwrite(
        &state.db, channel_id, payload.role_id, payload.allow, payload.deny,
    ).await?;
    state.broadcast(shared::events::ServerEvent::OverwriteUpdate(shared::models::ChannelOverwrite {
        channel_id,
        role_id: payload.role_id,
        allow: payload.allow,
        deny: payload.deny,
    }));
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
) -> Result<StatusCode, AppError> {
    require_permission(&state.db, auth.0, permissions::MANAGE_CHANNELS).await?;
    crate::db::roles::delete_channel_overwrite(&state.db, channel_id, payload.role_id).await?;
    state.broadcast(shared::events::ServerEvent::OverwriteDelete { channel_id, role_id: payload.role_id });
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/channels/:id/attachments?limit=50&before=123
async fn list_attachments(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
    Query(query): Query<ListAttachmentsQuery>,
) -> Result<Json<Vec<shared::models::ChannelAttachment>>, AppError> {
    require_channel_permission(&state.db, auth.0, channel_id, permissions::READ_MESSAGE_HISTORY)
        .await?;

    let attachments = crate::db::attachments::list_by_channel(
        &state.db,
        channel_id,
        query.limit.unwrap_or(50).min(100),
        query.before,
    )
    .await?;

    Ok(Json(attachments))
}

/// GET /api/channels/:id/search?q=hello&limit=25
async fn search_messages(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
    Query(query): Query<SearchMessagesQuery>,
) -> Result<Json<Vec<shared::models::Message>>, AppError> {
    require_channel_permission(&state.db, auth.0, channel_id, permissions::READ_MESSAGE_HISTORY)
        .await?;

    let q = query.q.trim();
    if q.is_empty() {
        return Ok(Json(vec![]));
    }

    // Sanitize FTS5 query: wrap each word in quotes, last word gets prefix match (*)
    let words: Vec<&str> = q.split_whitespace().collect();
    let fts_query: String = words
        .iter()
        .enumerate()
        .map(|(i, w)| {
            let clean = w.replace('"', "");
            if i == words.len() - 1 {
                format!("\"{}\"*", clean) // prefix match on last word for live search
            } else {
                format!("\"{}\"", clean)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let mut messages = crate::db::messages::search(
        &state.db,
        channel_id,
        &fts_query,
        query.limit.unwrap_or(25).min(50),
    )
    .await?;

    crate::db::messages::enrich_with_attachments(&state.db, &mut messages).await?;
    crate::db::messages::enrich_with_reactions(&state.db, &mut messages).await?;

    Ok(Json(messages))
}

/// POST /api/channels/:id/messages/:msg_id/pin
async fn pin_message(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path((channel_id, msg_id)): Path<(i64, i64)>,
) -> Result<StatusCode, AppError> {
    require_channel_permission(&state.db, auth.0, channel_id, permissions::MANAGE_MESSAGES).await?;

    let row = crate::db::messages::find_by_id(&state.db, msg_id)
        .await?
        .ok_or(AppError::NotFound)?;

    if row.channel_id != channel_id {
        return Err(AppError::NotFound);
    }

    let new_pinned = row.pinned == 0;
    crate::db::messages::set_pinned(&state.db, msg_id, new_pinned).await?;

    let mut updated = crate::db::messages::to_model(
        &crate::db::messages::find_by_id(&state.db, msg_id)
            .await?
            .ok_or(AppError::NotFound)?,
    );
    let _ = crate::db::messages::enrich_with_attachments(&state.db, std::slice::from_mut(&mut updated)).await;
    let _ = crate::db::messages::enrich_with_reactions(&state.db, std::slice::from_mut(&mut updated)).await;
    state.broadcast(shared::events::ServerEvent::MessageUpdate(updated));

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/channels/:id/pins
async fn list_pinned(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(channel_id): Path<i64>,
) -> Result<Json<Vec<shared::models::Message>>, AppError> {
    require_channel_permission(&state.db, auth.0, channel_id, permissions::READ_MESSAGE_HISTORY).await?;

    let mut messages = crate::db::messages::list_pinned(&state.db, channel_id).await?;
    crate::db::messages::enrich_with_attachments(&state.db, &mut messages).await?;
    crate::db::messages::enrich_with_reactions(&state.db, &mut messages).await?;

    Ok(Json(messages))
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
        .route("/:id/search", get(search_messages))
        .route("/:id/attachments", get(list_attachments))
        .route("/:id/upload", axum::routing::post(send_message_upload)
            .layer(DefaultBodyLimit::max(MAX_FILE_SIZE * upload::MAX_FILES_PER_MESSAGE + 1024 * 64)))
        .route("/:id/pins", get(list_pinned))
        .route("/:id/messages/:msg_id/pin", axum::routing::post(pin_message))
        .route("/:id/overwrites", get(list_overwrites).put(set_overwrite).delete(delete_overwrite))
}
