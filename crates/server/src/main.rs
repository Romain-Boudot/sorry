mod auth;
mod db;
mod livekit;
mod routes;
mod state;
mod storage;
mod ws;

use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use axum::{routing::get, Router};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

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
    let livekit_internal_url = std::env::var("LIVEKIT_INTERNAL_URL")
        .unwrap_or_else(|_| "http://livekit:7880".to_string());
    let livekit_api_key = std::env::var("LIVEKIT_API_KEY").unwrap_or_default();
    let livekit_api_secret = std::env::var("LIVEKIT_API_SECRET").unwrap_or_default();
    let max_file_size: usize = std::env::var("MAX_FILE_SIZE_MB")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(25) * 1024 * 1024;

    let s3_endpoint = std::env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string());
    let s3_bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".to_string());
    let s3_access_key = std::env::var("S3_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string());
    let s3_secret_key = std::env::var("S3_SECRET_KEY").unwrap_or_else(|_| "minioadmin".to_string());

    let connect_options: SqliteConnectOptions = database_url
        .parse::<SqliteConnectOptions>()
        .expect("Invalid DATABASE_URL")
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true)
        .create_if_missing(true);

    let db = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await
        .expect("Failed to connect to database");

    sqlx::migrate!("../../migrations")
        .run(&db)
        .await
        .expect("Failed to run migrations");

    // ── Init S3/MinIO ──
    let bucket = storage::create_bucket(&s3_endpoint, &s3_bucket, &s3_access_key, &s3_secret_key).await;
    tracing::info!("S3 storage ready (endpoint={}, bucket={})", s3_endpoint, s3_bucket);

    // ── Admin account bootstrap ──
    ensure_admin(&db).await;

    let jwt_ttl_secs: i64 = 30 * 24 * 3600; // 30 days — same as auth.rs

    // Load recently banned users into memory
    let banned_ids = db::users::load_recent_bans(&db, jwt_ttl_secs)
        .await
        .expect("Failed to load banned users");
    let banned_set: std::collections::HashSet<i64> = banned_ids.into_iter().collect();
    tracing::info!("Loaded {} banned users into memory", banned_set.len());

    let state = Arc::new(AppState::new(db, server_name, jwt_secret, jwt_ttl_secs, livekit_url, livekit_internal_url, livekit_api_key, livekit_api_secret, bucket, max_file_size, banned_set));

    let info_state = state.clone();
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/info", get(move || async move {
            let name = crate::db::servers::get_setting(&info_state.db, "name")
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| info_state.server_name.clone());
            let description = crate::db::servers::get_setting(&info_state.db, "description")
                .await
                .ok()
                .flatten();
            let icon_url = crate::db::servers::get_setting(&info_state.db, "icon_url")
                .await
                .ok()
                .flatten();
            axum::Json(serde_json::json!({ "name": name, "description": description, "icon_url": icon_url, "version": env!("CARGO_PKG_VERSION") }))
        }))
        .route("/version", get(|| async { env!("CARGO_PKG_VERSION") }))
        .route("/ws", get(ws::handler))
        .nest("/api", routes::router())
        .route("/uploads/:msg_id/:filename", get(routes::uploads::serve_upload))
        .route("/avatars/:user_id/:filename", get(routes::users::serve_avatar))
        .route("/server-icon/:filename", get(routes::servers::serve_icon))
        .fallback_service(ServeDir::new("/app/static").append_index_html_on_directories(true))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());

    let tls_cert = std::env::var("TLS_CERT").ok().filter(|s| !s.is_empty());
    let tls_key = std::env::var("TLS_KEY").ok().filter(|s| !s.is_empty());

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
    let admin_password_env = std::env::var("ADMIN_PASSWORD").ok().filter(|s| !s.is_empty());

    let existing = db::users::find_by_id_internal(db, 1)
        .await
        .expect("Failed to check admin user");

    match existing {
        Some(_) => {
            // Le compte id=1 existe — s'assurer que les rôles sont assignés
            let _ = db::roles::assign_to_user(db, 1, 1).await;
            let _ = db::roles::assign_to_user(db, 1, 2).await;
            tracing::info!("Admin account (id=1) verified");
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
