-- Ban: unix timestamp on users
ALTER TABLE users ADD COLUMN banned_at INTEGER;

-- Invites
CREATE TABLE invites (
    code       TEXT PRIMARY KEY,
    created_by INTEGER NOT NULL REFERENCES users(id),
    max_uses   INTEGER,
    uses       INTEGER NOT NULL DEFAULT 0,
    expires_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);
