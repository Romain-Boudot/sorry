use serde::{Deserialize, Serialize};
use crate::models::{Message, User, VoiceUserState};

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
}
