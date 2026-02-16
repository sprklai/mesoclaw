-- Reactivate Anthropic provider
--
-- Note: Direct Anthropic API requires their Messages API format, not OpenAI-compatible.
-- To use Claude models directly, you'll need to implement Anthropic's native API.
-- 
-- For now, we recommend using:
-- - OpenRouter (openrouter provider) - OpenAI-compatible access to Claude
-- - Vercel AI Gateway (vercel-ai-gateway provider) - OpenAI-compatible access to Claude
--
-- If you want to use direct Anthropic API, you'll need to:
-- 1. Use endpoint: https://api.anthropic.com/v1/messages
-- 2. Use Anthropic-specific request format (not OpenAI-compatible)
-- 3. Add anthropic-version header

-- Reactivate the provider (user can choose to use it or not)
UPDATE ai_providers SET is_active = 1 WHERE id = 'anthropic';

-- Reactivate all models for the provider
UPDATE ai_models SET is_active = 1 WHERE provider_id = 'anthropic';
