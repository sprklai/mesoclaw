-- Add notification settings columns to settings table

ALTER TABLE settings ADD COLUMN enable_notifications INTEGER NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN notify_general INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN notify_reminders INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN notify_updates INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN notify_alerts INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN notify_activity INTEGER NOT NULL DEFAULT 0;
