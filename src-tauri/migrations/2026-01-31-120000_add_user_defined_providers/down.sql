-- SQLite doesn't support DROP COLUMN directly in older versions
-- For SQLite 3.35.0+, these would work:
-- ALTER TABLE ai_providers DROP COLUMN is_user_defined;
-- ALTER TABLE settings DROP COLUMN default_provider_id;
-- ALTER TABLE settings DROP COLUMN default_model_id;

-- For compatibility, we create new tables without the columns and migrate data
-- ai_providers rollback
CREATE TABLE ai_providers_backup (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    requires_api_key INTEGER NOT NULL DEFAULT 1,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO ai_providers_backup (id, name, base_url, requires_api_key, is_active, created_at)
SELECT id, name, base_url, requires_api_key, is_active, created_at FROM ai_providers;

DROP TABLE ai_providers;

ALTER TABLE ai_providers_backup RENAME TO ai_providers;

-- settings rollback
CREATE TABLE settings_backup (
    id INTEGER PRIMARY KEY CHECK(id = 1) NOT NULL DEFAULT 1,
    theme TEXT NOT NULL DEFAULT 'system',
    sidebar_expanded INTEGER NOT NULL DEFAULT 1,
    show_in_tray INTEGER NOT NULL DEFAULT 1,
    launch_at_login INTEGER NOT NULL DEFAULT 0,
    enable_logging INTEGER NOT NULL DEFAULT 1,
    log_level TEXT NOT NULL DEFAULT 'info',
    enable_notifications INTEGER NOT NULL DEFAULT 1,
    notify_general INTEGER NOT NULL DEFAULT 1,
    notify_reminders INTEGER NOT NULL DEFAULT 1,
    notify_updates INTEGER NOT NULL DEFAULT 1,
    notify_alerts INTEGER NOT NULL DEFAULT 1,
    notify_activity INTEGER NOT NULL DEFAULT 1,
    llm_model TEXT NOT NULL DEFAULT 'gemini-2.0-flash-exp',
    use_cloud_llm INTEGER NOT NULL DEFAULT 1,
    explanation_verbosity TEXT NOT NULL DEFAULT 'detailed',
    temperature REAL NOT NULL DEFAULT 0.3,
    max_tokens INTEGER NOT NULL DEFAULT 4096,
    timeout INTEGER NOT NULL DEFAULT 60000,
    stream_responses INTEGER NOT NULL DEFAULT 1,
    enable_caching INTEGER NOT NULL DEFAULT 1,
    debug_mode INTEGER NOT NULL DEFAULT 0,
    custom_base_url TEXT
);

INSERT INTO settings_backup (
    id, theme, sidebar_expanded, show_in_tray, launch_at_login, enable_logging, log_level,
    enable_notifications, notify_general, notify_reminders, notify_updates, notify_alerts, notify_activity,
    llm_model, use_cloud_llm, explanation_verbosity, temperature, max_tokens, timeout,
    stream_responses, enable_caching, debug_mode, custom_base_url
)
SELECT
    id, theme, sidebar_expanded, show_in_tray, launch_at_login, enable_logging, log_level,
    enable_notifications, notify_general, notify_reminders, notify_updates, notify_alerts, notify_activity,
    llm_model, use_cloud_llm, explanation_verbosity, temperature, max_tokens, timeout,
    stream_responses, enable_caching, debug_mode, custom_base_url
FROM settings;

DROP TABLE settings;

ALTER TABLE settings_backup RENAME TO settings;
