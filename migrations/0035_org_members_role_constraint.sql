-- Add CHECK constraint to org_members.role to enforce valid values
-- SQLite does not support ALTER TABLE ADD CONSTRAINT, so we recreate the table.
-- Valid roles: 'owner' | 'admin' | 'member'

PRAGMA foreign_keys = OFF;

-- 1. Drop existing indexes (they survive the table rename and would conflict on recreate)
DROP INDEX IF EXISTS idx_org_members_user;
DROP INDEX IF EXISTS idx_org_members_org;

-- 2. Rename existing table
ALTER TABLE org_members RENAME TO org_members_old;

-- 3. Create new table with CHECK constraint
CREATE TABLE org_members (
  org_id    TEXT NOT NULL,
  user_id   TEXT NOT NULL,
  role      TEXT NOT NULL DEFAULT 'member' CHECK(role IN ('owner', 'admin', 'member')),
  joined_at INTEGER NOT NULL,
  PRIMARY KEY (org_id, user_id),
  FOREIGN KEY (org_id)  REFERENCES organizations(id),
  FOREIGN KEY (user_id) REFERENCES users(id)
);

-- 4. Copy data (existing rows should only contain 'owner' or 'member')
INSERT INTO org_members SELECT * FROM org_members_old;

-- 5. Recreate indexes
CREATE INDEX idx_org_members_user ON org_members(user_id);
CREATE INDEX idx_org_members_org  ON org_members(org_id);

-- 6. Drop old table
DROP TABLE org_members_old;

PRAGMA foreign_keys = ON;
