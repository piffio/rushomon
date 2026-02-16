-- Add user suspension columns
-- Allows admins to suspend users, preventing access and disabling their links

ALTER TABLE users ADD COLUMN suspended_at INTEGER;
ALTER TABLE users ADD COLUMN suspension_reason TEXT;
ALTER TABLE users ADD COLUMN suspended_by TEXT;

-- Index for querying suspended users
CREATE INDEX idx_users_suspended ON users(suspended_at) WHERE suspended_at IS NOT NULL;
