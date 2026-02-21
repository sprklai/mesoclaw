-- Remove user identity columns from settings table
ALTER TABLE settings DROP COLUMN app_display_name;
ALTER TABLE settings DROP COLUMN user_name;
