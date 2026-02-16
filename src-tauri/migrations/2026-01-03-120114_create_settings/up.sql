-- Create settings table with typed columns
-- Uses CHECK(id = 1) to ensure only one settings row exists (singleton pattern)

CREATE TABLE settings (
    id INTEGER PRIMARY KEY NOT NULL DEFAULT 1,
    -- Appearance
    theme TEXT NOT NULL DEFAULT 'system' CHECK(theme IN ('light', 'dark', 'system')),
    sidebar_expanded INTEGER NOT NULL DEFAULT 1,
    -- Behavior
    show_in_tray INTEGER NOT NULL DEFAULT 1,
    launch_at_login INTEGER NOT NULL DEFAULT 0,
    -- Developer
    enable_logging INTEGER NOT NULL DEFAULT 1,
    log_level TEXT NOT NULL DEFAULT 'info' CHECK(log_level IN ('error', 'warn', 'info', 'debug', 'trace')),
    -- Ensure single row
    CHECK(id = 1)
);

-- Insert default settings row
INSERT INTO settings (id) VALUES (1);
