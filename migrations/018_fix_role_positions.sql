-- Reserve position 0 for Owner role, move Everyone to the bottom.
-- Custom roles must be between Owner (top) and Everyone (bottom).

-- Move Everyone role to a high position (bottom of hierarchy)
UPDATE roles SET position = 1000000 WHERE id = 2;

-- Shift any custom role at position 0 up by 1 (collision with Owner role)
UPDATE roles SET position = position + 1 WHERE id > 2 AND position = 0;
