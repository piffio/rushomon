-- Migration 0016: Switch billing provider from Stripe to Polar
-- Renames Stripe-specific columns to provider-agnostic names.
-- billing_accounts: stripe_customer_id → provider_customer_id
-- subscriptions: stripe_subscription_id → provider_subscription_id
--                stripe_customer_id     → provider_customer_id
--                stripe_price_id        → provider_price_id

-- billing_accounts table
ALTER TABLE billing_accounts RENAME COLUMN stripe_customer_id TO provider_customer_id;

-- subscriptions table
ALTER TABLE subscriptions RENAME COLUMN stripe_subscription_id TO provider_subscription_id;
ALTER TABLE subscriptions RENAME COLUMN stripe_customer_id TO provider_customer_id;
ALTER TABLE subscriptions RENAME COLUMN stripe_price_id TO provider_price_id;

-- Drop old Stripe-named indexes and recreate with generic names
DROP INDEX IF EXISTS idx_subscriptions_stripe_subscription;
DROP INDEX IF EXISTS idx_subscriptions_stripe_customer;

CREATE UNIQUE INDEX IF NOT EXISTS idx_subscriptions_provider_subscription
  ON subscriptions(provider_subscription_id);

CREATE INDEX IF NOT EXISTS idx_subscriptions_provider_customer
  ON subscriptions(provider_customer_id);
