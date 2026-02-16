-- SQLite doesn't support DROP COLUMN directly, so we need to recreate the table
-- This migration creates a new table without the notification columns

CREATE TABLE settings_backup (
    id INTEGER PRIMARY KEY NOT NULL DEFAULT 1,
    theme TEXT NOT NULL DEFAULT 'system' CHECK(theme IN ('light', 'dark', 'system')),
    sidebar_expanded INTEGER NOT NULL DEFAULT 1,
    show_in_tray INTEGER NOT NULL DEFAULT 1,
    launch_at_login INTEGER NOT NULL DEFAULT 0,
    enable_logging INTEGER NOT NULL DEFAULT 1,
    log_level TEXT NOT NULL DEFAULT 'info' CHECK(log_level IN ('error', 'warn', 'info', 'debug', 'trace')),
    CHECK(id = 1)
);

INSERT INTO settings_backup (id, theme, sidebar_expanded, show_in_tray, launch_at_login, enable_logging, log_level)
SELECT id, theme, sidebar_expanded, show_in_tray, launch_at_login, enable_logging, log_level FROM settings;

DROP TABLE settings;

ALTER TABLE settings_backup RENAME TO settings;
