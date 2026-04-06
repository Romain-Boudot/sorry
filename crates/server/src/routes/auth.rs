use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use totp_rs::{Algorithm, Secret, TOTP};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

// ── Login ──

#[derive(Deserialize)]
pub struct LoginPayload {
    username: String,
    password: String,
    invite_code: Option<String>,
    totp_code: Option<String>,
}

#[derive(Serialize)]
pub struct LoginResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<shared::models::User>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    totp_required: bool,
}

pub fn extract_client_ip(headers: &HeaderMap) -> std::net::IpAddr {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or_else(|| std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST))
}

async fn login(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoginPayload>,
) -> Result<Json<LoginResponse>, AppError> {
    let client_ip = extract_client_ip(&headers);
    if !state.check_rate_limit(client_ip) {
        return Err(AppError::TooManyRequests);
    }

    let user_row = authenticate_or_register(&state, &payload).await?;
    let user_id = user_row.id.unwrap_or(0);

    if let Some(resp) = check_totp(&state, user_id, &user_row.username, &payload.totp_code).await? {
        return Ok(Json(resp));
    }

    let token = crate::auth::create_token(user_id, &state.jwt_secret, state.jwt_ttl_secs)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(LoginResponse {
        token: Some(token),
        user: Some(crate::db::users::to_model(&user_row)),
        totp_required: false,
    }))
}

/// Authenticate an existing user or register a new one via invite.
async fn authenticate_or_register(
    state: &AppState,
    payload: &LoginPayload,
) -> Result<crate::db::users::UserRow, AppError> {
    let existing = crate::db::users::find_by_username(&state.db, &payload.username).await?;

    match existing {
        Some(row) => {
            if row.banned_at.is_some() {
                return Err(AppError::Forbidden);
            }
            let hash = PasswordHash::new(&row.password_hash)
                .map_err(|e| AppError::Internal(e.to_string()))?;
            Argon2::default()
                .verify_password(payload.password.as_bytes(), &hash)
                .map_err(|_| AppError::Unauthorized)?;
            Ok(row)
        }
        None => register_with_invite(state, payload).await,
    }
}

/// Register a new user using an invite code.
async fn register_with_invite(
    state: &AppState,
    payload: &LoginPayload,
) -> Result<crate::db::users::UserRow, AppError> {
    let invite_code = payload.invite_code.as_deref().unwrap_or("");
    if invite_code.is_empty() {
        return Err(AppError::Forbidden);
    }

    let invite = crate::db::invites::use_invite(&state.db, invite_code)
        .await
        .map_err(|_| AppError::Forbidden)?;

    if invite.guest {
        return Err(AppError::Forbidden);
    }

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .to_string();

    let id = crate::db::users::create(&state.db, &payload.username, &payload.username, &hash).await?;

    if let Some(role_id) = invite.role_id {
        let _ = crate::db::roles::assign_to_user(&state.db, id, role_id).await;
    }

    Ok(crate::db::users::UserRow {
        id: Some(id),
        username: payload.username.clone(),
        display_name: payload.username.clone(),
        password_hash: hash,
        avatar_url: None,
        banned_at: None,
        guest: 0,
    })
}

/// Check TOTP if enabled. Returns Some(LoginResponse) if TOTP is required but not provided.
async fn check_totp(
    state: &AppState,
    user_id: i64,
    username: &str,
    totp_code: &Option<String>,
) -> Result<Option<LoginResponse>, AppError> {
    let totp_enabled = crate::db::totp::is_enabled(&state.db, user_id).await?;
    if !totp_enabled {
        return Ok(None);
    }

    match totp_code {
        None => Ok(Some(LoginResponse {
            token: None,
            user: None,
            totp_required: true,
        })),
        Some(code) => {
            let (secret, _) = crate::db::totp::get_secret(&state.db, user_id)
                .await?
                .ok_or(AppError::Internal("TOTP secret missing".into()))?;

            let totp = make_totp(&secret, username)
                .map_err(|e| AppError::Internal(e.to_string()))?;

            if !totp.check_current(code).map_err(|e| AppError::Internal(e.to_string()))? {
                return Err(AppError::Unauthorized);
            }
            Ok(None)
        }
    }
}

// ── Quick (guest) login ──

#[derive(Deserialize)]
pub struct QuickLoginPayload {
    invite_code: String,
    display_name: String,
}

async fn quick_login(
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<QuickLoginPayload>,
) -> Result<Json<LoginResponse>, AppError> {
    let client_ip = extract_client_ip(&headers);
    if !state.check_rate_limit(client_ip) {
        return Err(AppError::TooManyRequests);
    }

    let display_name = payload.display_name.trim();
    if display_name.is_empty() || display_name.len() > 32 {
        return Err(AppError::BadRequest("Display name must be 1-32 chars".into()));
    }

    let invite = crate::db::invites::use_invite(&state.db, &payload.invite_code)
        .await
        .map_err(|_| AppError::Forbidden)?;

    if !invite.guest {
        return Err(AppError::Forbidden);
    }

    let user_id = crate::db::users::create_guest(&state.db, display_name).await?;

    if let Some(role_id) = invite.role_id {
        let _ = crate::db::roles::assign_to_user(&state.db, user_id, role_id).await;
    }

    let token = crate::auth::create_token(user_id, &state.jwt_secret, state.jwt_ttl_secs)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let user = crate::db::users::find_by_id(&state.db, user_id)
        .await?
        .ok_or(AppError::Internal("User just created not found".into()))?;

    Ok(Json(LoginResponse {
        token: Some(token),
        user: Some(user),
        totp_required: false,
    }))
}

// ── TOTP setup ──

#[derive(Serialize)]
pub struct TotpSetupResponse {
    secret: String,
    otpauth_url: String,
}

async fn totp_setup(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<TotpSetupResponse>, AppError> {
    let secret = Secret::generate_secret();
    let secret_b32 = secret.to_encoded().to_string();

    let user = crate::db::users::find_by_id_internal(&state.db, auth.0)
        .await?
        .ok_or(AppError::NotFound)?;

    let totp = make_totp(&secret_b32, &user.username)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    crate::db::totp::save_secret(&state.db, auth.0, &secret_b32).await?;

    Ok(Json(TotpSetupResponse {
        secret: secret_b32,
        otpauth_url: totp.get_url(),
    }))
}

#[derive(Deserialize)]
pub struct TotpVerifyPayload {
    code: String,
}

async fn totp_verify(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<TotpVerifyPayload>,
) -> Result<StatusCode, AppError> {
    let (secret, verified) = crate::db::totp::get_secret(&state.db, auth.0)
        .await?
        .ok_or(AppError::NotFound)?;

    if verified {
        return Err(AppError::Conflict);
    }

    let user = crate::db::users::find_by_id_internal(&state.db, auth.0)
        .await?
        .ok_or(AppError::NotFound)?;

    let totp = make_totp(&secret, &user.username)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    if !totp.check_current(&payload.code).map_err(|e| AppError::Internal(e.to_string()))? {
        return Err(AppError::Unauthorized);
    }

    crate::db::totp::verify_enable(&state.db, auth.0).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn totp_disable(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, AppError> {
    crate::db::totp::disable(&state.db, auth.0).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct TotpStatusResponse {
    enabled: bool,
}

async fn totp_status(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<TotpStatusResponse>, AppError> {
    let enabled = crate::db::totp::is_enabled(&state.db, auth.0).await?;
    Ok(Json(TotpStatusResponse { enabled }))
}

// ── Refresh ──

async fn refresh(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let token = crate::auth::create_token(auth.0, &state.jwt_secret, state.jwt_ttl_secs)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(serde_json::json!({ "token": token })))
}

// ── Helpers ──

fn make_totp(secret_b32: &str, account: &str) -> Result<TOTP, totp_rs::TotpUrlError> {
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(secret_b32.to_string()).to_bytes().unwrap(),
        Some("Sorry".to_string()),
        account.to_string(),
    )
}

// ── Router ──

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/login", post(login))
        .route("/quick", post(quick_login))
        .route("/refresh", post(refresh))
        .route("/totp/setup", post(totp_setup))
        .route("/totp/verify", post(totp_verify))
        .route("/totp/disable", post(totp_disable))
        .route("/totp/status", axum::routing::get(totp_status))
}
