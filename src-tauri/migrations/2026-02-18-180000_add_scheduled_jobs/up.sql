-- Persisted scheduler jobs.
-- schedule_json and payload_json store the serde-serialised enum values.
CREATE TABLE IF NOT EXISTS scheduled_jobs (
    id           TEXT PRIMARY KEY NOT NULL,
    name         TEXT NOT NULL,
    schedule_json TEXT NOT NULL,
    session_target TEXT NOT NULL DEFAULT 'main',
    payload_json  TEXT NOT NULL,
    enabled       INTEGER NOT NULL DEFAULT 1,
    error_count   INTEGER NOT NULL DEFAULT 0,
    next_run      TEXT,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
);
