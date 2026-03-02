-- Migration 0017: Add pricing fields to subscriptions table
-- Store actual pricing data from Polar webhooks to display real prices including discounts

-- Add pricing columns to subscriptions table
ALTER TABLE subscriptions ADD COLUMN amount_cents INTEGER;
ALTER TABLE subscriptions ADD COLUMN currency TEXT DEFAULT 'eur';
ALTER TABLE subscriptions ADD COLUMN discount_name TEXT;

-- Add founder pricing setting to settings table
INSERT OR REPLACE INTO settings (key, value, updated_at) 
VALUES ('founder_pricing_active', 'true', strftime('%s', 'now'));

-- Add indexes for new pricing fields (optional but useful for admin queries)
CREATE INDEX IF NOT EXISTS idx_subscriptions_amount ON subscriptions(amount_cents);
CREATE INDEX IF NOT EXISTS idx_subscriptions_currency ON subscriptions(currency);
CREATE INDEX IF NOT EXISTS idx_subscriptions_discount ON subscriptions(discount_name);
