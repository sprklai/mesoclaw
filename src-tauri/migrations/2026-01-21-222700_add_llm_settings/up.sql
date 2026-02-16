-- Add LLM configuration fields to settings table
ALTER TABLE settings ADD COLUMN llm_model TEXT NOT NULL DEFAULT 'google/gemini-3-flash';
ALTER TABLE settings ADD COLUMN use_cloud_llm INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN explanation_verbosity TEXT NOT NULL DEFAULT 'balanced' CHECK(explanation_verbosity IN ('concise', 'balanced', 'detailed'));
