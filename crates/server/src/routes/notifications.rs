use axum::{
    extract::State,
    http::StatusCode,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;
use crate::db::notification_prefs::{self, NotificationPref};

/// GET /api/notifications/preferences
async fn get_preferences(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<NotificationPref>>, AppError> {
    let prefs = notification_prefs::get_all(&state.db, auth.0).await?;
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
) -> Result<StatusCode, AppError> {
    if !["channel", "server"].contains(&payload.scope.as_str()) {
        return Err(AppError::BadRequest("Invalid scope".into()));
    }
    if !["all", "mentions", "nothing"].contains(&payload.level.as_str()) {
        return Err(AppError::BadRequest("Invalid level".into()));
    }
    if let Some(ref mute_str) = payload.mute_until {
        chrono::DateTime::parse_from_rfc3339(mute_str)
            .map_err(|_| AppError::BadRequest("Invalid mute_until timestamp".into()))?;
    }
    if payload.scope == "channel" {
        crate::db::channels::find_by_id(&state.db, payload.target_id)
            .await?
            .ok_or(AppError::NotFound)?;
    }

    notification_prefs::upsert(
        &state.db, auth.0, &payload.scope, payload.target_id, &payload.level, payload.mute_until.as_deref(),
    ).await?;

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
) -> Result<StatusCode, AppError> {
    notification_prefs::delete(&state.db, auth.0, &payload.scope, payload.target_id).await?;
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
