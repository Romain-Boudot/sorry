mod auth;
mod db;
mod livekit;
mod routes;
mod state;
mod ws;

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
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

    // ── Admin account bootstrap ──
    ensure_admin(&db).await;

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

    let tls_cert = std::env::var("TLS_CERT").ok();
    let tls_key = std::env::var("TLS_KEY").ok();

    if let (Some(cert_path), Some(key_path)) = (tls_cert, tls_key) {
        let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(&cert_path, &key_path)
            .await
            .expect("Failed to load TLS certificates");
        tracing::info!("Listening on {} (HTTPS)", addr);
        axum_server::bind_rustls(addr.parse().unwrap(), tls_config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        let listener = TcpListener::bind(&addr).await.unwrap();
        tracing::info!("Listening on {} (HTTP)", addr);
        axum::serve(listener, app).await.unwrap();
    }
}

/// SHA-256 un mot de passe, comme le fait le client avant envoi
fn sha256_password(password: &str) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(password.as_bytes());
    hex::encode(hash)
}

async fn ensure_admin(db: &sqlx::SqlitePool) {
    use argon2::password_hash::rand_core::RngCore;

    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password_env = std::env::var("ADMIN_PASSWORD").ok();

    let existing = db::users::find_by_id_internal(db, 1)
        .await
        .expect("Failed to check admin user");

    match existing {
        Some(user) => {
            // Le compte id=1 existe — s'assurer qu'il est bien configuré
            if user.username != admin_username {
                db::users::update_username(db, 1, &admin_username)
                    .await
                    .expect("Failed to update admin username");
                tracing::info!("Admin username updated to '{}'", admin_username);
            }

            // Si ADMIN_PASSWORD est défini en env, reset le mot de passe
            if let Some(ref password) = admin_password_env {
                let pre_hashed = sha256_password(password);
                let salt = SaltString::generate(&mut OsRng);
                let hash = Argon2::default()
                    .hash_password(pre_hashed.as_bytes(), &salt)
                    .expect("Failed to hash admin password")
                    .to_string();
                db::users::update_password(db, 1, &hash)
                    .await
                    .expect("Failed to update admin password");
            }

            // S'assurer que le rôle Admin est toujours assigné
            let _ = db::roles::assign_to_user(db, 1, 1).await;
            let _ = db::roles::assign_to_user(db, 1, 2).await;

            tracing::info!("Admin account '{}' (id=1) verified", admin_username);
        }
        None => {
            // Le compte n'existe pas — le créer
            let password = admin_password_env.unwrap_or_else(|| {
                let mut bytes = [0u8; 16];
                OsRng.fill_bytes(&mut bytes);
                bytes.iter().map(|b| format!("{:02x}", b)).collect()
            });

            let pre_hashed = sha256_password(&password);
            let salt = SaltString::generate(&mut OsRng);
            let hash = Argon2::default()
                .hash_password(pre_hashed.as_bytes(), &salt)
                .expect("Failed to hash admin password")
                .to_string();

            let id = db::users::create(db, &admin_username, &admin_username, &hash)
                .await
                .expect("Failed to create admin user");

            let _ = db::roles::assign_to_user(db, id, 1).await;
            let _ = db::roles::assign_to_user(db, id, 2).await;

            tracing::info!("════════════════════════════════════════════");
            tracing::info!("  Admin account created (id={})", id);
            tracing::info!("  Username: {}", admin_username);
            tracing::info!("  Password: {}", password);
            tracing::info!("  Save this password, it won't be shown again!");
            tracing::info!("════════════════════════════════════════════");
        }
    }
}
