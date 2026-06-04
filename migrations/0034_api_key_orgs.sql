-- API Key Org Scope
--
-- Stores which organizations an API key is authorized to act on behalf of.
-- A key with no rows in this table is treated as legacy and falls back to
-- the org_id stored directly on the api_keys row (backward compatibility).
--
-- When a user leaves or is removed from an org:
--   - Their key's row for that org is deleted (via ON DELETE CASCADE on org_id
--     is NOT used here since a user leaving is not an org deletion — the org
--     still exists; we handle removal explicitly in application code)
--   - If a key ends up with zero allowed orgs it is automatically revoked

CREATE TABLE api_key_orgs (
    api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    org_id     TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    PRIMARY KEY (api_key_id, org_id)
);

CREATE INDEX IF NOT EXISTS idx_api_key_orgs_key ON api_key_orgs(api_key_id);
CREATE INDEX IF NOT EXISTS idx_api_key_orgs_org ON api_key_orgs(org_id);
