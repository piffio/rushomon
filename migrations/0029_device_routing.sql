-- Device-based routing URLs (Business tier feature)
-- Allows configuring platform-specific destination URLs for iOS, Android, and Desktop
ALTER TABLE links ADD COLUMN ios_url TEXT;
ALTER TABLE links ADD COLUMN android_url TEXT;
ALTER TABLE links ADD COLUMN desktop_url TEXT;
