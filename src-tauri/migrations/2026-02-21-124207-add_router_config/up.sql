-- Router configuration table (singleton row)
CREATE TABLE router_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    active_profile TEXT NOT NULL DEFAULT 'balanced',
    custom_routes TEXT,  -- JSON: { "code": ["model1", "model2"], ... }
    task_overrides TEXT, -- JSON: { "code": "claude-opus-4", ... }
    last_discovery TEXT, -- ISO timestamp
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Insert default configuration
INSERT INTO router_config (id, active_profile) VALUES (1, 'balanced');

-- Discovered models table with multi-modality support
CREATE TABLE discovered_models (
    id TEXT PRIMARY KEY,              -- e.g., "anthropic/claude-sonnet-4-5"
    display_name TEXT NOT NULL,       -- Human-readable name
    provider_id TEXT NOT NULL,        -- e.g., "vercel-ai-gateway"
    model_id TEXT NOT NULL,           -- Original model ID from provider
    cost_tier TEXT NOT NULL DEFAULT 'medium',  -- 'low', 'medium', 'high'
    context_limit INTEGER DEFAULT 4096,
    -- Multi-modality support
    modalities TEXT NOT NULL DEFAULT '["text"]',  -- JSON array: ["text", "image", "audio"]
    capabilities TEXT,                -- JSON: { "tool_calling": true, "streaming": true, ... }
    -- Metadata
    discovered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    UNIQUE(provider_id, model_id)     -- Prevent duplicates per provider
);

-- Indexes for efficient queries
CREATE INDEX idx_discovered_models_provider ON discovered_models(provider_id);
CREATE INDEX idx_discovered_models_tier ON discovered_models(cost_tier);
CREATE INDEX idx_discovered_models_modalities ON discovered_models(modalities);
CREATE INDEX idx_discovered_models_active ON discovered_models(is_active);
