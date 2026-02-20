-- Add agent, session, and run tables for multi-agent system

-- Agent configuration table
CREATE TABLE agents (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    system_prompt TEXT NOT NULL,
    model_id TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens INTEGER,
    tools_enabled INTEGER NOT NULL DEFAULT 1,
    memory_enabled INTEGER NOT NULL DEFAULT 1,
    workspace_path TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (provider_id) REFERENCES ai_providers(id) ON DELETE RESTRICT
);

CREATE INDEX idx_agents_provider ON agents(provider_id);
CREATE INDEX idx_agents_active ON agents(is_active);

-- Agent session table (groups multiple runs)
CREATE TABLE agent_sessions (
    id TEXT NOT NULL PRIMARY KEY,
    agent_id TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

CREATE INDEX idx_sessions_agent ON agent_sessions(agent_id);
CREATE INDEX idx_sessions_status ON agent_sessions(status);

-- Agent run table (individual execution instances)
CREATE TABLE agent_runs (
    id TEXT NOT NULL PRIMARY KEY,
    session_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    parent_run_id TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    input_message TEXT NOT NULL,
    output_message TEXT,
    error_message TEXT,
    tokens_used INTEGER,
    duration_ms INTEGER,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_run_id) REFERENCES agent_runs(id) ON DELETE CASCADE
);

CREATE INDEX idx_runs_session ON agent_runs(session_id);
CREATE INDEX idx_runs_agent ON agent_runs(agent_id);
CREATE INDEX idx_runs_status ON agent_runs(status);
CREATE INDEX idx_runs_parent ON agent_runs(parent_run_id);
