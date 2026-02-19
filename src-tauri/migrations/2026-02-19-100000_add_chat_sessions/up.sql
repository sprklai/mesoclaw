-- Phase 4.3.3: Structured session key routing.
-- Sessions follow the format {agent}:{scope}:{channel}:{peer}.
-- Existing in-memory sessions will be persisted here once the
-- SessionRouter is wired to the database layer.

CREATE TABLE IF NOT EXISTS chat_sessions (
    id          TEXT PRIMARY KEY NOT NULL,
    session_key TEXT NOT NULL UNIQUE,
    agent       TEXT NOT NULL DEFAULT 'main',
    scope       TEXT NOT NULL DEFAULT 'dm',
    channel     TEXT NOT NULL DEFAULT 'tauri',
    peer        TEXT NOT NULL DEFAULT 'user',
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
    compaction_summary TEXT
);

CREATE INDEX idx_chat_sessions_session_key ON chat_sessions (session_key);
CREATE INDEX idx_chat_sessions_channel     ON chat_sessions (channel);
