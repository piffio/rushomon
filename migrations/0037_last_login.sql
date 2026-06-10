-- Add last_login_at column to users table
-- This will track the most recent login timestamp for each user

ALTER TABLE users ADD COLUMN last_login_at INTEGER;