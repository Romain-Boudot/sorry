-- Clé publique X25519 par utilisateur (base64).
-- fingerprint = hex court dérivé de la clé (affiché dans l'UI pour vérification TOFU).
-- updated_at permet de détecter un re-handshake (rotation de clé).
ALTER TABLE users ADD COLUMN public_key TEXT;
ALTER TABLE users ADD COLUMN key_fingerprint TEXT;
ALTER TABLE users ADD COLUMN key_updated_at INTEGER;

-- Messages privés chiffrés end-to-end.
-- Le serveur ne voit jamais le plaintext : il ne stocke que le ciphertext + nonce.
-- sender_key_fingerprint = fingerprint de la clé publique utilisée par l'émetteur au moment de l'envoi
-- (permet au destinataire de détecter un changement de clé sur un message donné).
CREATE TABLE dm_messages (
    id                       INTEGER PRIMARY KEY AUTOINCREMENT,
    sender_id                INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recipient_id             INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ciphertext               TEXT    NOT NULL,
    nonce                    TEXT    NOT NULL,
    sender_key_fingerprint   TEXT    NOT NULL,
    created_at               DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index pour récupérer rapidement une conversation entre deux users (dans les deux sens).
CREATE INDEX idx_dms_pair ON dm_messages(sender_id, recipient_id, id DESC);
CREATE INDEX idx_dms_pair_rev ON dm_messages(recipient_id, sender_id, id DESC);
