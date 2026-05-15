-- Add SSL certificate status tracking to custom_domains
-- This allows us to track SSL certificate status separately from hostname status
-- Hostname can be "active" (CNAME verified) while SSL cert is still "pending"
ALTER TABLE custom_domains ADD COLUMN ssl_status TEXT NOT NULL DEFAULT 'pending';
