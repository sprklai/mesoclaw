-- Revert NOT NULL constraints (restore nullable primary keys)

-- Revert ai_providers.id
CREATE TABLE ai_providers_old (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    requires_api_key INTEGER NOT NULL DEFAULT 1,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    is_user_defined INTEGER NOT NULL DEFAULT 0
);

INSERT INTO ai_providers_old SELECT * FROM ai_providers;
DROP TABLE ai_providers;
ALTER TABLE ai_providers_old RENAME TO ai_providers;

-- Revert ai_models.id
CREATE TABLE ai_models_old (
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

INSERT INTO ai_models_old SELECT * FROM ai_models;
DROP TABLE ai_models;
ALTER TABLE ai_models_old RENAME TO ai_models;

-- Revert generated_prompts.id
CREATE TABLE generated_prompts_old (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    artifact_type TEXT NOT NULL,
    content TEXT NOT NULL,
    disk_path TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    provider_id TEXT,
    model_id TEXT
);

INSERT INTO generated_prompts_old SELECT * FROM generated_prompts;
DROP TABLE generated_prompts;
ALTER TABLE generated_prompts_old RENAME TO generated_prompts;
