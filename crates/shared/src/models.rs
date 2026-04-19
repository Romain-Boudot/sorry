use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VoiceUserState {
    pub muted: bool,
    pub deafened: bool,
    pub force_muted: bool,
    pub force_deafened: bool,
    #[serde(default)]
    pub screen_sharing: bool,
    #[serde(default)]
    pub camera_on: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub display_name: String,
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub guest: bool,
    /// Public key (base64 X25519) used for DM E2EE. None if user hasn't generated a keypair yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    /// Short hex fingerprint of `public_key` — TOFU display value, also bumps on rotation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BannedUser {
    pub id: i64,
    pub display_name: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub banned_at: i64,
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
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub user_limit: Option<i64>,
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
pub struct ChannelAttachment {
    pub id: i64,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub url: String,
    pub author_id: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplyPreview {
    pub id: i64,
    pub author_id: i64,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mention {
    pub kind: MentionKind,
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MentionKind {
    User,
    Role,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub emoji: String,
    pub count: i64,
    pub user_ids: Vec<i64>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<ReplyPreview>,
    #[serde(default)]
    pub mentions: Vec<Mention>,
    #[serde(default)]
    pub reactions: Vec<Reaction>,
    #[serde(default)]
    pub pinned: bool,
    /// If set, the message was authored by a webhook (not a real user).
    /// `author_id` still references the user who created the webhook (for audit).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_id: Option<i64>,
    /// Per-message display name override (Discord-style). Falls back to the webhook's `name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_username: Option<String>,
    /// Per-message avatar override. Falls back to the webhook's `avatar_url`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webhook_avatar_url: Option<String>,
}

/// Public-safe webhook descriptor — exposed to all clients via the snapshot
/// so the UI can render webhook-authored messages with the correct identity.
/// Never includes the token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookInfo {
    pub id: i64,
    pub channel_id: i64,
    pub name: String,
    pub avatar_url: Option<String>,
}

/// Full webhook record — only returned via admin endpoints (includes the token).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: i64,
    pub channel_id: i64,
    pub token: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub created_by: i64,
    pub created_at: String,
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
pub struct AuditLog {
    pub id: i64,
    pub actor_id: i64,
    pub action: String,
    pub target_user_id: Option<i64>,
    pub target_channel_id: Option<i64>,
    pub target_role_id: Option<i64>,
    pub details: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerLogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// E2EE direct message between two users. Server only stores ciphertext + nonce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmMessage {
    pub id: i64,
    pub sender_id: i64,
    pub recipient_id: i64,
    pub ciphertext: String,
    pub nonce: String,
    /// Fingerprint of the sender's public key at send time. Lets the recipient detect a key rotation.
    pub sender_key_fingerprint: String,
    pub created_at: String,
    /// ID du DM auquel celui-ci répond (ou None). Le client décrypte localement l'original.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<i64>,
    /// Si ce DM a été édité — l'UI affiche un indicateur "(modifié)".
    #[serde(default)]
    pub edited: bool,
    /// Réactions non chiffrées (juste emoji + user_ids). Assumé : moins sensible que le contenu.
    #[serde(default)]
    pub reactions: Vec<Reaction>,
}

/// Compact summary of a DM conversation — used to render the "private messages" list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmConversation {
    pub user_id: i64,
    pub last_message_id: i64,
    pub last_message_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelOverwrite {
    pub channel_id: i64,
    pub role_id: i64,
    pub allow: i64,
    pub deny: i64,
}
