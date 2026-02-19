CREATE TABLE generated_prompts (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  artifact_type TEXT NOT NULL,
  content TEXT NOT NULL,
  disk_path TEXT,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  provider_id TEXT,
  model_id TEXT
);
