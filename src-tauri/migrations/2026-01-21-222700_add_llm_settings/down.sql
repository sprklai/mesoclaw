-- Remove LLM configuration fields from settings table
ALTER TABLE settings DROP COLUMN explanation_verbosity;
ALTER TABLE settings DROP COLUMN use_cloud_llm;
ALTER TABLE settings DROP COLUMN llm_model;
