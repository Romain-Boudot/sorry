CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    actor_id INTEGER NOT NULL,
    action TEXT NOT NULL,
    target_user_id INTEGER,
    target_channel_id INTEGER,
    target_role_id INTEGER,
    details TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (actor_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_audit_log_actor ON audit_log(actor_id, id DESC);
CREATE INDEX idx_audit_log_created ON audit_log(id DESC);
