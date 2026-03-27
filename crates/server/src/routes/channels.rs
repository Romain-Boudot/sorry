use axum::{
    extract::{Path, Query, State},
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

    crate::db::channels::delete(&state.db, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

    let messages = crate::db::messages::list_by_channel(
        &state.db,
        channel_id,
        query.limit.unwrap_or(50).min(100),
        query.before,
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(messages))
}

/// POST /api/channels/:id/messages
async fn send_message(
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

    let message = crate::db::messages::create(&state.db, channel_id, auth.0, &payload.content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_channels).post(create_channel))
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/:id", axum::routing::delete(delete_group))
        .route("/groups/reorder", axum::routing::post(reorder_groups))
        .route("/reorder", axum::routing::post(reorder_channels))
        .route("/:id", get(|| async { "channel" }).delete(delete_channel))
        .route("/:id/group", axum::routing::patch(move_channel))
        .route("/:id/messages", get(list_messages).post(send_message))
}
