-- Add AI provider and model tables
CREATE TABLE ai_providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    requires_api_key INTEGER NOT NULL DEFAULT 1,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE ai_models (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    context_limit INTEGER,
    is_custom INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (provider_id) REFERENCES ai_providers(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_ai_models_provider_model ON ai_models(provider_id, model_id);

-- Seed data for built-in providers and models
INSERT INTO ai_providers (id, name, base_url) VALUES
    ('vercel-ai-gateway', 'Vercel AI Gateway', 'https://ai-gateway.vercel.sh/v1'),
    ('openrouter', 'OpenRouter', 'https://openrouter.ai/api/v1');

INSERT INTO ai_models (id, provider_id, model_id, display_name, context_limit) VALUES
    -- Vercel AI Gateway models (existing)
    ('vercel-gpt-4o', 'vercel-ai-gateway', 'openai/gpt-4o', 'GPT-4o', 128000),
    ('vercel-claude-sonnet-4.5', 'vercel-ai-gateway', 'anthropic/claude-sonnet-4.5', 'Claude Sonnet 4.5', 200000),
    ('vercel-gemini-3-flash', 'vercel-ai-gateway', 'google/gemini-3-flash', 'Gemini 3 Flash', 1000000),
    -- OpenRouter popular models
    ('openrouter-claude-3.5-sonnet', 'openrouter', 'anthropic/claude-3.5-sonnet', 'Claude 3.5 Sonnet', 200000),
    ('openrouter-gpt-4o', 'openrouter', 'openai/gpt-4o', 'GPT-4o', 128000);
