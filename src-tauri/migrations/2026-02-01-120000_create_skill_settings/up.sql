-- Create skill_settings table for per-workspace skill configuration
CREATE TABLE skill_settings (
    workspace_id TEXT PRIMARY KEY NOT NULL,
    settings_json TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

-- Index for faster lookups
CREATE INDEX idx_skill_settings_workspace ON skill_settings(workspace_id);
