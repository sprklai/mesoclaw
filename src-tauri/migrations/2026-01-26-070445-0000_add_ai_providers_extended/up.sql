-- Add extended AI providers (OpenAI, Anthropic, Gemini, Ollama)
--
-- This migration adds the remaining standard AI providers to the existing
-- ai_providers and ai_models tables. No schema changes are needed because
-- all providers use the same OpenAI-compatible protocol.
--
-- Provider-Agnostic Design:
-- - No provider_type column needed (all providers are identical at protocol level)
-- - Only differences are base_url and requires_api_key flag
-- - Model ID format is just a string convention
--
-- NOTE: Models are now managed in src-tauri/src/ai/model_registry.rs
-- To seed models after migration, call the seed_ai_models_command from the frontend

-- Update existing providers to match our naming
UPDATE ai_providers SET
  name = 'OpenRouter',
  base_url = 'https://openrouter.ai/api/v1',
  requires_api_key = 1
WHERE id = 'openrouter';

UPDATE ai_providers SET
  name = 'Vercel AI Gateway',
  base_url = 'https://ai-gateway.vercel.sh/v1',
  requires_api_key = 1
WHERE id = 'vercel-ai-gateway';

-- Insert new standard providers
INSERT INTO ai_providers (id, name, base_url, requires_api_key, is_active, created_at)
VALUES
  ('openai', 'OpenAI', 'https://api.openai.com/v1', 1, 1, datetime('now')),
  ('anthropic', 'Anthropic', 'https://api.anthropic.com/v1', 1, 1, datetime('now')),
  ('gemini', 'Google Gemini', 'https://generativelanguage.googleapis.com/v1beta/openai', 1, 1, datetime('now')),
  ('ollama', 'Ollama (Local)', 'http://localhost:11434/v1', 0, 1, datetime('now'))
ON CONFLICT(id) DO UPDATE SET
  name = excluded.name,
  base_url = excluded.base_url,
  requires_api_key = excluded.requires_api_key;

-- Note: Models are seeded via seed_ai_models_command
-- This allows for easier model management without needing new migrations
