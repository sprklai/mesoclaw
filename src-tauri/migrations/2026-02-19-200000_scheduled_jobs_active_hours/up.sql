-- Add active hours window and one-shot support to scheduled jobs.
ALTER TABLE scheduled_jobs ADD COLUMN active_hours_json TEXT;
ALTER TABLE scheduled_jobs ADD COLUMN delete_after_run INTEGER NOT NULL DEFAULT 0;
