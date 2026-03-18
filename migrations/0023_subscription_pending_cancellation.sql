-- Add pending_cancellation flag to track subscriptions that cancel at period end
-- When cancel_at_period_end is true, the subscription remains active until current_period_end
-- This flag allows us to defer tier downgrade until the subscription actually expires

ALTER TABLE subscriptions ADD COLUMN pending_cancellation INTEGER DEFAULT 0;

-- Create index for efficient cron queries
CREATE INDEX IF NOT EXISTS idx_subscriptions_pending_cancellation 
  ON subscriptions(pending_cancellation, current_period_end)
  WHERE pending_cancellation = 1;
