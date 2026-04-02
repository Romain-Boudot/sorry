use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::state::AppState;

const ALLOWED_AVATAR_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "webp"];

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
) -> Result<Json<MeResponse>, StatusCode> {
    let user = crate::db::users::find_by_id(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let permissions = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let users = crate::db::users::list_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let roles = crate::db::roles::list_all(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_roles = crate::db::roles::get_all_user_roles(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
) -> Result<Json<shared::models::User>, StatusCode> {
    let trimmed = payload.display_name.trim();
    if trimmed.is_empty() || trimmed.len() > 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    crate::db::users::update_display_name(&state.db, auth.0, trimmed)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = crate::db::users::find_by_id(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let _ = state.event_tx.send(shared::events::ServerEvent::UserUpdate(user.clone()));

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
) -> Result<StatusCode, StatusCode> {
    if payload.new_password.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Fetch user with password hash
    let user = crate::db::users::find_by_id_internal(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Verify current password (client sends SHA-256 pre-hash)
    let hash = PasswordHash::new(&user.password_hash)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Argon2::default()
        .verify_password(payload.current_password.as_bytes(), &hash)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Hash new password
    let salt = SaltString::generate(&mut OsRng);
    let new_hash = Argon2::default()
        .hash_password(payload.new_password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    crate::db::users::update_password(&state.db, auth.0, &new_hash)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/users/:id/roles
async fn user_roles(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<Json<Vec<shared::models::Role>>, StatusCode> {
    let roles = crate::db::roles::get_user_roles(&state.db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(roles))
}

/// POST /api/users/:id/ban — ban a user
async fn ban_user(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    if auth.0 == user_id {
        return Err(StatusCode::BAD_REQUEST);
    }
    if user_id == 1 {
        return Err(StatusCode::FORBIDDEN);
    }

    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !shared::permissions::has(perms, shared::permissions::BAN_MEMBERS) {
        return Err(StatusCode::FORBIDDEN);
    }

    crate::db::users::find_by_id(&state.db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    crate::db::users::ban(&state.db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Add to in-memory banned set
    state.banned_users.write().unwrap().insert(user_id);

    // Disconnect the user
    let _ = state.event_tx.send(shared::events::ServerEvent::UserOffline { user_id });

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/users/:id/ban — unban a user
async fn unban_user(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(user_id): Path<i64>,
) -> Result<StatusCode, StatusCode> {
    let perms = crate::db::roles::get_user_permissions(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !shared::permissions::has(perms, shared::permissions::BAN_MEMBERS) {
        return Err(StatusCode::FORBIDDEN);
    }

    crate::db::users::unban(&state.db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Remove from in-memory banned set
    state.banned_users.write().unwrap().remove(&user_id);

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/users/me/avatar (multipart — single image file)
async fn upload_avatar(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<shared::models::User>, StatusCode> {
    let mut file_data: Option<(Vec<u8>, String, String)> = None; // (data, content_type, ext)

    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "avatar" {
            let raw_filename = field.file_name().unwrap_or("avatar.png").to_string();
            let ext = std::path::Path::new(&raw_filename)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !ALLOWED_AVATAR_EXTENSIONS.contains(&ext.as_str()) {
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

    // Delete old avatar from S3
    let _ = crate::storage::delete_prefix(&state.storage, &format!("avatars/{}/", auth.0)).await;

    // Upload new avatar
    let stored_name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
    let key = format!("avatars/{}/{}", auth.0, stored_name);

    crate::storage::upload(&state.storage, &key, &data, &content_type)
        .await
        .map_err(|e| {
            tracing::error!("Avatar upload failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let avatar_url = format!("/avatars/{}/{}", auth.0, stored_name);
    crate::db::users::update_avatar_url(&state.db, auth.0, Some(&avatar_url))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = crate::db::users::find_by_id(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let _ = state.event_tx.send(shared::events::ServerEvent::UserUpdate(user.clone()));

    Ok(Json(user))
}

/// DELETE /api/users/me/avatar
async fn delete_avatar(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<StatusCode, StatusCode> {
    let _ = crate::storage::delete_prefix(&state.storage, &format!("avatars/{}/", auth.0)).await;
    crate::db::users::update_avatar_url(&state.db, auth.0, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Ok(Some(user)) = crate::db::users::find_by_id(&state.db, auth.0).await {
        let _ = state.event_tx.send(shared::events::ServerEvent::UserUpdate(user));
    }

    Ok(StatusCode::NO_CONTENT)
}

/// GET /avatars/:user_id/:filename — public, serves avatar images
pub async fn serve_avatar(
    State(state): State<Arc<AppState>>,
    Path((user_id, filename)): Path<(String, String)>,
) -> Result<axum::response::Response, StatusCode> {
    use axum::http::header;
    use axum::response::IntoResponse;

    if user_id.contains("..") || filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let key = format!("avatars/{}/{}", user_id, filename);
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
        .route("/me", get(me).patch(update_me))
        .route("/me/avatar", post(upload_avatar).delete(delete_avatar))
        .route("/me/password", post(change_password))
        .route("/:id/ban", post(ban_user).delete(unban_user))
        .route("/:id/roles", get(user_roles))
}
