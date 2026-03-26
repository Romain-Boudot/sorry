use axum::Router;
use std::sync::Arc;
use crate::state::AppState;

mod auth;
mod channels;
mod livekit;
mod roles;
mod users;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/channels", channels::router())
        .nest("/users", users::router())
        .nest("/roles", roles::router())
        .nest("/livekit", livekit::router())
}
