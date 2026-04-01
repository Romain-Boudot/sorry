use s3::Bucket;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::RwLock;
use tokio::sync::broadcast;
use shared::events::ServerEvent;
use shared::models::VoiceUserState;

pub type UserId = i64;
pub type ChannelId = i64;

pub struct AppState {
    pub db: SqlitePool,
    pub server_name: String,
    pub jwt_secret: String,
    pub jwt_ttl_secs: i64,
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub bucket: Box<Bucket>,
    pub max_file_size: usize,
    pub online_users: RwLock<HashMap<UserId, usize>>,
    pub voice_state: RwLock<HashMap<ChannelId, HashMap<UserId, VoiceUserState>>>,
    pub event_tx: broadcast::Sender<ServerEvent>,
    /// In-memory set of banned user IDs (only bans within JWT TTL window)
    pub banned_users: RwLock<std::collections::HashSet<UserId>>,
}

impl AppState {
    pub fn new(
        db: SqlitePool,
        server_name: String,
        jwt_secret: String,
        jwt_ttl_secs: i64,
        livekit_url: String,
        livekit_api_key: String,
        livekit_api_secret: String,
        bucket: Box<Bucket>,
        max_file_size: usize,
        banned_users: std::collections::HashSet<UserId>,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            db,
            server_name,
            jwt_secret,
            jwt_ttl_secs,
            livekit_url,
            livekit_api_key,
            livekit_api_secret,
            bucket,
            max_file_size,
            online_users: RwLock::new(HashMap::new()),
            voice_state: RwLock::new(HashMap::new()),
            event_tx,
            banned_users: RwLock::new(banned_users),
        }
    }
}
