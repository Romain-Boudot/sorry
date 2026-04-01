use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Deserialize)]
pub struct LoginPayload {
    username: String,
    password: String,
    invite_code: Option<String>,
}

#[derive(Serialize)]
pub struct LoginResponse {
    token: String,
    user: shared::models::User,
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let existing = crate::db::users::find_by_username(&state.db, &payload.username)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user_row = match existing {
        Some(row) => {
            // Check if banned
            if row.banned_at.is_some() {
                return Err(StatusCode::FORBIDDEN);
            }

            let hash = PasswordHash::new(&row.password_hash)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Argon2::default()
                .verify_password(payload.password.as_bytes(), &hash)
                .map_err(|_| StatusCode::UNAUTHORIZED)?;
            row
        }
        None => {
            // New user: require valid invite code
            let invite_code = payload.invite_code.as_deref().unwrap_or("");
            if invite_code.is_empty() {
                return Err(StatusCode::FORBIDDEN);
            }

            // Validate and consume invite
            crate::db::invites::use_invite(&state.db, invite_code)
                .await
                .map_err(|_| StatusCode::FORBIDDEN)?;

            let salt = SaltString::generate(&mut OsRng);
            let hash = Argon2::default()
                .hash_password(payload.password.as_bytes(), &salt)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .to_string();

            let id =
                crate::db::users::create(&state.db, &payload.username, &payload.username, &hash)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Assign default "Membre" role (id=2)
            let _ = crate::db::roles::assign_to_user(&state.db, id, 2).await;

            crate::db::users::UserRow {
                id: Some(id),
                username: payload.username.clone(),
                display_name: payload.username.clone(),
                password_hash: hash,
                avatar_url: None,
                banned_at: None,
            }
        }
    };

    let token = crate::auth::create_token(user_row.id.unwrap_or(0), &state.jwt_secret)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token,
        user: crate::db::users::to_model(&user_row),
    }))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/login", post(login))
}
