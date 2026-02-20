-- Remove skill settings columns from settings table
ALTER TABLE settings DROP COLUMN skill_enabled_ids;
ALTER TABLE settings DROP COLUMN skill_auto_select;
