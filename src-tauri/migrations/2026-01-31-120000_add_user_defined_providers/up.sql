-- Add is_user_defined column to ai_providers table
ALTER TABLE ai_providers ADD COLUMN is_user_defined INTEGER NOT NULL DEFAULT 0;

-- Add global default model columns to settings table
ALTER TABLE settings ADD COLUMN default_provider_id TEXT;
ALTER TABLE settings ADD COLUMN default_model_id TEXT;
