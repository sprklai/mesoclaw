-- SQLite does not support DROP COLUMN directly; recreate without the new columns.
CREATE TABLE scheduled_jobs_backup AS SELECT
    id, name, schedule_json, session_target, payload_json,
    enabled, error_count, next_run, created_at
FROM scheduled_jobs;
DROP TABLE scheduled_jobs;
ALTER TABLE scheduled_jobs_backup RENAME TO scheduled_jobs;
