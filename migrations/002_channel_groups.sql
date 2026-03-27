CREATE TABLE channel_groups (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    name     TEXT    NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);

ALTER TABLE channels ADD COLUMN group_id INTEGER REFERENCES channel_groups(id) ON DELETE SET NULL;
