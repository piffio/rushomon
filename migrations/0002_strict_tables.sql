-- Migration to convert tables to STRICT

-- First, drop the foreign key constraint from analytics_events
-- Note: SQLite doesn't support ALTER TABLE DROP CONSTRAINT directly
-- So we need to recreate the table without the constraint
CREATE TABLE analytics_events_new (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  link_id TEXT NOT NULL,
  org_id TEXT NOT NULL,
  timestamp INTEGER NOT NULL,
  referrer TEXT,
  user_agent TEXT,
  country TEXT,
  city TEXT
) STRICT;

-- Copy data from analytics_events to analytics_events_new
INSERT INTO analytics_events_new 
SELECT id, link_id, org_id, timestamp, referrer, user_agent, country, city 
FROM analytics_events;

-- Drop the old analytics_events table
DROP TABLE analytics_events;

-- Rename analytics_events_new to analytics_events
ALTER TABLE analytics_events_new RENAME TO analytics_events;

-- Now create the new STRICT links table
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
  is_active INTEGER DEFAULT 1 CHECK(is_active IN (0, 1)),  -- Boolean as INTEGER with CHECK constraint
  click_count INTEGER DEFAULT 0,
  UNIQUE(org_id, short_code)
) STRICT;

-- Copy data from old table to new table
INSERT INTO links_new 
SELECT id, org_id, short_code, destination_url, title, created_by, created_at, updated_at, expires_at, is_active, click_count 
FROM links;

-- Drop the old table
DROP TABLE links;

-- Rename the new table to the original name
ALTER TABLE links_new RENAME TO links;

-- Finally, recreate the foreign key constraint on analytics_events
-- Since we can't add FK constraints to existing tables in SQLite,
-- we need to recreate analytics_events again with the constraint
CREATE TABLE analytics_events_final (
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

-- Copy data back to analytics_events_final
INSERT INTO analytics_events_final 
SELECT id, link_id, org_id, timestamp, referrer, user_agent, country, city 
FROM analytics_events;

-- Drop the intermediate analytics_events table
DROP TABLE analytics_events;

-- Rename analytics_events_final to analytics_events
ALTER TABLE analytics_events_final RENAME TO analytics_events;