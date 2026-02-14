-- Add tier column to organizations for usage-based limits
-- Tiers: 'free' (with limits) or 'unlimited' (no limits)
-- Default is 'free' for the managed service
ALTER TABLE organizations ADD COLUMN tier TEXT NOT NULL DEFAULT 'free';

-- Add default_user_tier setting (controls what tier new signups get)
INSERT INTO settings (key, value, updated_at) VALUES ('default_user_tier', 'free', 0);
