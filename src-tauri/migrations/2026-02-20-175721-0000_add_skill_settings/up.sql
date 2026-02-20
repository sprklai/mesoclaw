-- Add skill settings columns to existing settings table
ALTER TABLE settings ADD COLUMN skill_auto_select INTEGER NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN skill_enabled_ids TEXT NOT NULL DEFAULT '[]';
