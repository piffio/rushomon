-- Custom domains for organizations (Pro+ feature)
-- Allows orgs to use their own domain (e.g. go.mybrand.com) for short links
-- SSL and domain verification are handled via Cloudflare for SaaS API
CREATE TABLE custom_domains (
    id TEXT PRIMARY KEY NOT NULL,
    org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    hostname TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'pending',
    cf_hostname_id TEXT,
    created_at INTEGER NOT NULL,
    verified_at INTEGER
);

CREATE INDEX idx_custom_domains_org ON custom_domains(org_id);
CREATE INDEX idx_custom_domains_hostname ON custom_domains(hostname);
