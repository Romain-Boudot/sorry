PRAGMA journal_mode = WAL;

CREATE TABLE users (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    username      TEXT    NOT NULL UNIQUE,
    display_name  TEXT    NOT NULL,
    password_hash TEXT    NOT NULL,
    created_at    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE roles (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL UNIQUE,
    permissions INTEGER NOT NULL DEFAULT 0,
    color       TEXT,
    position    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE user_roles (
    user_id INTEGER NOT NULL REFERENCES users(id),
    role_id INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE channels (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    kind       TEXT    NOT NULL CHECK(kind IN ('text', 'voice')),
    position   INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE messages (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_id INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_id  INTEGER NOT NULL REFERENCES users(id),
    content    TEXT    NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE channel_permission_overwrites (
    channel_id INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    role_id    INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    allow      INTEGER NOT NULL DEFAULT 0,
    deny       INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (channel_id, role_id)
);

CREATE INDEX idx_messages_channel ON messages(channel_id, created_at);

-- Rôles par défaut
INSERT INTO roles (name, permissions, color, position) VALUES ('Admin', 1, '#e74c3c', 0);
INSERT INTO roles (name, permissions, color, position) VALUES ('Membre', 9418432, NULL, 1);

-- Channels par défaut
INSERT INTO channels (name, kind, position) VALUES ('general', 'text', 0);
INSERT INTO channels (name, kind, position) VALUES ('General', 'voice', 1);
