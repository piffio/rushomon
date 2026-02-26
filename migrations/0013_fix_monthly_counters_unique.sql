-- Migration 0013: Simplify monthly_counters to billing account level only
-- Removes org_id column and updates primary key to billing_account_id

-- Disable foreign key constraints temporarily
PRAGMA foreign_keys = OFF;

-- Create new table with simplified schema
CREATE TABLE monthly_counters_new (
  billing_account_id TEXT NOT NULL,
  year_month TEXT NOT NULL,  -- Format: "2026-02"
  links_created INTEGER DEFAULT 0,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (billing_account_id, year_month)
);

-- Migrate data: Aggregate org-level counters to billing account level
INSERT OR REPLACE INTO monthly_counters_new (billing_account_id, year_month, links_created, updated_at)
SELECT o.billing_account_id, mc.year_month, MAX(mc.links_created), MAX(mc.updated_at)
FROM monthly_counters mc
JOIN organizations o ON mc.org_id = o.id
WHERE mc.org_id != ''
GROUP BY o.billing_account_id, mc.year_month;

-- Also migrate existing billing account rows (where org_id was empty)
INSERT OR REPLACE INTO monthly_counters_new (billing_account_id, year_month, links_created, updated_at)
SELECT billing_account_id, year_month, links_created, updated_at
FROM monthly_counters
WHERE billing_account_id IS NOT NULL AND (org_id IS NULL OR org_id = '');

-- Drop old table
DROP TABLE monthly_counters;

-- Rename new table
ALTER TABLE monthly_counters_new RENAME TO monthly_counters;

-- Create index for efficient lookups
CREATE INDEX idx_monthly_counters_lookup ON monthly_counters(billing_account_id, year_month);

-- Re-enable foreign key constraints
PRAGMA foreign_keys = ON;
