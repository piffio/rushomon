-- Link abuse reports table
-- Stores user reports for inappropriate/malicious links
-- Supports both authenticated and anonymous reporting

CREATE TABLE link_reports (
  id TEXT PRIMARY KEY,
  link_id TEXT NOT NULL,
  reason TEXT NOT NULL,
  reporter_user_id TEXT,
  reporter_email TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  admin_notes TEXT,
  reviewed_by TEXT,
  reviewed_at INTEGER,
  created_at INTEGER NOT NULL,
  FOREIGN KEY (link_id) REFERENCES links(id),
  FOREIGN KEY (reporter_user_id) REFERENCES users(id),
  FOREIGN KEY (reviewed_by) REFERENCES users(id)
);

-- Indexes for efficient queries
CREATE INDEX idx_reports_link_id ON link_reports(link_id);
CREATE INDEX idx_reports_status ON link_reports(status);
CREATE INDEX idx_reports_created_at ON link_reports(created_at DESC);
CREATE INDEX idx_reports_reporter_user ON link_reports(reporter_user_id);
CREATE INDEX idx_reports_reviewed_by ON link_reports(reviewed_by);

-- Composite index for pending reports queries
CREATE INDEX idx_reports_pending_created ON link_reports(status, created_at DESC) WHERE status = 'pending';
