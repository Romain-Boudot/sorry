-- Webhooks: incoming-only HTTP endpoints that post messages to a channel.
-- Token = bearer secret in the URL (anyone with the URL can post).
-- name/avatar_url = default display identity (overridable per message via the POST payload).
CREATE TABLE webhooks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_id  INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    token       TEXT    NOT NULL UNIQUE,
    name        TEXT    NOT NULL,
    avatar_url  TEXT,
    created_by  INTEGER NOT NULL REFERENCES users(id),
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_webhooks_channel ON webhooks(channel_id);

-- author_id stays NOT NULL (references the user who created the webhook).
-- webhook_id != NULL signals a webhook-authored message; the client uses webhook
-- name/avatar_url for display, with optional per-message overrides.
ALTER TABLE messages ADD COLUMN webhook_id INTEGER REFERENCES webhooks(id) ON DELETE SET NULL;
ALTER TABLE messages ADD COLUMN webhook_username TEXT;
ALTER TABLE messages ADD COLUMN webhook_avatar_url TEXT;
