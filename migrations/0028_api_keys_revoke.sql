-- Add status field with check constraint for three-state lifecycle
ALTER TABLE api_keys ADD COLUMN status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'revoked', 'deleted'));

-- Track when and by whom the last state change occurred
ALTER TABLE api_keys ADD COLUMN updated_at INTEGER;
ALTER TABLE api_keys ADD COLUMN updated_by TEXT REFERENCES users(id);

-- Index for efficient status-based filtering
CREATE INDEX IF NOT EXISTS idx_api_keys_status ON api_keys(status);
