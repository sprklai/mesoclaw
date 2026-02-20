-- GAP-8: DND schedule and per-category notification preferences

-- DND schedule: opt-in flag plus time window (defaults: off, 22:00–07:00)
ALTER TABLE settings ADD COLUMN dnd_schedule_enabled    INTEGER NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN dnd_start_hour          INTEGER NOT NULL DEFAULT 22;
ALTER TABLE settings ADD COLUMN dnd_end_hour            INTEGER NOT NULL DEFAULT 7;

-- Per-category notification flags (1 = enabled by default)
ALTER TABLE settings ADD COLUMN notify_heartbeat        INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN notify_cron_reminder    INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN notify_agent_complete   INTEGER NOT NULL DEFAULT 1;
ALTER TABLE settings ADD COLUMN notify_approval_request INTEGER NOT NULL DEFAULT 1;
