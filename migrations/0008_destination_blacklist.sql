-- Destination blacklist table
-- Stores blocked URLs/domains to prevent creation of links to malicious destinations
-- Supports both exact URL matching and domain/host matching

CREATE TABLE destination_blacklist (
  id TEXT PRIMARY KEY,
  destination TEXT NOT NULL,  -- Full URL or domain (e.g., "https://malware.com" or "malware.com")
  match_type TEXT NOT NULL DEFAULT 'exact',  -- 'exact' or 'domain'
  reason TEXT NOT NULL,
  created_by TEXT NOT NULL,  -- User ID of admin who added this
  created_at INTEGER NOT NULL,
  FOREIGN KEY (created_by) REFERENCES users(id)
);

-- Indexes for efficient lookups
CREATE INDEX idx_blacklist_destination ON destination_blacklist(destination);
CREATE INDEX idx_blacklist_match_type ON destination_blacklist(match_type);
CREATE INDEX idx_blacklist_created_at ON destination_blacklist(created_at DESC);
