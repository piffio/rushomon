-- Migration 0042: MiniJinja redirect templates

ALTER TABLE links ADD COLUMN is_template INTEGER NOT NULL DEFAULT 0
  CHECK (is_template IN (0, 1));

-- Template redirects check moderation rules at request time.
CREATE INDEX IF NOT EXISTS idx_destination_blacklist_match
  ON destination_blacklist(match_type, destination);
