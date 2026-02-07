-- Migration: Replace is_active boolean with status field
-- This enables Active/Disabled/Deleted states for better link management
-- Active/Disabled count toward org quota, Deleted frees up the short code

-- Step 1: Backup analytics_events table (has FK to links)
CREATE TABLE analytics_events_backup AS SELECT * FROM analytics_events;

-- Step 2: Drop analytics_events (removes FK constraint)
DROP TABLE analytics_events;

-- Step 3: Add status column to links (TEXT for SQLite compatibility)
ALTER TABLE links ADD COLUMN status TEXT DEFAULT 'active';

-- Step 4: Migrate existing data
-- is_active = 1 -> 'active', is_active = 0 -> 'disabled'
UPDATE links SET status = CASE
    WHEN is_active = 1 THEN 'active'
    ELSE 'disabled'
END;

-- Step 5: Drop old is_active column by recreating the table
CREATE TABLE links_new (
  id TEXT PRIMARY KEY,
  org_id TEXT NOT NULL,
  short_code TEXT NOT NULL,
  destination_url TEXT NOT NULL,
  title TEXT,
  created_by TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER,
  expires_at INTEGER,
  status TEXT NOT NULL DEFAULT 'active', -- 'active' or 'disabled'
  click_count INTEGER DEFAULT 0,
  FOREIGN KEY (org_id) REFERENCES organizations(id),
  FOREIGN KEY (created_by) REFERENCES users(id)
);

-- Step 6: Copy all data
INSERT INTO links_new SELECT
  id, org_id, short_code, destination_url, title,
  created_by, created_at, updated_at, expires_at,
  status, click_count
FROM links;

-- Step 7: Drop old table
DROP TABLE links;

-- Step 8: Rename new table
ALTER TABLE links_new RENAME TO links;

-- Step 9: Create unique index that only applies to non-deleted links
-- This allows short code reuse after deletion
CREATE UNIQUE INDEX idx_links_shortcode_active
  ON links(short_code)
  WHERE status IN ('active', 'disabled');

-- Step 10: Recreate other indexes
CREATE INDEX idx_links_org_created ON links(org_id, created_at DESC);
CREATE INDEX idx_links_user ON links(created_by, created_at DESC);
CREATE INDEX idx_links_org_status ON links(org_id, status);

-- Step 11: Recreate analytics_events table with FK constraint
CREATE TABLE analytics_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  link_id TEXT NOT NULL,
  org_id TEXT NOT NULL,
  timestamp INTEGER NOT NULL,
  referrer TEXT,
  user_agent TEXT,
  country TEXT,
  city TEXT,
  FOREIGN KEY (link_id) REFERENCES links(id)
) STRICT;

-- Step 12: Restore analytics_events data
INSERT INTO analytics_events SELECT * FROM analytics_events_backup;

-- Step 13: Drop backup table
DROP TABLE analytics_events_backup;
