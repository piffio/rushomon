-- Migration: 0039_account_deletion
-- Add columns to support self-service account deletion with a grace period.
-- When a user requests deletion, pending_deletion_at is set to now + 7 days.
-- A daily cron job checks for expired entries and permanently deletes the user.

ALTER TABLE users ADD COLUMN pending_deletion_at INTEGER;
ALTER TABLE users ADD COLUMN pending_deletion_scheduled_at INTEGER;

-- Index for the cron job to efficiently find users due for deletion
CREATE INDEX idx_users_pending_deletion ON users(pending_deletion_at) WHERE pending_deletion_at IS NOT NULL;
