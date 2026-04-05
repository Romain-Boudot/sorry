-- Notification preferences per user per channel or server-wide
-- scope: 'channel' or 'server'
-- target_id: channel_id (for channel scope) or 0 (for server scope)
-- level: 'all' (notify on every message), 'mentions' (only @mentions), 'nothing' (no notifications)
-- mute_until: NULL = permanent, otherwise ISO-8601 timestamp when mute expires
CREATE TABLE IF NOT EXISTS notification_preferences (
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    scope TEXT NOT NULL CHECK(scope IN ('channel', 'server')),
    target_id INTEGER NOT NULL DEFAULT 0,
    level TEXT NOT NULL DEFAULT 'all' CHECK(level IN ('all', 'mentions', 'nothing')),
    mute_until TEXT,
    PRIMARY KEY (user_id, scope, target_id)
);
