-- Migration 0033: User notification preferences
--
-- Stores per-user email notification opt-in/out flags.
-- Missing row = opted in by default (handled in the repository layer).
-- New notification types should be added as additional INTEGER columns here.

CREATE TABLE notification_preferences (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    email_monthly_stats INTEGER NOT NULL DEFAULT 1,
    updated_at INTEGER NOT NULL
) STRICT;
