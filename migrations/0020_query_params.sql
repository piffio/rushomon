-- Add UTM params and query parameter forwarding to links
ALTER TABLE links ADD COLUMN utm_params TEXT;
-- JSON: {"utm_source":"...","utm_medium":"...","utm_campaign":"...","utm_term":"...","utm_content":"...", "ref":"..."}
-- Only 5 standard Google UTM fields plus ref. NULL if not set.

ALTER TABLE links ADD COLUMN forward_query_params INTEGER;
-- NULL = use org default, 1 = always forward, 0 = never forward

-- Add org-level default for query parameter forwarding
ALTER TABLE organizations ADD COLUMN forward_query_params INTEGER NOT NULL DEFAULT 0;
-- 0 = off by default (changes apply only to new links or links edited after this change)
