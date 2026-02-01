-- Rushomon - Initial Database Schema
-- Run with: wrangler d1 migrations apply rushomon

-- Organizations (teams) - multi-tenant support
CREATE TABLE organizations (
  id TEXT PRIMARY KEY,              -- UUID
  name TEXT NOT NULL,               -- Display name
  slug TEXT UNIQUE NOT NULL,        -- URL-safe identifier
  created_at INTEGER NOT NULL,      -- Unix timestamp
  created_by TEXT NOT NULL          -- User ID
);

-- Users - OAuth identity
CREATE TABLE users (
  id TEXT PRIMARY KEY,              -- UUID
  email TEXT UNIQUE NOT NULL,
  name TEXT,
  avatar_url TEXT,
  oauth_provider TEXT NOT NULL,    -- 'github' or 'google'
  oauth_id TEXT NOT NULL,           -- Provider's user ID
  org_id TEXT NOT NULL,             -- Default organization
  role TEXT NOT NULL DEFAULT 'member', -- 'admin' or 'member'
  created_at INTEGER NOT NULL,
  FOREIGN KEY (org_id) REFERENCES organizations(id),
  UNIQUE(oauth_provider, oauth_id)  -- Prevent duplicate OAuth accounts
);

-- Links metadata (actual URL mapping stored in KV)
CREATE TABLE links (
  id TEXT PRIMARY KEY,              -- UUID
  org_id TEXT NOT NULL,
  short_code TEXT NOT NULL,         -- e.g., 'abc123'
  destination_url TEXT NOT NULL,
  title TEXT,                       -- Optional description
  created_by TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER,
  expires_at INTEGER,               -- Optional expiration (unix timestamp)
  is_active INTEGER DEFAULT 1,      -- SQLite doesn't have BOOLEAN, uses INTEGER
  click_count INTEGER DEFAULT 0,    -- Denormalized counter
  UNIQUE(org_id, short_code),
  FOREIGN KEY (org_id) REFERENCES organizations(id),
  FOREIGN KEY (created_by) REFERENCES users(id)
);

-- Index for querying user's links
CREATE INDEX idx_links_org_created ON links(org_id, created_at DESC);
CREATE INDEX idx_links_user ON links(created_by, created_at DESC);

-- Analytics events (click tracking)
CREATE TABLE analytics_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  link_id TEXT NOT NULL,
  org_id TEXT NOT NULL,
  timestamp INTEGER NOT NULL,       -- Unix timestamp
  referrer TEXT,                    -- HTTP Referer header
  user_agent TEXT,
  country TEXT,                     -- From CF-IPCountry header
  city TEXT,                        -- From CF-IPCity header (Enterprise only)
  FOREIGN KEY (link_id) REFERENCES links(id)
);

-- Index for analytics queries
CREATE INDEX idx_analytics_link ON analytics_events(link_id, timestamp DESC);
CREATE INDEX idx_analytics_org ON analytics_events(org_id, timestamp DESC);
