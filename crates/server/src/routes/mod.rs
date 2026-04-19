use axum::Router;
use std::sync::Arc;
use crate::state::AppState;

mod audit;
mod auth;
mod channels;
mod dms;
mod invites;
mod livekit;
mod notifications;
pub mod og;
mod roles;
pub mod servers;
pub mod uploads;
pub mod users;
pub mod webhooks;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/audit", audit::router())
        .nest("/channels", channels::router())
        .nest("/users", users::router())
        .nest("/roles", roles::router())
        .nest("/livekit", livekit::router())
        .nest("/server", servers::router())
        .nest("/invites", invites::router())
        .nest("/notifications", notifications::router())
        .nest("/dms", dms::router())
        .nest("/webhooks", webhooks::router())
        .route("/og", axum::routing::post(og::fetch_og))
}
