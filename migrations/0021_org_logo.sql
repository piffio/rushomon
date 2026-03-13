-- Add logo_url to organizations for Pro+ custom QR code branding
ALTER TABLE organizations ADD COLUMN logo_url TEXT;
-- NULL means no logo set. When set, points to /api/orgs/:id/logo (served from R2)
