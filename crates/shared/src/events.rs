use serde::{Deserialize, Serialize};
use crate::models::{Channel, ChannelGroup, ChannelOverwrite, Message, Role, User, VoiceUserState};

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
    /// Demande de snapshot complet (reconnexion ou gap détecté)
    RequestSnapshot,
}
