//! Public, unauthenticated incoming webhook ingest.
//!
//! - `POST /api/webhooks/:id/:token`        — native + Discord-compatible payload
//! - `POST /api/webhooks/:id/:token/slack`  — Slack `incoming-webhook` payload alias
//!
//! Embeds (Discord) and `attachments`/`blocks` (Slack) are **flattened to markdown**:
//! the MVP supports text + simple embed structure (title/description/fields/footer)
//! but ignores colors, images, and Slack `blocks`.
//!
//! Admin endpoints to manage webhooks live in `routes/channels.rs` (require MANAGE_CHANNELS).

use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

use crate::error::AppError;
use crate::state::AppState;
use shared::events::ServerEvent;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/:id/:token", post(post_native))
        .route("/:id/:token/slack", post(post_slack))
}

/// GET /webhook-avatars/:wh_id/:filename — serve a webhook avatar (public, cached forever).
pub async fn serve_avatar(
    State(state): State<Arc<AppState>>,
    Path((wh_id, filename)): Path<(i64, String)>,
) -> Result<axum::response::Response, AppError> {
    use axum::http::header;
    use axum::response::IntoResponse;

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(AppError::BadRequest("Invalid path".into()));
    }

    let key = format!("webhook-avatars/{}/{}", wh_id, filename);
    let data = crate::storage::download(&state.storage, &key)
        .await
        .map_err(|_| AppError::NotFound)?;

    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    let content_type = crate::upload::content_type_from_ext(&ext);

    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (header::CACHE_CONTROL, "public, max-age=31536000, immutable".to_string()),
        ],
        data,
    ).into_response())
}

/// Discord-compatible payload. All fields optional except that one of
/// `content` / `embeds` must produce non-empty text after flattening.
#[derive(Debug, Deserialize, Default)]
struct NativePayload {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    avatar_url: Option<String>,
    #[serde(default)]
    embeds: Option<Vec<Value>>,
}

/// Slack `incoming-webhook` payload (the legacy `text`/`attachments` shape).
/// `blocks` are accepted but flattened only superficially.
#[derive(Debug, Deserialize, Default)]
struct SlackPayload {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    icon_url: Option<String>,
    #[serde(default)]
    attachments: Option<Vec<Value>>,
    #[serde(default)]
    blocks: Option<Vec<Value>>,
}

async fn post_native(
    State(state): State<Arc<AppState>>,
    Path((id, token)): Path<(i64, String)>,
    Json(payload): Json<NativePayload>,
) -> Result<axum::http::StatusCode, AppError> {
    deliver(&state, id, &token, payload.username, payload.avatar_url, payload.content, payload.embeds, None).await
}

async fn post_slack(
    State(state): State<Arc<AppState>>,
    Path((id, token)): Path<(i64, String)>,
    Json(payload): Json<SlackPayload>,
) -> Result<axum::http::StatusCode, AppError> {
    deliver(
        &state, id, &token,
        payload.username,
        payload.icon_url,
        payload.text,
        None,
        Some(SlackExtras { attachments: payload.attachments, blocks: payload.blocks }),
    ).await
}

struct SlackExtras {
    attachments: Option<Vec<Value>>,
    blocks: Option<Vec<Value>>,
}

const MAX_CONTENT_LEN: usize = 4000;
const MAX_USERNAME_LEN: usize = 80;

async fn deliver(
    state: &AppState,
    webhook_id: i64,
    token: &str,
    username_override: Option<String>,
    avatar_override: Option<String>,
    content: Option<String>,
    discord_embeds: Option<Vec<Value>>,
    slack_extras: Option<SlackExtras>,
) -> Result<axum::http::StatusCode, AppError> {
    // Per-webhook rate limit — protects each channel without coupling unrelated webhooks.
    if !check_rate_limit(state, webhook_id) {
        return Err(AppError::TooManyRequests);
    }

    let webhook = crate::db::webhooks::find_by_id(&state.db, webhook_id)
        .await?
        .ok_or(AppError::NotFound)?;

    // Constant-time-ish token compare (the URL is the secret — leak prevention is the user's job).
    if !constant_time_eq(webhook.token.as_bytes(), token.as_bytes()) {
        return Err(AppError::NotFound);
    }

    // Build the final message body: content + flattened embeds.
    let mut body = content.unwrap_or_default();
    if let Some(embeds) = discord_embeds {
        for embed in embeds {
            let block = flatten_discord_embed(&embed);
            if !block.is_empty() {
                if !body.is_empty() { body.push_str("\n\n"); }
                body.push_str(&block);
            }
        }
    }
    if let Some(extras) = slack_extras {
        if let Some(attachments) = extras.attachments {
            for att in attachments {
                let block = flatten_slack_attachment(&att);
                if !block.is_empty() {
                    if !body.is_empty() { body.push_str("\n\n"); }
                    body.push_str(&block);
                }
            }
        }
        if let Some(blocks) = extras.blocks {
            for blk in blocks {
                let line = flatten_slack_block(&blk);
                if !line.is_empty() {
                    if !body.is_empty() { body.push_str("\n\n"); }
                    body.push_str(&line);
                }
            }
        }
    }

    let body = body.trim().to_string();
    if body.is_empty() {
        return Err(AppError::BadRequest("Empty message (no content / embeds / attachments)".into()));
    }
    if body.len() > MAX_CONTENT_LEN {
        return Err(AppError::PayloadTooLarge);
    }

    let username_override = username_override
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() <= MAX_USERNAME_LEN);
    let avatar_override = avatar_override
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && (s.starts_with("http://") || s.starts_with("https://")));

    let message = crate::db::messages::create_from_webhook(
        &state.db,
        webhook.channel_id,
        webhook.created_by,
        webhook.id,
        &body,
        username_override.as_deref(),
        avatar_override.as_deref(),
    ).await?;

    state.broadcast(ServerEvent::MessageCreate { message, nonce: None });
    Ok(axum::http::StatusCode::NO_CONTENT)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) { diff |= x ^ y; }
    diff == 0
}

/// 30 posts per minute per webhook id. Independent buckets so a noisy CI doesn't
/// starve another channel's webhook.
fn check_rate_limit(state: &AppState, webhook_id: i64) -> bool {
    let now = Instant::now();
    let window = std::time::Duration::from_secs(60);
    let max = 30usize;

    let mut attempts = state.webhook_attempts.write().unwrap();
    let entry = attempts.entry(webhook_id).or_default();
    entry.retain(|t| now.duration_since(*t) < window);
    if entry.len() >= max { return false; }
    entry.push(now);
    true
}

// ── Embed flattening ──

fn flatten_discord_embed(embed: &Value) -> String {
    let mut out = String::new();

    let title = embed.get("title").and_then(|v| v.as_str()).unwrap_or("").trim();
    let url = embed.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
    if !title.is_empty() {
        if !url.is_empty() {
            out.push_str(&format!("**[{}]({})**\n", title, url));
        } else {
            out.push_str(&format!("**{}**\n", title));
        }
    }

    if let Some(author) = embed.get("author").and_then(|v| v.as_object()) {
        let name = author.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
        if !name.is_empty() {
            out.push_str(&format!("_{}_\n", name));
        }
    }

    let description = embed.get("description").and_then(|v| v.as_str()).unwrap_or("").trim();
    if !description.is_empty() {
        out.push_str(description);
        out.push('\n');
    }

    if let Some(fields) = embed.get("fields").and_then(|v| v.as_array()) {
        for field in fields {
            let name = field.get("name").and_then(|v| v.as_str()).unwrap_or("").trim();
            let value = field.get("value").and_then(|v| v.as_str()).unwrap_or("").trim();
            if !name.is_empty() && !value.is_empty() {
                out.push_str(&format!("**{}** : {}\n", name, value));
            }
        }
    }

    if let Some(footer) = embed.get("footer").and_then(|v| v.as_object()) {
        let text = footer.get("text").and_then(|v| v.as_str()).unwrap_or("").trim();
        if !text.is_empty() {
            out.push_str(&format!("— _{}_", text));
        }
    }

    out.trim_end().to_string()
}

fn flatten_slack_attachment(att: &Value) -> String {
    let mut out = String::new();

    let title = att.get("title").and_then(|v| v.as_str()).unwrap_or("").trim();
    let title_link = att.get("title_link").and_then(|v| v.as_str()).unwrap_or("").trim();
    if !title.is_empty() {
        if !title_link.is_empty() {
            out.push_str(&format!("**[{}]({})**\n", title, title_link));
        } else {
            out.push_str(&format!("**{}**\n", title));
        }
    }

    let pretext = att.get("pretext").and_then(|v| v.as_str()).unwrap_or("").trim();
    if !pretext.is_empty() {
        out.push_str(pretext);
        out.push('\n');
    }

    let text = att.get("text").and_then(|v| v.as_str()).unwrap_or("").trim();
    if !text.is_empty() {
        out.push_str(text);
        out.push('\n');
    }

    if let Some(fields) = att.get("fields").and_then(|v| v.as_array()) {
        for field in fields {
            let title = field.get("title").and_then(|v| v.as_str()).unwrap_or("").trim();
            let value = field.get("value").and_then(|v| v.as_str()).unwrap_or("").trim();
            if !title.is_empty() && !value.is_empty() {
                out.push_str(&format!("**{}** : {}\n", title, value));
            }
        }
    }

    let footer = att.get("footer").and_then(|v| v.as_str()).unwrap_or("").trim();
    if !footer.is_empty() {
        out.push_str(&format!("— _{}_", footer));
    }

    out.trim_end().to_string()
}

/// Slack `blocks` are a richer schema; we only extract `section.text.text` lines.
fn flatten_slack_block(blk: &Value) -> String {
    let kind = blk.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match kind {
        "section" => {
            let text = blk.get("text").and_then(|v| v.get("text")).and_then(|v| v.as_str()).unwrap_or("").trim();
            text.to_string()
        }
        "header" => {
            let text = blk.get("text").and_then(|v| v.get("text")).and_then(|v| v.as_str()).unwrap_or("").trim();
            if text.is_empty() { String::new() } else { format!("**{}**", text) }
        }
        "divider" => "---".to_string(),
        _ => String::new(),
    }
}
