-- SQLite does not support DROP COLUMN in older versions; recreate the table.
-- This is a best-effort rollback that removes the GAP-8 columns.

CREATE TABLE settings_backup AS SELECT
    id, theme, sidebar_expanded, show_in_tray, launch_at_login,
    enable_logging, log_level, enable_notifications,
    notify_general, notify_reminders, notify_updates, notify_alerts, notify_activity,
    llm_model, use_cloud_llm, explanation_verbosity,
    temperature, max_tokens, timeout, stream_responses, enable_caching, debug_mode,
    custom_base_url, default_provider_id, default_model_id
FROM settings;

DROP TABLE settings;

ALTER TABLE settings_backup RENAME TO settings;
