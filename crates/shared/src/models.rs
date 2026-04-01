use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceUserState {
    pub muted: bool,
    pub deafened: bool,
    pub force_muted: bool,
    pub force_deafened: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub display_name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelGroup {
    pub id: i64,
    pub name: String,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: i64,
    pub name: String,
    pub kind: ChannelKind,
    pub position: i64,
    pub group_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelKind {
    Text,
    Voice,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: i64,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub channel_id: i64,
    pub author_id: i64,
    pub content: String,
    pub created_at: String,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: i64,
    pub name: String,
    pub permissions: i64,
    pub color: Option<String>,
    pub position: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelOverwrite {
    pub channel_id: i64,
    pub role_id: i64,
    pub allow: i64,
    pub deny: i64,
}
