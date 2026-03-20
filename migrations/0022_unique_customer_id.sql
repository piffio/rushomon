-- Add UNIQUE constraint on provider_customer_id to prevent duplicate Polar customers
-- This is a safety net in case the application-level deduplication fails

CREATE UNIQUE INDEX IF NOT EXISTS idx_billing_accounts_provider_customer_id 
  ON billing_accounts(provider_customer_id) 
  WHERE provider_customer_id IS NOT NULL;
