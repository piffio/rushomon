-- Migration 0012: Add billing_accounts layer to prevent multi-org abuse
-- This introduces a separate billing entity that organizations belong to,
-- ensuring tier limits and quotas are enforced at the billing account level
-- rather than per-organization.

-- Temporarily disable foreign key constraints for migration
PRAGMA foreign_keys = OFF;

-- Create billing_accounts table
CREATE TABLE IF NOT EXISTS billing_accounts (
  id TEXT PRIMARY KEY,
  owner_user_id TEXT NOT NULL,
  tier TEXT NOT NULL DEFAULT 'free',
  stripe_customer_id TEXT,
  created_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_billing_accounts_owner ON billing_accounts(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_billing_accounts_tier ON billing_accounts(tier);

-- Backfill: Create billing account for each existing organization
-- Each org gets its own billing account initially (1:1 mapping)
INSERT INTO billing_accounts (id, owner_user_id, tier, created_at)
SELECT
  'ba_' || id,  -- Prefix to distinguish from org IDs
  created_by,
  tier,
  created_at
FROM organizations;

-- Add billing_account_id to organizations
ALTER TABLE organizations ADD COLUMN billing_account_id TEXT;

-- Link orgs to their billing accounts (backfill from above)
UPDATE organizations SET billing_account_id = 'ba_' || id;

-- Update monthly_counters to track billing account
ALTER TABLE monthly_counters ADD COLUMN billing_account_id TEXT;

-- Backfill billing_account_id in monthly_counters
UPDATE monthly_counters
SET billing_account_id = (
  SELECT billing_account_id
  FROM organizations
  WHERE organizations.id = monthly_counters.org_id
);

CREATE INDEX IF NOT EXISTS idx_monthly_counters_billing_account
ON monthly_counters(billing_account_id, year_month);

-- Re-enable foreign key constraints
PRAGMA foreign_keys = ON;

-- Note: We keep org.tier for backward compatibility during transition
-- After migration is stable, org.tier will be deprecated in favor of billing_account.tier
