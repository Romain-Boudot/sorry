use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: i64,  // user_id
    pub exp: usize,
}

pub fn create_token(user_id: i64, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize
        + 30 * 24 * 3600; // 30 jours

    let claims = Claims { sub: user_id, exp };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
}

/// Extractor pour les routes protégées — récupère le user_id depuis le JWT
/// Usage : `async fn handler(auth: AuthUser) { auth.0 → user_id }`
pub struct AuthUser(pub i64);

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    Arc<AppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = Arc::<AppState>::from_ref(state);

        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let claims = verify_token(token, &app_state.jwt_secret)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser(claims.sub))
    }
}
