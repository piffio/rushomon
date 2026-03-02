-- Migration 0015: Add subscriptions table for Stripe billing
-- Tracks subscription state per billing account.
-- billing_accounts.tier remains the source of truth for limit enforcement;
-- this table drives tier updates via Stripe webhook events.

CREATE TABLE IF NOT EXISTS subscriptions (
  id TEXT PRIMARY KEY,
  billing_account_id TEXT NOT NULL REFERENCES billing_accounts(id),
  -- Subscription lifecycle status
  status TEXT NOT NULL DEFAULT 'inactive',
  -- 'inactive' | 'active' | 'trialing' | 'past_due' | 'canceled' | 'unpaid'
  -- Plan name mirrors billing_accounts.tier
  plan TEXT NOT NULL DEFAULT 'free',
  -- 'free' | 'pro' | 'business'
  interval TEXT,
  -- 'month' | 'year' | NULL (NULL for free)
  stripe_subscription_id TEXT UNIQUE,
  stripe_customer_id TEXT,
  stripe_price_id TEXT,
  current_period_start INTEGER,
  current_period_end INTEGER,
  cancel_at_period_end INTEGER NOT NULL DEFAULT 0,
  canceled_at INTEGER,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_billing_account
  ON subscriptions(billing_account_id);

CREATE INDEX IF NOT EXISTS idx_subscriptions_stripe_subscription
  ON subscriptions(stripe_subscription_id);

CREATE INDEX IF NOT EXISTS idx_subscriptions_stripe_customer
  ON subscriptions(stripe_customer_id);

CREATE INDEX IF NOT EXISTS idx_subscriptions_status
  ON subscriptions(status, plan);
