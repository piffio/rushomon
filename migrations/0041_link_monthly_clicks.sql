-- Migration 0041: Per-link monthly click counters
-- Pre-aggregates clicks per (link_id, month) so the monthly stats email can
-- compute org totals and top links without scanning the entire analytics_events table.

CREATE TABLE IF NOT EXISTS link_monthly_clicks (
  link_id TEXT NOT NULL,
  org_id TEXT NOT NULL,
  year_month TEXT NOT NULL,  -- Format: "YYYY-MM"
  clicks INTEGER DEFAULT 0,
  updated_at INTEGER NOT NULL,
  PRIMARY KEY (link_id, year_month),
  FOREIGN KEY (link_id) REFERENCES links(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_link_monthly_clicks_org
  ON link_monthly_clicks(org_id, year_month);

-- Backfill from existing analytics events. This is a one-time scan of the
-- analytics_events table; afterwards the email job reads only this small table.
INSERT INTO link_monthly_clicks (link_id, org_id, year_month, clicks, updated_at)
SELECT
  link_id,
  org_id,
  strftime('%Y-%m', timestamp, 'unixepoch') AS year_month,
  COUNT(*) AS clicks,
  strftime('%s', 'now') AS updated_at
FROM analytics_events
GROUP BY link_id, org_id, strftime('%Y-%m', timestamp, 'unixepoch')
ON CONFLICT(link_id, year_month) DO UPDATE SET
  clicks = excluded.clicks,
  updated_at = excluded.updated_at;
