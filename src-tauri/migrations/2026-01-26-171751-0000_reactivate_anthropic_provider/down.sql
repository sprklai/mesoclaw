-- Revert: Deactivate Anthropic provider again
UPDATE ai_providers SET is_active = 0 WHERE id = 'anthropic';
UPDATE ai_models SET is_active = 0 WHERE provider_id = 'anthropic';
