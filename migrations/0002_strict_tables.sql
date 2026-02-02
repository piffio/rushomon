-- Migration to convert tables to STRICT
-- Disable foreign key constraints temporarily
PRAGMA foreign_keys = OFF;

BEGIN TRANSACTION;

-- Create new STRICT links table without foreign keys first
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

COMMIT;

-- Re-enable foreign key constraints
PRAGMA foreign_keys = ON;