-- Revert: Reactivate direct Anthropic provider
-- Note: This will not work because Anthropic doesn't have an OpenAI-compatible API

UPDATE ai_providers SET is_active = 1 WHERE id = 'anthropic';
UPDATE ai_models SET is_active = 1 WHERE provider_id = 'anthropic';
