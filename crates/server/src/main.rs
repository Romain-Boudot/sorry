mod auth;
mod db;
mod livekit;
mod routes;
mod state;
mod ws;

use axum::{routing::get, Router};
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use state::AppState;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "server=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:./data.db".to_string());

    let server_name = std::env::var("SERVER_NAME").unwrap_or_else(|_| "Sorry Server".to_string());
    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");
    let livekit_url = std::env::var("LIVEKIT_URL").unwrap_or_default();
    let livekit_api_key = std::env::var("LIVEKIT_API_KEY").unwrap_or_default();
    let livekit_api_secret = std::env::var("LIVEKIT_API_SECRET").unwrap_or_default();

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("../../migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    let state = Arc::new(AppState::new(db, server_name, jwt_secret, livekit_url, livekit_api_key, livekit_api_secret));

    let info_state = state.clone();
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/info", get(move || async move {
            axum::Json(serde_json::json!({ "name": info_state.server_name }))
        }))
        .route("/ws", get(ws::handler))
        .nest("/api", routes::router())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = TcpListener::bind(&addr).await.unwrap();
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await.unwrap();
}
