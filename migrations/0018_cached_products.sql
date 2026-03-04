-- Create table for cached Polar product data
CREATE TABLE IF NOT EXISTS cached_products (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    price_amount INTEGER NOT NULL, -- Price in cents
    price_currency TEXT NOT NULL DEFAULT 'eur',
    recurring_interval TEXT, -- 'month', 'year', or null for one-time
    recurring_interval_count INTEGER DEFAULT 1,
    is_archived BOOLEAN NOT NULL DEFAULT FALSE,
    polar_product_id TEXT NOT NULL, -- The actual Polar product ID
    polar_price_id TEXT NOT NULL, -- The specific price ID
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_cached_products_polar_product_id ON cached_products(polar_product_id);

-- Create index for archiving queries
CREATE INDEX IF NOT EXISTS idx_cached_products_is_archived ON cached_products(is_archived);
