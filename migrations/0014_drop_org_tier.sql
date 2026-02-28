-- Migration: Drop deprecated tier column from organizations
-- Tier limits are now enforced at the billing account level via billing_accounts.tier
--
-- This is the final step in migrating from per-organization tier to billing account tier.
-- All code now reads tier from the associated billing account.

-- Drop the deprecated tier column from organizations
ALTER TABLE organizations DROP COLUMN tier;

-- Note: The billing_account_id column (added in 0012_billing_accounts.sql)
-- links organizations to their billing account, which contains the authoritative tier.
