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

fn extract_client_ip(headers: &HeaderMap) -> std::net::IpAddr {
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
) -> Result<Json<LoginResponse>, StatusCode> {
    // Rate limiting: 5 attempts per minute per IP
    let client_ip = extract_client_ip(&headers);
    if !state.check_rate_limit(client_ip) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

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
            let invite = crate::db::invites::use_invite(&state.db, invite_code)
                .await
                .map_err(|_| StatusCode::FORBIDDEN)?;

            // Guest invites cannot be used for normal registration
            if invite.guest {
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

            // Assign role from invite if present
            if let Some(role_id) = invite.role_id {
                let _ = crate::db::roles::assign_to_user(&state.db, id, role_id).await;
            }

            crate::db::users::UserRow {
                id: Some(id),
                username: payload.username.clone(),
                display_name: payload.username.clone(),
                password_hash: hash,
                avatar_url: None,
                banned_at: None,
                guest: 0,
            }
        }
    };

    let user_id = user_row.id.unwrap_or(0);

    // Check TOTP
    let totp_enabled = crate::db::totp::is_enabled(&state.db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if totp_enabled {
        match &payload.totp_code {
            None => {
                // Password OK but TOTP required — tell the client
                return Ok(Json(LoginResponse {
                    token: None,
                    user: None,
                    totp_required: true,
                }));
            }
            Some(code) => {
                // Verify TOTP code
                let (secret, _) = crate::db::totp::get_secret(&state.db, user_id)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

                let totp = make_totp(&secret, &user_row.username)
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                if !totp.check_current(code).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
        }
    }

    let token = crate::auth::create_token(user_id, &state.jwt_secret, state.jwt_ttl_secs)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(LoginResponse {
        token: Some(token),
        user: Some(crate::db::users::to_model(&user_row)),
        totp_required: false,
    }))
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
) -> Result<Json<LoginResponse>, StatusCode> {
    let client_ip = extract_client_ip(&headers);
    if !state.check_rate_limit(client_ip) {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let display_name = payload.display_name.trim();
    if display_name.is_empty() || display_name.len() > 32 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Validate and consume invite — must be a guest invite
    let invite = crate::db::invites::use_invite(&state.db, &payload.invite_code)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;

    if !invite.guest {
        return Err(StatusCode::FORBIDDEN);
    }

    // Create guest user
    let user_id = crate::db::users::create_guest(&state.db, display_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Assign the role from the invite
    if let Some(role_id) = invite.role_id {
        let _ = crate::db::roles::assign_to_user(&state.db, user_id, role_id).await;
    }

    let token = crate::auth::create_token(user_id, &state.jwt_secret, state.jwt_ttl_secs)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = crate::db::users::find_by_id(&state.db, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

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
) -> Result<Json<TotpSetupResponse>, StatusCode> {
    let secret = Secret::generate_secret();
    let secret_b32 = secret.to_encoded().to_string();

    // Fetch username for the TOTP label
    let user = crate::db::users::find_by_id_internal(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let totp = make_totp(&secret_b32, &user.username)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Save (unverified) secret
    crate::db::totp::save_secret(&state.db, auth.0, &secret_b32)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
) -> Result<StatusCode, StatusCode> {
    let (secret, verified) = crate::db::totp::get_secret(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if verified {
        return Err(StatusCode::CONFLICT); // already enabled
    }

    let user = crate::db::users::find_by_id_internal(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let totp = make_totp(&secret, &user.username)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !totp.check_current(&payload.code).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        return Err(StatusCode::UNAUTHORIZED);
    }

    crate::db::totp::verify_enable(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn totp_disable(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, StatusCode> {
    crate::db::totp::disable(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
pub struct TotpStatusResponse {
    enabled: bool,
}

async fn totp_status(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<TotpStatusResponse>, StatusCode> {
    let enabled = crate::db::totp::is_enabled(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(TotpStatusResponse { enabled }))
}

// ── Refresh ──

async fn refresh(
    auth: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let token = crate::auth::create_token(auth.0, &state.jwt_secret, state.jwt_ttl_secs)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
