-- Organization Members & Invitations
-- Adds many-to-many org membership and invitation system

-- Junction table: many-to-many users <-> organizations
CREATE TABLE org_members (
  org_id    TEXT NOT NULL,
  user_id   TEXT NOT NULL,
  role      TEXT NOT NULL DEFAULT 'member',  -- 'owner' | 'member'
  joined_at INTEGER NOT NULL,
  PRIMARY KEY (org_id, user_id),
  FOREIGN KEY (org_id)  REFERENCES organizations(id),
  FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE INDEX idx_org_members_user ON org_members(user_id);
CREATE INDEX idx_org_members_org  ON org_members(org_id);

-- Pending invitations (token = id UUID)
CREATE TABLE org_invitations (
  id          TEXT PRIMARY KEY,      -- UUID, also serves as the magic-link token
  org_id      TEXT NOT NULL,
  invited_by  TEXT NOT NULL,         -- user_id of inviter
  email       TEXT NOT NULL,         -- target email address
  role        TEXT NOT NULL DEFAULT 'member',
  created_at  INTEGER NOT NULL,
  expires_at  INTEGER NOT NULL,      -- created_at + 7 days
  accepted_at INTEGER,               -- NULL = pending
  FOREIGN KEY (org_id)     REFERENCES organizations(id),
  FOREIGN KEY (invited_by) REFERENCES users(id)
);

CREATE INDEX idx_org_invitations_org   ON org_invitations(org_id);
CREATE INDEX idx_org_invitations_email ON org_invitations(email);

-- Backfill: every existing user becomes 'owner' of their personal org
INSERT INTO org_members (org_id, user_id, role, joined_at)
SELECT org_id, id, 'owner', created_at FROM users;
