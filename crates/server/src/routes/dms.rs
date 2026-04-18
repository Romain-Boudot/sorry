use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct DmHistoryQuery {
    limit: Option<i64>,
    before: Option<i64>,
}

/// GET /api/dms — liste les conversations DM (un peer + dernier message).
async fn list_conversations(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Vec<shared::models::DmConversation>>, AppError> {
    let convs = crate::db::dms::list_conversations(&state.db, auth.0).await?;
    Ok(Json(convs))
}

/// GET /api/dms/:user_id?limit=50&before=123 — historique chiffré avec un peer.
/// Le serveur renvoie ciphertext + nonce — il ne déchiffre rien.
async fn list_with(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(other): Path<i64>,
    Query(query): Query<DmHistoryQuery>,
) -> Result<Json<Vec<shared::models::DmMessage>>, AppError> {
    if other == auth.0 {
        return Err(AppError::BadRequest("Cannot DM yourself".into()));
    }
    let messages = crate::db::dms::list_with(
        &state.db,
        auth.0,
        other,
        query.limit.unwrap_or(50).min(100),
        query.before,
    )
    .await?;
    Ok(Json(messages))
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_conversations))
        .route("/:user_id", get(list_with))
}
