-- Guest flag on users
ALTER TABLE users ADD COLUMN guest INTEGER NOT NULL DEFAULT 0;

-- Invite can optionally assign a role and be guest-only
ALTER TABLE invites ADD COLUMN role_id INTEGER REFERENCES roles(id) ON DELETE SET NULL;
ALTER TABLE invites ADD COLUMN guest INTEGER NOT NULL DEFAULT 0;
