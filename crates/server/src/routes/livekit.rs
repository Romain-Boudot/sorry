use axum::Router;
use std::sync::Arc;
use crate::state::AppState;

// POST /api/livekit/token  → génère un token pour rejoindre une room

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
    // TODO
}
