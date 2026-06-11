-- Migration: Create dedicated tags table for comprehensive tag management
-- This normalizes the schema: tags table holds metadata, link_tags is the join table

-- Step 1: Create the new tags table for tag metadata
CREATE TABLE tags (
  org_id TEXT NOT NULL,
  tag_name TEXT NOT NULL,
  created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
  color_index INTEGER,
  PRIMARY KEY (org_id, tag_name),
  FOREIGN KEY (org_id) REFERENCES organizations(id) ON DELETE CASCADE
);

-- Index for efficient tag lookups by org
CREATE INDEX idx_tags_org_id ON tags(org_id);

-- Step 2: Migrate existing unique tags from link_tags to the new tags table
INSERT INTO tags (org_id, tag_name, created_at)
SELECT DISTINCT org_id, tag_name, strftime('%s', 'now')
FROM link_tags;

-- Step 3: Add created_at to link_tags for analytics tracking
ALTER TABLE link_tags ADD COLUMN created_at INTEGER DEFAULT (strftime('%s', 'now'));

-- Index for time-based analytics queries
CREATE INDEX idx_link_tags_created_at ON link_tags(org_id, created_at);

-- Step 4: Create a view for efficient tag statistics
-- This view aggregates usage counts, first/last usage dates
CREATE VIEW tag_stats AS
SELECT
  t.org_id,
  t.tag_name,
  t.created_at AS tag_created_at,
  t.color_index,
  COUNT(lt.link_id) AS usage_count,
  MIN(lt.created_at) AS first_used_at,
  MAX(lt.created_at) AS last_used_at
FROM tags t
LEFT JOIN link_tags lt ON t.org_id = lt.org_id AND t.tag_name = lt.tag_name
GROUP BY t.org_id, t.tag_name;

-- Step 5: Create an index to help find similar tags (for analytics)
-- This helps with Levenshtein-style similarity detection
CREATE INDEX idx_tags_name_lower ON tags(tag_name COLLATE NOCASE);
