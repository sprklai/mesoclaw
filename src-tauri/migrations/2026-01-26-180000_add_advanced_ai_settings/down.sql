-- Rollback advanced AI settings
ALTER TABLE settings DROP COLUMN custom_base_url;
ALTER TABLE settings DROP COLUMN debug_mode;
ALTER TABLE settings DROP COLUMN enable_caching;
ALTER TABLE settings DROP COLUMN stream_responses;
ALTER TABLE settings DROP COLUMN timeout;
ALTER TABLE settings DROP COLUMN max_tokens;
ALTER TABLE settings DROP COLUMN temperature;
