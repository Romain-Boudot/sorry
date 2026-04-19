use sqlx::SqlitePool;
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};
use shared::events::{SequencedEvent, ServerEvent};
use shared::models::VoiceUserState;

pub type UserId = i64;
pub type ChannelId = i64;

pub struct AppState {
    pub db: SqlitePool,
    pub server_name: String,
    pub jwt_secret: String,
    pub jwt_ttl_secs: i64,
    pub started_at: Instant,
    /// Short prefix for LiveKit room names, unique per instance.
    pub room_prefix: String,
    pub livekit_url: String,
    pub livekit_internal_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub storage: crate::storage::Storage,
    pub max_file_size: usize,
    pub online_users: RwLock<HashMap<UserId, usize>>,
    pub voice_state: RwLock<HashMap<ChannelId, HashMap<UserId, VoiceUserState>>>,
    pub event_tx: broadcast::Sender<SequencedEvent>,
    pub seq_counter: AtomicU64,
    /// Livraison ciblée des events DM (par user_id). Chaque WS connectée s'enregistre à
    /// l'arrivée et se retire à la déconnexion. Permet d'envoyer un DmCreate / DmUpdate / DmDelete
    /// / DmReaction uniquement au sender + recipient, sans passer par le broadcast global
    /// (qui fuiterait le métadonnée "qui parle à qui" à tous les clients connectés).
    pub direct_senders: RwLock<HashMap<UserId, Vec<mpsc::UnboundedSender<ServerEvent>>>>,
    pub banned_users: RwLock<std::collections::HashSet<UserId>>,
    pub login_attempts: RwLock<HashMap<IpAddr, Vec<Instant>>>,
    pub og_cache: RwLock<HashMap<String, (crate::routes::og::OgData, Instant)>>,
    pub invite_attempts: RwLock<HashMap<IpAddr, Vec<Instant>>>,
    /// Per-webhook rate limiting (independent buckets per webhook id).
    pub webhook_attempts: RwLock<HashMap<i64, Vec<Instant>>>,
    pub server_logs: crate::log_buffer::LogBuffer,
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
        server_logs: crate::log_buffer::LogBuffer,
    ) -> Self {
        // Derive a short unique prefix from jwt_secret
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(jwt_secret.as_bytes());
        let room_prefix = hex::encode(&hash[..4]);

        let (event_tx, _) = broadcast::channel(1024);
        Self {
            db,
            server_name,
            jwt_secret,
            room_prefix,
            jwt_ttl_secs,
            livekit_url,
            livekit_internal_url,
            livekit_api_key,
            livekit_api_secret,
            storage,
            max_file_size,
            started_at: Instant::now(),
            online_users: RwLock::new(HashMap::new()),
            voice_state: RwLock::new(HashMap::new()),
            event_tx,
            seq_counter: AtomicU64::new(0),
            direct_senders: RwLock::new(HashMap::new()),
            banned_users: RwLock::new(banned_users),
            login_attempts: RwLock::new(HashMap::new()),
            og_cache: RwLock::new(HashMap::new()),
            invite_attempts: RwLock::new(HashMap::new()),
            webhook_attempts: RwLock::new(HashMap::new()),
            server_logs,
        }
    }

    /// LiveKit room name, unique per instance: `{prefix}-voice-{channel_id}`
    pub fn room_name(&self, channel_id: i64) -> String {
        format!("{}-voice-{}", self.room_prefix, channel_id)
    }

    /// Broadcast un event avec un numéro de séquence global auto-incrémenté.
    pub fn broadcast(&self, event: ServerEvent) -> u64 {
        let seq = self.seq_counter.fetch_add(1, Ordering::SeqCst) + 1;
        let sequenced = SequencedEvent { seq, event };
        let _ = self.event_tx.send(sequenced);
        seq
    }

    /// Retourne le numéro de séquence courant (dernier event émis).
    pub fn current_seq(&self) -> u64 {
        self.seq_counter.load(Ordering::SeqCst)
    }

    /// Enregistre un sender pour recevoir les events ciblés d'un user. Appelé à la connexion WS.
    pub fn register_direct_sender(&self, user_id: UserId, sender: mpsc::UnboundedSender<ServerEvent>) {
        self.direct_senders
            .write()
            .unwrap()
            .entry(user_id)
            .or_default()
            .push(sender);
    }

    /// Retire un sender (identifié par pointeur de sameness via `same_channel`) — appelé à la déconnexion WS.
    pub fn unregister_direct_sender(&self, user_id: UserId, sender: &mpsc::UnboundedSender<ServerEvent>) {
        let mut map = self.direct_senders.write().unwrap();
        if let Some(list) = map.get_mut(&user_id) {
            list.retain(|s| !s.same_channel(sender));
            if list.is_empty() {
                map.remove(&user_id);
            }
        }
    }

    /// Envoie un event uniquement aux users listés (via leurs WS enregistrés). Duplicats dédoublonnés.
    /// Utilisé pour les DMs — évite la fuite métadonnée qu'aurait un broadcast global.
    pub fn deliver_direct(&self, user_ids: &[UserId], event: ServerEvent) {
        let map = self.direct_senders.read().unwrap();
        let mut seen: std::collections::HashSet<UserId> = std::collections::HashSet::new();
        for &uid in user_ids {
            if !seen.insert(uid) { continue; }
            if let Some(list) = map.get(&uid) {
                for s in list {
                    let _ = s.send(event.clone());
                }
            }
        }
    }

    /// Generic rate limiter: returns `true` if the request is allowed, `false` if rate-limited.
    fn rate_limit(
        store: &RwLock<HashMap<IpAddr, Vec<Instant>>>,
        ip: IpAddr,
        max_attempts: usize,
        window_secs: u64,
    ) -> bool {
        let now = Instant::now();
        let window = std::time::Duration::from_secs(window_secs);

        let mut attempts = store.write().unwrap();
        let entry = attempts.entry(ip).or_default();

        entry.retain(|t| now.duration_since(*t) < window);

        if entry.len() >= max_attempts {
            return false;
        }

        entry.push(now);
        true
    }

    /// Check if an IP is rate-limited for login (max 5 attempts per 60s)
    pub fn check_rate_limit(&self, ip: IpAddr) -> bool {
        Self::rate_limit(&self.login_attempts, ip, 5, 60)
    }

    /// Check if an IP is rate-limited for invite checks (max 10 attempts per 60s)
    pub fn check_invite_rate_limit(&self, ip: IpAddr) -> bool {
        Self::rate_limit(&self.invite_attempts, ip, 10, 60)
    }
}
