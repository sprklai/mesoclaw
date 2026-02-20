-- Tool execution audit log — persists every tool invocation for security review.
CREATE TABLE IF NOT EXISTS tool_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    timestamp TEXT NOT NULL,           -- ISO 8601 UTC
    session_id TEXT,                   -- Agent session identifier (nullable)
    tool_name TEXT NOT NULL,           -- Name of the tool that was invoked
    args TEXT NOT NULL,                -- JSON-serialised arguments
    risk_level TEXT NOT NULL,          -- "low" | "medium" | "high"
    decision TEXT NOT NULL,            -- "allowed" | "needs_approval" | "denied"
    result TEXT,                       -- Tool output (nullable — null if denied)
    success INTEGER NOT NULL DEFAULT 0 -- 1 = success, 0 = failure/denied
);

-- Index for time-range queries and session filtering
CREATE INDEX IF NOT EXISTS idx_tool_audit_timestamp ON tool_audit_log (timestamp);
CREATE INDEX IF NOT EXISTS idx_tool_audit_session ON tool_audit_log (session_id);
CREATE INDEX IF NOT EXISTS idx_tool_audit_tool ON tool_audit_log (tool_name);
