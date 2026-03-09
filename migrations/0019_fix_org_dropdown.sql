-- Migration 0019: Fix organization dropdown by ensuring all users have org_members records
-- Add missing org_members records for users who own their default organization
-- but don't have corresponding org_members entries
INSERT INTO org_members (org_id, user_id, role, joined_at)
SELECT u.org_id, u.id, 'owner', u.created_at
FROM users u
LEFT JOIN org_members m ON u.org_id = m.org_id AND u.id = m.user_id
WHERE u.org_id IS NOT NULL
AND m.user_id IS NULL
ON CONFLICT(org_id, user_id) DO NOTHING;
