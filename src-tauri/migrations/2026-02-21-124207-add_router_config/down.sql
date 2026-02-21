-- Drop indexes first
DROP INDEX IF EXISTS idx_discovered_models_active;
DROP INDEX IF EXISTS idx_discovered_models_modalities;
DROP INDEX IF EXISTS idx_discovered_models_tier;
DROP INDEX IF EXISTS idx_discovered_models_provider;

-- Drop tables
DROP TABLE IF EXISTS discovered_models;
DROP TABLE IF EXISTS router_config;
