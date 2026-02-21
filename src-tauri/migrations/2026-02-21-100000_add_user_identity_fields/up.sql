-- Add user identity columns to existing settings table
ALTER TABLE settings ADD COLUMN user_name TEXT;
ALTER TABLE settings ADD COLUMN app_display_name TEXT;
