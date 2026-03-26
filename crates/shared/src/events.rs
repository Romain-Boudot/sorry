use serde::{Deserialize, Serialize};
use crate::models::{Message, User};

/// Events envoyés du serveur → client via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerEvent {
    MessageCreate(Message),
    MessageDelete { id: i64 },
    UserJoinedVoice { user: User, channel_id: i64 },
    UserLeftVoice { user_id: i64, channel_id: i64 },
    UserOnline { user_id: i64 },
    UserOffline { user_id: i64 },
}

/// Events envoyés du client → serveur via WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientEvent {
    SendMessage { channel_id: i64, content: String },
    JoinVoice { channel_id: i64 },
    LeaveVoice { channel_id: i64 },
}
