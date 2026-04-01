use axum::Router;
use std::sync::Arc;
use crate::state::AppState;

mod auth;
mod channels;
mod invites;
mod livekit;
mod roles;
pub mod servers;
pub mod uploads;
pub mod users;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/channels", channels::router())
        .nest("/users", users::router())
        .nest("/roles", roles::router())
        .nest("/livekit", livekit::router())
        .nest("/server", servers::router())
        .nest("/invites", invites::router())
}
