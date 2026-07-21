-- Org-level default: exclude ambiguous characters (0, O, I, l) from
-- randomly generated short codes by using the Base58 alphabet
ALTER TABLE organizations ADD COLUMN exclude_ambiguous_chars INTEGER NOT NULL DEFAULT 0;
