-- Add ends_at column to track when subscription actually ends
-- This may differ from current_period_end if admin sets a custom end date

ALTER TABLE subscriptions ADD COLUMN ends_at INTEGER;

-- Create index for efficient queries on ending subscriptions
CREATE INDEX IF NOT EXISTS idx_subscriptions_ends_at 
  ON subscriptions(ends_at)
  WHERE ends_at IS NOT NULL;
