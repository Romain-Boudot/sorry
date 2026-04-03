use serde::{Deserialize, Serialize};
use crate::models::{Message, Role, User, VoiceUserState};

/// Events envoyés du serveur → client via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerEvent {
    MessageCreate(Message),
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
}

/// Events envoyés du client → serveur via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientEvent {
    SendMessage { channel_id: i64, content: String },
    EditMessage { message_id: i64, content: String },
    DeleteMessage { message_id: i64 },
    JoinVoice { channel_id: i64 },
    LeaveVoice { channel_id: i64 },
    UpdateVoiceState { muted: bool, deafened: bool },
    ForceMute { user_id: i64, muted: bool },
    ForceDeafen { user_id: i64, deafened: bool },
    KickVoice { user_id: i64 },
}
