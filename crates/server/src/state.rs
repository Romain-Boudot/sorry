use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::sync::RwLock;
use tokio::sync::broadcast;
use shared::events::ServerEvent;

pub type UserId = i64;
pub type ChannelId = i64;

pub struct AppState {
    pub db: SqlitePool,
    pub jwt_secret: String,
    /// Utilisateurs connectés via WebSocket
    pub online_users: RwLock<HashSet<UserId>>,
    /// channel_id → [user_ids] présents dans le vocal
    pub voice_state: RwLock<HashMap<ChannelId, HashSet<UserId>>>,
    /// Broadcast d'events vers tous les WebSocket connectés
    pub event_tx: broadcast::Sender<ServerEvent>,
}

impl AppState {
    pub fn new(db: SqlitePool, jwt_secret: String) -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        Self {
            db,
            jwt_secret,
            online_users: RwLock::new(HashSet::new()),
            voice_state: RwLock::new(HashMap::new()),
            event_tx,
        }
    }
}
