-- Organization domains for just-in-time (JIT) provisioning.
-- Users whose email domain matches a verified org domain are auto-joined to
-- that organization on sign-in.
CREATE TABLE org_domains (
    id TEXT PRIMARY KEY,
    org_id TEXT NOT NULL,
    domain TEXT NOT NULL,
    verification_method TEXT NOT NULL DEFAULT 'dns', -- e.g., 'dns', 'oidc', 'manual'
    verification_token TEXT, -- nullable: methods other than 'dns' won't need it
    is_verified BOOLEAN DEFAULT 0,
    created_at INTEGER NOT NULL,
    verified_at INTEGER,
    FOREIGN KEY (org_id) REFERENCES organizations(id) ON DELETE CASCADE
);

-- Multiple orgs can generate challenges for a domain, but only one can verify it.
CREATE UNIQUE INDEX idx_org_domains_domain ON org_domains(domain) WHERE is_verified = 1;
