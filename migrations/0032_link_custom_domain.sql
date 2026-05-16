-- Track which custom domain a link was created under.
-- NULL means the link uses the default short domain.
-- Immutable after creation (same as short_code) — changing domain would
-- orphan all previously shared links since the KV key is {hostname}:{short_code}.
ALTER TABLE links ADD COLUMN custom_domain TEXT;
