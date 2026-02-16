-- Remove direct Anthropic provider
--
-- Anthropic does not provide an OpenAI-compatible API endpoint.
-- To use Claude models, users should go through:
-- - Vercel AI Gateway (vercel-ai-gateway provider)
-- - OpenRouter (openrouter provider)
-- Both gateways provide OpenAI-compatible access to Claude models.
--
-- See: https://docs.anthropic.com/en/api/versioning

-- Deactivate the direct Anthropic provider
UPDATE ai_providers SET is_active = 0 WHERE id = 'anthropic';

-- Deactivate all models for the direct Anthropic provider
UPDATE ai_models SET is_active = 0 WHERE provider_id = 'anthropic';
