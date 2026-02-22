//! Persistent storage for lifecycle state using SQLite.
//!
//! Provides crash recovery and audit trail for all tracked resources.

use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::database::DbPool;
use crate::lifecycle::states::{
    FailureContext, HeartbeatConfig, InterventionOption, ResourceConfig, ResourceId,
    ResourceInstance, ResourceState, ResourceType, StateTransition, UserInterventionRequest,
};

/// Maximum number of completed instances to keep for audit.
const KEEP_COMPLETED_COUNT: usize = 100;

/// Persistent storage for lifecycle state.
pub struct LifecycleStorage {
    pool: DbPool,
}

impl LifecycleStorage {
    /// Create a new lifecycle storage instance.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Save or update an instance in the database.
    pub fn save_instance(&self, instance: &ResourceInstance) -> Result<(), String> {
        let conn = &mut self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let config_json = serde_json::to_string(&instance.config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        let state_str = state_to_string(&instance.state);
        let substate = instance.state.substate().map(|s| s.to_string());
        let progress = get_progress(&instance.state);

        diesel::sql_query(
            r#"
            INSERT OR REPLACE INTO lifecycle_instances (
                resource_id, resource_type, state, substate, progress,
                config_json, escalation_tier, recovery_attempts,
                heartbeat_interval_secs, stuck_threshold,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(&instance.id.to_string())
        .bind::<diesel::sql_types::Text, _>(&instance.resource_type.to_string())
        .bind::<diesel::sql_types::Text, _>(&state_str)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&substate)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Float>, _>(&progress)
        .bind::<diesel::sql_types::Text, _>(&config_json)
        .bind::<diesel::sql_types::Integer, _>(instance.current_escalation_tier as i32)
        .bind::<diesel::sql_types::Integer, _>(instance.recovery_attempts as i32)
        .bind::<diesel::sql_types::Integer, _>(instance.heartbeat_config.interval_secs as i32)
        .bind::<diesel::sql_types::Integer, _>(instance.heartbeat_config.stuck_threshold as i32)
        .bind::<diesel::sql_types::Text, _>(&instance.created_at.to_rfc3339())
        .execute(conn)
        .map_err(|e| format!("Failed to save instance: {}", e))?;

        Ok(())
    }

    /// Load all active instances (not completed/failed) on startup.
    pub fn load_active_instances(&self) -> Result<Vec<ResourceInstance>, String> {
        let conn = &mut self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let rows: Vec<InstanceRow> = diesel::sql_query(
            r#"
            SELECT resource_id, resource_type, state, substate, progress,
                   config_json, escalation_tier, recovery_attempts,
                   heartbeat_interval_secs, stuck_threshold,
                   created_at, updated_at
            FROM lifecycle_instances
            WHERE state NOT IN ('completed', 'failed')
            ORDER BY updated_at DESC
            "#,
        )
        .load(conn)
        .map_err(|e| format!("Failed to load instances: {}", e))?;

        rows.into_iter().map(|row| row_to_instance(row)).collect()
    }

    /// Load a specific instance by ID.
    pub fn load_instance(
        &self,
        resource_id: &ResourceId,
    ) -> Result<Option<ResourceInstance>, String> {
        let conn = &mut self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let rows: Vec<InstanceRow> = diesel::sql_query(
            r#"
            SELECT resource_id, resource_type, state, substate, progress,
                   config_json, escalation_tier, recovery_attempts,
                   heartbeat_interval_secs, stuck_threshold,
                   created_at, updated_at
            FROM lifecycle_instances
            WHERE resource_id = ?
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(&resource_id.to_string())
        .load(conn)
        .map_err(|e| format!("Failed to load instance: {}", e))?;

        rows.into_iter()
            .next()
            .map(|row| row_to_instance(row))
            .transpose()
    }

    /// Record a state transition in the history.
    pub fn record_transition(
        &self,
        transition: &StateTransition,
        substate: Option<&str>,
    ) -> Result<(), String> {
        let conn = &mut self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        diesel::sql_query(
            r#"
            INSERT INTO lifecycle_transitions (
                resource_id, from_state, to_state, substate, reason, timestamp
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(&transition.resource_id)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&transition.from_state)
        .bind::<diesel::sql_types::Text, _>(&transition.to_state)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&substate)
        .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&transition.reason)
        .bind::<diesel::sql_types::Text, _>(&transition.timestamp.to_rfc3339())
        .execute(conn)
        .map_err(|e| format!("Failed to record transition: {}", e))?;

        Ok(())
    }

    /// Get transition history for a resource.
    pub fn get_transitions(
        &self,
        resource_id: &ResourceId,
        limit: usize,
    ) -> Result<Vec<StateTransition>, String> {
        let conn = &mut self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let rows: Vec<TransitionRow> = diesel::sql_query(
            r#"
            SELECT resource_id, from_state, to_state, substate, reason, timestamp
            FROM lifecycle_transitions
            WHERE resource_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(&resource_id.to_string())
        .bind::<diesel::sql_types::Integer, _>(limit as i32)
        .load(conn)
        .map_err(|e| format!("Failed to load transitions: {}", e))?;

        rows.into_iter()
            .map(|row| {
                Ok(StateTransition {
                    resource_id: row.resource_id,
                    from_state: row.from_state.unwrap_or_default(),
                    to_state: row.to_state,
                    timestamp: parse_datetime(&row.timestamp)?,
                    reason: row.reason.unwrap_or_default(),
                })
            })
            .collect()
    }

    /// Delete completed instances beyond the retention limit.
    pub fn cleanup_old_instances(&self) -> Result<(), String> {
        let conn = &mut self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        diesel::sql_query(
            r#"
            DELETE FROM lifecycle_instances
            WHERE rowid IN (
                SELECT rowid FROM lifecycle_instances
                WHERE state IN ('completed', 'failed')
                ORDER BY updated_at DESC
                LIMIT -1 OFFSET ?
            )
            "#,
        )
        .bind::<diesel::sql_types::Integer, _>(KEEP_COMPLETED_COUNT as i32)
        .execute(conn)
        .map_err(|e| format!("Failed to cleanup instances: {}", e))?;

        Ok(())
    }

    /// Remove an instance from storage.
    pub fn remove_instance(&self, resource_id: &ResourceId) -> Result<(), String> {
        let conn = &mut self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        diesel::sql_query("DELETE FROM lifecycle_instances WHERE resource_id = ?")
            .bind::<diesel::sql_types::Text, _>(&resource_id.to_string())
            .execute(conn)
            .map_err(|e| format!("Failed to remove instance: {}", e))?;

        Ok(())
    }

    /// Save a user intervention request.
    pub fn save_intervention(&self, request: &UserInterventionRequest) -> Result<(), String> {
        let conn = &mut self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let attempted_tiers_json = serde_json::to_string(&request.attempted_tiers)
            .map_err(|e| format!("Failed to serialize attempted tiers: {}", e))?;

        let options_json = serde_json::to_string(&request.options)
            .map_err(|e| format!("Failed to serialize options: {}", e))?;

        diesel::sql_query(
            r#"
            INSERT OR REPLACE INTO lifecycle_interventions (
                id, resource_id, resource_type, error_message,
                recovery_attempts, running_duration_secs, last_state,
                attempted_tiers, options_json, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(&request.id)
        .bind::<diesel::sql_types::Text, _>(&request.resource_id)
        .bind::<diesel::sql_types::Text, _>(&request.resource_type.to_string())
        .bind::<diesel::sql_types::Text, _>(&request.failure_context.error)
        .bind::<diesel::sql_types::Integer, _>(request.failure_context.recovery_attempts as i32)
        .bind::<diesel::sql_types::Integer, _>(request.failure_context.running_duration_secs as i32)
        .bind::<diesel::sql_types::Text, _>(&request.failure_context.last_state)
        .bind::<diesel::sql_types::Text, _>(&attempted_tiers_json)
        .bind::<diesel::sql_types::Text, _>(&options_json)
        .bind::<diesel::sql_types::Text, _>(&request.created_at.to_rfc3339())
        .execute(conn)
        .map_err(|e| format!("Failed to save intervention: {}", e))?;

        Ok(())
    }

    /// Load pending (unresolved) intervention requests.
    pub fn load_pending_interventions(&self) -> Result<Vec<UserInterventionRequest>, String> {
        let conn = &mut self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        let rows: Vec<InterventionRow> = diesel::sql_query(
            r#"
            SELECT id, resource_id, resource_type, error_message,
                   recovery_attempts, running_duration_secs, last_state,
                   attempted_tiers, options_json, created_at, resolved_at, resolution_option
            FROM lifecycle_interventions
            WHERE resolved_at IS NULL
            ORDER BY created_at DESC
            "#,
        )
        .load(conn)
        .map_err(|e| format!("Failed to load interventions: {}", e))?;

        rows.into_iter()
            .map(|row| row_to_intervention(row))
            .collect()
    }

    /// Mark an intervention as resolved.
    pub fn resolve_intervention(
        &self,
        request_id: &str,
        resolution_option: &str,
    ) -> Result<(), String> {
        let conn = &mut self
            .pool
            .get()
            .map_err(|e| format!("Failed to get database connection: {}", e))?;

        diesel::sql_query(
            r#"
            UPDATE lifecycle_interventions
            SET resolved_at = datetime('now'), resolution_option = ?
            WHERE id = ?
            "#,
        )
        .bind::<diesel::sql_types::Text, _>(resolution_option)
        .bind::<diesel::sql_types::Text, _>(request_id)
        .execute(conn)
        .map_err(|e| format!("Failed to resolve intervention: {}", e))?;

        Ok(())
    }
}

// ─── Helper Types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, QueryableByName)]
struct InstanceRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    resource_id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    resource_type: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    state: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    substate: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Float>)]
    progress: Option<f32>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    config_json: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    escalation_tier: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    recovery_attempts: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    heartbeat_interval_secs: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    stuck_threshold: i32,
    #[diesel(sql_type = diesel::sql_types::Text)]
    created_at: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    updated_at: String,
}

#[derive(Debug, Clone, QueryableByName)]
struct TransitionRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    resource_id: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    from_state: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    to_state: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    substate: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    reason: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Text)]
    timestamp: String,
}

#[derive(Debug, Clone, QueryableByName)]
struct InterventionRow {
    #[diesel(sql_type = diesel::sql_types::Text)]
    id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    resource_id: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    resource_type: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    error_message: String,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    recovery_attempts: i32,
    #[diesel(sql_type = diesel::sql_types::Integer)]
    running_duration_secs: i32,
    #[diesel(sql_type = diesel::sql_types::Text)]
    last_state: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    attempted_tiers: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    options_json: String,
    #[diesel(sql_type = diesel::sql_types::Text)]
    created_at: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    resolved_at: Option<String>,
    #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
    resolution_option: Option<String>,
}

// ─── Helper Functions ────────────────────────────────────────────────────────

fn row_to_instance(row: InstanceRow) -> Result<ResourceInstance, String> {
    let resource_type = parse_resource_type(&row.resource_type)?;
    let instance_id = row
        .resource_id
        .splitn(2, ':')
        .nth(1)
        .unwrap_or(&row.resource_id)
        .to_string();
    let resource_id = ResourceId::new(resource_type.clone(), instance_id);

    let config: ResourceConfig = serde_json::from_str(&row.config_json)
        .map_err(|e| format!("Failed to parse config JSON: {}", e))?;

    let state = parse_state(&row.state, &row.substate, row.progress)?;

    let created_at = parse_datetime(&row.created_at)?;

    let heartbeat_config = HeartbeatConfig {
        interval_secs: row.heartbeat_interval_secs as u64,
        stuck_threshold: row.stuck_threshold as u32,
        ..Default::default()
    };

    Ok(ResourceInstance {
        id: resource_id,
        resource_type,
        state,
        config,
        created_at,
        recovery_attempts: row.recovery_attempts as u32,
        current_escalation_tier: row.escalation_tier as u8,
        heartbeat_config,
    })
}

fn row_to_intervention(row: InterventionRow) -> Result<UserInterventionRequest, String> {
    let resource_type = parse_resource_type(&row.resource_type)?;

    let attempted_tiers: Vec<u8> = serde_json::from_str(&row.attempted_tiers)
        .map_err(|e| format!("Failed to parse attempted tiers: {}", e))?;

    let options: Vec<InterventionOption> = serde_json::from_str(&row.options_json)
        .map_err(|e| format!("Failed to parse options: {}", e))?;

    let created_at = parse_datetime(&row.created_at)?;

    Ok(UserInterventionRequest {
        id: row.id,
        resource_id: row.resource_id,
        resource_type,
        failure_context: FailureContext {
            error: row.error_message,
            recovery_attempts: row.recovery_attempts as u32,
            running_duration_secs: row.running_duration_secs as u64,
            last_state: row.last_state,
            failed_at: created_at,
        },
        attempted_tiers,
        options,
        created_at,
    })
}

fn parse_resource_type(s: &str) -> Result<ResourceType, String> {
    match s {
        "agent" => Ok(ResourceType::Agent),
        "channel" => Ok(ResourceType::Channel),
        "tool" => Ok(ResourceType::Tool),
        "scheduler_job" => Ok(ResourceType::SchedulerJob),
        "subagent" => Ok(ResourceType::Subagent),
        "gateway_handler" => Ok(ResourceType::GatewayHandler),
        "memory_operation" => Ok(ResourceType::MemoryOperation),
        other if other.starts_with("custom:") => Ok(ResourceType::Custom(
            other.strip_prefix("custom:").unwrap_or(other).to_string(),
        )),
        other => Err(format!("Unknown resource type: {}", other)),
    }
}

fn state_to_string(state: &ResourceState) -> String {
    match state {
        ResourceState::Idle => "idle".to_string(),
        ResourceState::Running { .. } => "running".to_string(),
        ResourceState::Stuck { .. } => "stuck".to_string(),
        ResourceState::Recovering { .. } => "recovering".to_string(),
        ResourceState::Completed { .. } => "completed".to_string(),
        ResourceState::Failed { .. } => "failed".to_string(),
    }
}

fn get_progress(state: &ResourceState) -> Option<f32> {
    match state {
        ResourceState::Running { progress, .. } => *progress,
        _ => None,
    }
}

fn parse_state(
    state_str: &str,
    substate: &Option<String>,
    progress: Option<f32>,
) -> Result<ResourceState, String> {
    let now = Utc::now();
    match state_str {
        "idle" => Ok(ResourceState::Idle),
        "running" => Ok(ResourceState::Running {
            substate: substate
                .clone()
                .unwrap_or_else(|| "initialized".to_string()),
            started_at: now,
            progress,
        }),
        "stuck" => Ok(ResourceState::Stuck {
            since: now,
            recovery_attempts: 0,
            last_known_progress: progress,
        }),
        "recovering" => Ok(ResourceState::Recovering {
            action: crate::lifecycle::states::RecoveryActionType::Retry,
            started_at: now,
        }),
        "completed" => Ok(ResourceState::Completed {
            at: now,
            result: None,
        }),
        "failed" => Ok(ResourceState::Failed {
            at: now,
            error: "Recovered from crash".to_string(),
            terminal: false,
            escalation_tier_reached: 0,
        }),
        other => Err(format!("Unknown state: {}", other)),
    }
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, String> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .map(|dt| chrono::DateTime::from_naive_utc_and_offset(dt, Utc))
        })
        .map_err(|e| format!("Failed to parse datetime '{}': {}", s, e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_to_string() {
        assert_eq!(state_to_string(&ResourceState::Idle), "idle");
        assert_eq!(
            state_to_string(&ResourceState::Running {
                substate: "thinking".to_string(),
                started_at: Utc::now(),
                progress: Some(0.5)
            }),
            "running"
        );
    }

    #[test]
    fn test_parse_resource_type() {
        assert!(matches!(
            parse_resource_type("agent"),
            Ok(ResourceType::Agent)
        ));
        assert!(matches!(
            parse_resource_type("channel"),
            Ok(ResourceType::Channel)
        ));
        assert!(parse_resource_type("invalid").is_err());
    }
}
