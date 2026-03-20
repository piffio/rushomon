-- Add processed_webhooks table for webhook idempotency
-- Supports multiple providers (Polar, Stripe, etc.) via provider column
-- Webhooks are automatically cleaned up after 30 days

CREATE TABLE IF NOT EXISTS processed_webhooks (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL CHECK (provider IN ('polar')),
    webhook_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    processed_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

-- Composite index for fast lookups by provider + webhook_id
CREATE INDEX IF NOT EXISTS idx_webhooks_provider_id 
  ON processed_webhooks(provider, webhook_id);

-- Index for efficient cleanup of expired entries
CREATE INDEX IF NOT EXISTS idx_webhooks_expires_at 
  ON processed_webhooks(expires_at);
