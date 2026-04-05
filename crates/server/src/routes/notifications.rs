use axum::{
    extract::State,
    http::StatusCode,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::state::AppState;
use crate::db::notification_prefs::{self, NotificationPref};

/// GET /api/notifications/preferences
async fn get_preferences(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<NotificationPref>>, StatusCode> {
    let prefs = notification_prefs::get_all(&state.db, auth.0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(prefs))
}

#[derive(Deserialize)]
pub struct SetPreferencePayload {
    scope: String,
    target_id: i64,
    level: String,
    mute_until: Option<String>,
}

/// PUT /api/notifications/preferences
async fn set_preference(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<SetPreferencePayload>,
) -> Result<StatusCode, StatusCode> {
    if !["channel", "server"].contains(&payload.scope.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    if !["all", "mentions", "nothing"].contains(&payload.level.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    // Validate mute_until is a valid ISO-8601 timestamp if provided
    if let Some(ref mute_str) = payload.mute_until {
        chrono::DateTime::parse_from_rfc3339(mute_str)
            .map_err(|_| StatusCode::BAD_REQUEST)?;
    }
    // Validate channel exists for channel scope
    if payload.scope == "channel" {
        crate::db::channels::find_by_id(&state.db, payload.target_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;
    }

    notification_prefs::upsert(
        &state.db,
        auth.0,
        &payload.scope,
        payload.target_id,
        &payload.level,
        payload.mute_until.as_deref(),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct DeletePreferencePayload {
    scope: String,
    target_id: i64,
}

/// DELETE /api/notifications/preferences
async fn delete_preference(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(payload): Json<DeletePreferencePayload>,
) -> Result<StatusCode, StatusCode> {
    notification_prefs::delete(&state.db, auth.0, &payload.scope, payload.target_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route(
        "/preferences",
        axum::routing::get(get_preferences)
            .put(set_preference)
            .delete(delete_preference),
    )
}
