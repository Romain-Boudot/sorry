use sqlx::SqlitePool;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::RwLock;
use std::time::Instant;
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
    pub livekit_internal_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub storage: crate::storage::Storage,
    pub max_file_size: usize,
    pub online_users: RwLock<HashMap<UserId, usize>>,
    pub voice_state: RwLock<HashMap<ChannelId, HashMap<UserId, VoiceUserState>>>,
    pub event_tx: broadcast::Sender<ServerEvent>,
    pub banned_users: RwLock<std::collections::HashSet<UserId>>,
    pub login_attempts: RwLock<HashMap<IpAddr, Vec<Instant>>>,
    pub og_cache: RwLock<HashMap<String, (crate::routes::og::OgData, Instant)>>,
    pub invite_attempts: RwLock<HashMap<IpAddr, Vec<Instant>>>,
}

impl AppState {
    pub fn new(
        db: SqlitePool,
        server_name: String,
        jwt_secret: String,
        jwt_ttl_secs: i64,
        livekit_url: String,
        livekit_internal_url: String,
        livekit_api_key: String,
        livekit_api_secret: String,
        storage: crate::storage::Storage,
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
            livekit_internal_url,
            livekit_api_key,
            livekit_api_secret,
            storage,
            max_file_size,
            online_users: RwLock::new(HashMap::new()),
            voice_state: RwLock::new(HashMap::new()),
            event_tx,
            banned_users: RwLock::new(banned_users),
            login_attempts: RwLock::new(HashMap::new()),
            og_cache: RwLock::new(HashMap::new()),
            invite_attempts: RwLock::new(HashMap::new()),
        }
    }

    /// Check if an IP is rate-limited (max 5 attempts per 60 seconds)
    pub fn check_rate_limit(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(60);
        let max_attempts = 5;

        let mut attempts = self.login_attempts.write().unwrap();
        let entry = attempts.entry(ip).or_default();

        // Remove attempts older than the window
        entry.retain(|t| now.duration_since(*t) < window);

        if entry.len() >= max_attempts {
            return false; // rate limited
        }

        entry.push(now);
        true
    }

    /// Check if an IP is rate-limited for invite operations (max 10 attempts per 60 seconds)
    pub fn check_invite_rate_limit(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(60);
        let max_attempts = 10;

        let mut attempts = self.invite_attempts.write().unwrap();
        let entry = attempts.entry(ip).or_default();

        entry.retain(|t| now.duration_since(*t) < window);

        if entry.len() >= max_attempts {
            return false;
        }

        entry.push(now);
        true
    }
}
