-- Add a default model for Ollama
--
-- This adds a default placeholder model for Ollama since models are discovered
-- dynamically via Ollama's API (GET http://localhost:11434/v1/models).
--
-- Note: The model_id "llama3" is a common default model that users can pull via:
--   ollama pull llama3
--
-- TODO: Implement dynamic model discovery by calling Ollama's /v1/models endpoint

INSERT INTO ai_models (id, provider_id, model_id, display_name, context_limit, is_custom, is_active, created_at)
VALUES
  ('ollama_llama3', 'ollama', 'llama3', 'Llama 3 (Default)', 128000, 0, 1, datetime('now'))
ON CONFLICT(id) DO NOTHING;
