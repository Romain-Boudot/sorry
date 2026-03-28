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
    server_password: Option<String>,
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
            let hash = PasswordHash::new(&row.password_hash)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Argon2::default()
                .verify_password(payload.password.as_bytes(), &hash)
                .map_err(|_| StatusCode::UNAUTHORIZED)?;
            row
        }
        None => {
            // Nouvel utilisateur : vérifier le mot de passe serveur
            let server_pwd = std::env::var("SERVER_PASSWORD").unwrap_or_default();
            if payload.server_password.as_deref() != Some(server_pwd.as_str()) {
                return Err(StatusCode::FORBIDDEN);
            }

            let salt = SaltString::generate(&mut OsRng);
            let hash = Argon2::default()
                .hash_password(payload.password.as_bytes(), &salt)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .to_string();

            let id =
                crate::db::users::create(&state.db, &payload.username, &payload.username, &hash)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Assigner le rôle "Membre" (id=2) par défaut
            let _ = crate::db::roles::assign_to_user(&state.db, id, 2).await;

            crate::db::users::UserRow {
                id: Some(id),
                username: payload.username.clone(),
                display_name: payload.username.clone(),
                password_hash: hash,
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
