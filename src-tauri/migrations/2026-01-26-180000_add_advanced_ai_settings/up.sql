-- Add advanced AI settings fields to settings table
-- These settings control AI request behavior and are used by all AI providers

ALTER TABLE settings ADD COLUMN temperature REAL NOT NULL DEFAULT 0.7 CHECK(temperature >= 0.0 AND temperature <= 1.0);
ALTER TABLE settings ADD COLUMN max_tokens INTEGER NOT NULL DEFAULT 4096 CHECK(max_tokens >= 256 AND max_tokens <= 32768);
ALTER TABLE settings ADD COLUMN timeout INTEGER NOT NULL DEFAULT 30 CHECK(timeout >= 5 AND timeout <= 300);
ALTER TABLE settings ADD COLUMN stream_responses INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN enable_caching INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN debug_mode INTEGER NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN custom_base_url TEXT;
