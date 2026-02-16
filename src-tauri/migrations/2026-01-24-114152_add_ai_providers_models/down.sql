-- Drop tables in reverse order of creation
DROP INDEX IF EXISTS idx_ai_models_provider_model;
DROP TABLE IF EXISTS ai_models;
DROP TABLE IF EXISTS ai_providers;
