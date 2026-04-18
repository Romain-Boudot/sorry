-- DMs : reply, edit, reactions (réactions non chiffrées par design — juste emoji + user_id).
ALTER TABLE dm_messages ADD COLUMN reply_to_id INTEGER REFERENCES dm_messages(id) ON DELETE SET NULL;
ALTER TABLE dm_messages ADD COLUMN edited INTEGER NOT NULL DEFAULT 0;

CREATE TABLE dm_reactions (
    dm_id   INTEGER NOT NULL REFERENCES dm_messages(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji   TEXT    NOT NULL,
    PRIMARY KEY (dm_id, user_id, emoji)
);

CREATE INDEX idx_dm_reactions_msg ON dm_reactions(dm_id);
