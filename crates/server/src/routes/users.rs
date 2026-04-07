use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::perms::require_user_hierarchy;
use crate::state::AppState;
use crate::upload;

#[derive(Serialize)]
pub struct MeResponse {
    user: shared::models::User,
    permissions: i64,
    users: Vec<shared::models::User>,
    online_users: Vec<i64>,
    voice_state: HashMap<i64, HashMap<i64, shared::models::VoiceUserState>>,
    roles: Vec<shared::models::Role>,
    user_roles: HashMap<i64, Vec<i64>>,
    max_file_size: usize,
}

/// GET /api/users/me
async fn me(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<MeResponse>, AppError> {
    let user = crate::db::users::find_by_id(&state.db, auth.0)
        .await?
        .ok_or(AppError::NotFound)?;

    let permissions = crate::db::roles::get_user_permissions(&state.db, auth.0).await?;
    let users = crate::db::users::list_all(&state.db).await?;
    let roles = crate::db::roles::list_all(&state.db).await?;
    let user_roles = crate::db::roles::get_all_user_roles(&state.db).await?;

    let online: Vec<i64> = state.online_users.read().unwrap().keys().copied().collect();

    let voice: HashMap<i64, HashMap<i64, shared::models::VoiceUserState>> = state
        .voice_state
        .read()
        .unwrap()
        .iter()
        .map(|(k, v)| (*k, v.iter().map(|(uid, vs)| (*uid, vs.clone())).collect()))
        .collect();

    Ok(Json(MeResponse {
        user,
        permissions,
        users,
        online_users: online,
        voice_state: voice,
        roles,
        user_roles,
        max_file_size: state.max_file_size,
    }))
}

#[derive(Deserialize)]
pub struct UpdateMePayload {
    display_name: String,
}

/// PATCH /api/users/me
async fn update_me(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<UpdateMePayload>,
) -> Result<Json<shared::models::User>, AppError> {
    let trimmed = payload.display_name.trim();
    if trimmed.is_empty() || trimmed.len() > 32 {
        return Err(AppError::BadRequest("Display name must be 1-32 chars".into()));
    }

    crate::db::users::update_display_name(&state.db, auth.0, trimmed).await?;

    let user = crate::db::users::find_by_id(&state.db, auth.0)
        .await?
        .ok_or(AppError::NotFound)?;

    state.broadcast(shared::events::ServerEvent::UserUpdate(user.clone()));
    Ok(Json(user))
}

#[derive(Deserialize)]
pub struct ChangePasswordPayload {
    current_password: String,
    new_password: String,
}

/// POST /api/users/me/password
async fn change_password(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<ChangePasswordPayload>,
) -> Result<StatusCode, AppError> {
    if payload.new_password.is_empty() {
        return Err(AppError::BadRequest("Password cannot be empty".into()));
    }

    let user = crate::db::users::find_by_id_internal(&state.db, auth.0)
        .await?
        .ok_or(AppError::NotFound)?;

    let hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Argon2::default()
        .verify_password(payload.current_password.as_bytes(), &hash)
        .map_err(|_| AppError::Unauthorized)?;

    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(payload.new_password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .to_string();

    crate::db::users::update_password(&state.db, auth.0, &new_hash).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/users/:id/roles
async fn user_roles(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Vec<shared::models::Role>>, AppError> {
    let roles = crate::db::roles::get_user_roles(&state.db, user_id).await?;
    Ok(Json(roles))
}

/// POST /api/users/:id/ban — ban a user
async fn ban_user(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    if auth.0 == user_id {
        return Err(AppError::BadRequest("Cannot ban yourself".into()));
    }
    if user_id == shared::OWNER_USER_ID {
        return Err(AppError::Forbidden);
    }

    crate::perms::require_permission(&state.db, auth.0, shared::permissions::BAN_MEMBERS).await?;
    require_user_hierarchy(&state.db, auth.0, user_id).await?;

    crate::db::users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::NotFound)?;

    crate::db::users::ban(&state.db, user_id).await?;
    state.banned_users.write().unwrap().insert(user_id);
    state.broadcast(shared::events::ServerEvent::UserOffline { user_id });

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/users/banned — list banned users
async fn list_banned(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<shared::models::BannedUser>>, AppError> {
    crate::perms::require_permission(&state.db, auth.0, shared::permissions::BAN_MEMBERS).await?;
    let banned = crate::db::users::list_banned(&state.db).await?;
    Ok(Json(banned))
}

/// DELETE /api/users/:id/ban — unban a user
async fn unban_user(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<StatusCode, AppError> {
    crate::perms::require_permission(&state.db, auth.0, shared::permissions::BAN_MEMBERS).await?;
    crate::db::users::unban(&state.db, user_id).await?;
    state.banned_users.write().unwrap().remove(&user_id);
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/users/me/avatar (multipart — single image file)
async fn upload_avatar(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<shared::models::User>, AppError> {
    let file = upload::parse_single_image(&mut multipart, "avatar", 5 * 1024 * 1024).await?;

    let _ = crate::storage::delete_prefix(&state.storage, &format!("avatars/{}/", auth.0)).await;

    let stored_name = format!("{}.{}", uuid::Uuid::new_v4(), file.ext);
    let key = format!("avatars/{}/{}", auth.0, stored_name);

    crate::storage::upload(&state.storage, &key, &file.data, &file.content_type)
        .await
        .map_err(|e| AppError::Internal(format!("Avatar upload failed: {e}")))?;

    let avatar_url = format!("/avatars/{}/{}", auth.0, stored_name);
    crate::db::users::update_avatar_url(&state.db, auth.0, Some(&avatar_url)).await?;

    let user = crate::db::users::find_by_id(&state.db, auth.0)
        .await?
        .ok_or(AppError::NotFound)?;

    state.broadcast(shared::events::ServerEvent::UserUpdate(user.clone()));
    Ok(Json(user))
}

/// DELETE /api/users/me/avatar
async fn delete_avatar(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<StatusCode, AppError> {
    let _ = crate::storage::delete_prefix(&state.storage, &format!("avatars/{}/", auth.0)).await;
    crate::db::users::update_avatar_url(&state.db, auth.0, None).await?;

    if let Ok(Some(user)) = crate::db::users::find_by_id(&state.db, auth.0).await {
        state.broadcast(shared::events::ServerEvent::UserUpdate(user));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// GET /avatars/:user_id/:filename — public, serves avatar images
pub async fn serve_avatar(
    State(state): State<Arc<AppState>>,
    Path((user_id, filename)): Path<(String, String)>,
) -> Result<axum::response::Response, AppError> {
    use axum::http::header;
    use axum::response::IntoResponse;

    if user_id.contains("..") || filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(AppError::BadRequest("Invalid path".into()));
    }

    let key = format!("avatars/{}/{}", user_id, filename);
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

#[derive(Deserialize)]
pub struct UserMessagesQuery {
    limit: Option<i64>,
    before: Option<i64>,
}

/// GET /api/users/:id/messages?limit=50&before=123
async fn user_messages(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(target_id): Path<i64>,
    Query(query): Query<UserMessagesQuery>,
) -> Result<Json<Vec<shared::models::Message>>, AppError> {
    // Only allow viewing own messages or if caller has MANAGE_MESSAGES
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0).await?;
    if auth.0 != target_id && !shared::permissions::has(perms, shared::permissions::BAN_MEMBERS) {
        return Err(AppError::Forbidden);
    }

    let mut messages = crate::db::messages::list_by_author(
        &state.db,
        target_id,
        query.limit.unwrap_or(50).min(100),
        query.before,
    )
    .await?;

    crate::db::messages::enrich_with_attachments(&state.db, &mut messages).await?;
    crate::db::messages::enrich_with_reactions(&state.db, &mut messages).await?;

    Ok(Json(messages))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/me", get(me).patch(update_me))
        .route("/me/avatar", post(upload_avatar).delete(delete_avatar))
        .route("/me/password", post(change_password))
        .route("/banned", get(list_banned))
        .route("/:id/ban", post(ban_user).delete(unban_user))
        .route("/:id/roles", get(user_roles))
        .route("/:id/messages", get(user_messages))
}
