use serde::{Deserialize, Serialize};
use crate::models::{Channel, ChannelGroup, ChannelOverwrite, DmMessage, Message, Role, User, VoiceUserState, WebhookInfo};

/// Events envoyés du serveur → client via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerEvent {
    MessageCreate {
        message: Message,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        nonce: Option<String>,
    },
    MessageUpdate(Message),
    MessageDelete { id: i64 },
    UserJoinedVoice { user: User, channel_id: i64, voice_state: VoiceUserState },
    UserLeftVoice { user_id: i64, channel_id: i64 },
    UserOnline { user: User },
    UserOffline { user_id: i64 },
    VoiceStateUpdate { user_id: i64, channel_id: i64, voice_state: VoiceUserState },
    RoleCreate(Role),
    RoleUpdate(Role),
    RoleDelete { id: i64 },
    UserRoleUpdate { user_id: i64, role_ids: Vec<i64>, permissions: i64 },
    UserUpdate(User),
    ServerUpdate { name: String, description: Option<String>, icon_url: Option<String> },
    ReactionAdded { message_id: i64, channel_id: i64, emoji: String, user_id: i64 },
    ReactionRemoved { message_id: i64, channel_id: i64, emoji: String, user_id: i64 },
    UserBanned { user_id: i64 },
    UserTyping { user_id: i64, channel_id: i64 },
    VoiceMoved { user_id: i64, channel_id: i64, token: String, url: String },
    ChannelCreate(Channel),
    ChannelUpdate(Channel),
    ChannelDelete { id: i64 },
    ChannelListUpdate { channels: Vec<Channel> },
    GroupCreate(ChannelGroup),
    GroupUpdate(ChannelGroup),
    GroupDelete { id: i64 },
    GroupListUpdate { groups: Vec<ChannelGroup> },
    OverwriteUpdate(ChannelOverwrite),
    OverwriteDelete { channel_id: i64, role_id: i64 },
    /// DMs : livrés uniquement au sender et au recipient via `deliver_direct` (pas de broadcast).
    /// Évite la fuite de métadonnées "qui DM qui" à tous les users connectés.
    DmCreate(DmMessage),
    DmUpdate(DmMessage),
    DmDelete { id: i64, sender_id: i64, recipient_id: i64 },
    /// peer_a/peer_b = les deux bouts de la conversation DM (ordre indifférent).
    /// Permet au client de savoir quelle liste de DMs mettre à jour.
    DmReactionAdded { dm_id: i64, peer_a: i64, peer_b: i64, user_id: i64, emoji: String },
    DmReactionRemoved { dm_id: i64, peer_a: i64, peer_b: i64, user_id: i64, emoji: String },
    /// Un utilisateur a publié/rotationné sa clé publique.
    UserKeyUpdate { user_id: i64, public_key: String, fingerprint: String },
    /// Webhook créé/modifié/supprimé. Public (pas de token) — visible par tous pour
    /// que l'UI puisse rendre les messages d'un webhook avec le bon nom/avatar.
    WebhookCreate(WebhookInfo),
    WebhookUpdate(WebhookInfo),
    WebhookDelete { id: i64, channel_id: i64 },
}

/// Wrapper avec numéro de séquence global pour détecter les events manqués
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencedEvent {
    pub seq: u64,
    #[serde(flatten)]
    pub event: ServerEvent,
}

/// Snapshot complet de l'état serveur, envoyé à la connexion/reconnexion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Numéro de séquence courant (les prochains events auront seq > ce numéro)
    pub seq: u64,
    pub user: User,
    pub permissions: i64,
    pub users: Vec<User>,
    pub online_users: Vec<i64>,
    pub channels: Vec<Channel>,
    pub groups: Vec<ChannelGroup>,
    pub roles: Vec<Role>,
    pub user_roles: std::collections::HashMap<i64, Vec<i64>>,
    pub voice_state: std::collections::HashMap<i64, std::collections::HashMap<i64, VoiceUserState>>,
    pub channel_overwrites: Vec<ChannelOverwrite>,
    #[serde(default)]
    pub webhooks: Vec<WebhookInfo>,
    pub server_name: String,
    pub server_description: Option<String>,
    pub server_icon_url: Option<String>,
    pub max_file_size: usize,
}

/// Events envoyés du client → serveur via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientEvent {
    SendMessage {
        channel_id: i64,
        content: String,
        reply_to_id: Option<i64>,
        #[serde(default)]
        nonce: Option<String>,
    },
    EditMessage { message_id: i64, content: String },
    DeleteMessage { message_id: i64 },
    JoinVoice { channel_id: i64 },
    LeaveVoice { channel_id: i64 },
    UpdateVoiceState { muted: bool, deafened: bool, screen_sharing: bool, camera_on: bool },
    ForceMute { user_id: i64, muted: bool },
    ForceDeafen { user_id: i64, deafened: bool },
    KickVoice { user_id: i64 },
    ToggleReaction { message_id: i64, emoji: String },
    Typing { channel_id: i64 },
    MoveVoice { user_id: i64, channel_id: i64 },
    /// Envoyer un DM chiffré (le serveur ne déchiffre rien — il route + persiste).
    SendDm {
        recipient_id: i64,
        ciphertext: String,
        nonce: String,
        sender_key_fingerprint: String,
        #[serde(default)]
        reply_to_id: Option<i64>,
    },
    /// Éditer un DM existant. Le client ré-encrypte le nouveau plaintext et l'envoie.
    EditDm { message_id: i64, ciphertext: String, nonce: String },
    /// Supprimer un DM (seulement l'auteur).
    DeleteDm { message_id: i64 },
    /// Toggle une réaction emoji sur un DM (non chiffré — emoji + user_id stockés en clair).
    ToggleDmReaction { message_id: i64, emoji: String },
    /// Demande de snapshot complet (reconnexion ou gap détecté)
    RequestSnapshot,
}
