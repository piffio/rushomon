-- Monthly usage counters for organizations
-- Tracks links created and clicks tracked per organization per month
-- Replaces expensive COUNT queries on the links table

CREATE TABLE monthly_counters (
  org_id TEXT NOT NULL,
  year_month TEXT NOT NULL,  -- Format: "2026-02"
  links_created INTEGER DEFAULT 0,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (org_id, year_month),
  FOREIGN KEY (org_id) REFERENCES organizations(id)
);

-- Index for efficient lookups (redundant with PRIMARY KEY but explicit for clarity)
CREATE INDEX idx_monthly_counters_lookup ON monthly_counters(org_id, year_month);
