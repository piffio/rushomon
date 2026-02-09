-- Settings table for instance-level configuration
-- Key-value store for admin-configurable settings
CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at INTEGER NOT NULL
);

-- Seed default settings
INSERT INTO settings (key, value, updated_at) VALUES ('signups_enabled', 'true', 0);
