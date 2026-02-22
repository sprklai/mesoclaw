-- Lifecycle instance persistence for crash recovery
-- Stores the state of all tracked resources across restarts

CREATE TABLE lifecycle_instances (
    resource_id TEXT PRIMARY KEY NOT NULL,
    resource_type TEXT NOT NULL,
    state TEXT NOT NULL,
    substate TEXT,
    progress REAL DEFAULT 0.0,
    config_json TEXT NOT NULL,
    escalation_tier INTEGER DEFAULT 0,
    recovery_attempts INTEGER DEFAULT 0,
    heartbeat_interval_secs INTEGER DEFAULT 10,
    stuck_threshold INTEGER DEFAULT 3,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_lifecycle_type ON lifecycle_instances(resource_type);
CREATE INDEX idx_lifecycle_state ON lifecycle_instances(state);
CREATE INDEX idx_lifecycle_updated ON lifecycle_instances(updated_at);

-- Lifecycle transition history (for debugging and audit)
CREATE TABLE lifecycle_transitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    resource_id TEXT NOT NULL,
    from_state TEXT,
    to_state TEXT NOT NULL,
    substate TEXT,
    reason TEXT,
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (resource_id) REFERENCES lifecycle_instances(resource_id) ON DELETE CASCADE
);

CREATE INDEX idx_transitions_resource ON lifecycle_transitions(resource_id);
CREATE INDEX idx_transitions_time ON lifecycle_transitions(timestamp);

-- User intervention requests
CREATE TABLE lifecycle_interventions (
    id TEXT PRIMARY KEY NOT NULL,
    resource_id TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    error_message TEXT NOT NULL,
    recovery_attempts INTEGER DEFAULT 0,
    running_duration_secs INTEGER DEFAULT 0,
    last_state TEXT,
    attempted_tiers TEXT,
    options_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT,
    resolution_option TEXT,
    FOREIGN KEY (resource_id) REFERENCES lifecycle_instances(resource_id) ON DELETE CASCADE
);

CREATE INDEX idx_interventions_resource ON lifecycle_interventions(resource_id);
CREATE INDEX idx_interventions_pending ON lifecycle_interventions(resolved_at);
