-- Add redirect_type column to links table
-- Allows users to choose between 301 (permanent) and 307 (temporary) redirects
-- Default is 301 for SEO optimization, 307 available on Pro+ plans

ALTER TABLE links ADD COLUMN redirect_type TEXT NOT NULL DEFAULT '301';
