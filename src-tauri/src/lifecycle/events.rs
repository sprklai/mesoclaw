//! Real-time event emission for lifecycle changes via Tauri.
//!
//! Provides event emission helpers for broadcasting lifecycle state changes
//! to the frontend without polling.

use tauri::{AppHandle, Emitter};

use crate::commands::lifecycle::ResourceStatus;
use crate::lifecycle::states::ResourceInstance;

/// Event names for lifecycle notifications.
pub mod events {
    /// Emitted when a new resource is created.
    pub const SESSION_CREATED: &str = "lifecycle:session:created";

    /// Emitted when resource state changes.
    pub const STATE_CHANGED: &str = "lifecycle:state:changed";

    /// Emitted when a session completes.
    pub const SESSION_COMPLETED: &str = "lifecycle:session:completed";

    /// Emitted when a session fails.
    pub const SESSION_FAILED: &str = "lifecycle:session:failed";

    /// Emitted when a session becomes stuck.
    pub const SESSION_STUCK: &str = "lifecycle:session:stuck";

    /// Emitted when recovery starts.
    pub const RECOVERY_STARTED: &str = "lifecycle:recovery:started";

    /// Emitted when recovery succeeds.
    pub const RECOVERY_SUCCEEDED: &str = "lifecycle:recovery:succeeded";

    /// Emitted when recovery fails.
    pub const RECOVERY_FAILED: &str = "lifecycle:recovery:failed";

    /// Emitted when user intervention is required.
    pub const INTERVENTION_REQUIRED: &str = "lifecycle:intervention:required";

    /// Emitted when an intervention is resolved.
    pub const INTERVENTION_RESOLVED: &str = "lifecycle:intervention:resolved";

    /// Emitted when resource list changes (batch update).
    pub const RESOURCES_UPDATED: &str = "lifecycle:resources:updated";

    /// Emitted with heartbeat status.
    pub const HEARTBEAT_MISSED: &str = "lifecycle:heartbeat:missed";

    /// Emitted when resource progress updates.
    pub const PROGRESS_UPDATED: &str = "lifecycle:progress:updated";
}

/// Payload for state change events.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateChangePayload {
    /// Resource ID
    pub resource_id: String,
    /// Resource type
    pub resource_type: String,
    /// Previous state
    pub from_state: String,
    /// New state
    pub to_state: String,
    /// Current substate
    pub substate: Option<String>,
    /// Progress percentage
    pub progress: Option<f32>,
    /// Timestamp
    pub timestamp: String,
}

/// Payload for intervention events.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterventionPayload {
    /// Request ID
    pub request_id: String,
    /// Resource ID
    pub resource_id: String,
    /// Error message
    pub error: String,
    /// Available options
    pub options: Vec<InterventionOptionPayload>,
}

/// Payload for intervention options.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterventionOptionPayload {
    pub id: String,
    pub label: String,
    pub description: String,
    pub destructive: bool,
}

/// Emit a lifecycle event to the frontend.
pub fn emit_lifecycle_event(
    app: &AppHandle,
    event_type: &str,
    payload: &impl serde::Serialize,
) -> Result<(), String> {
    app.emit(event_type, payload)
        .map_err(|e| format!("Failed to emit event '{}': {}", event_type, e))?;
    Ok(())
}

/// Emit a session created event.
pub fn emit_session_created(app: &AppHandle, instance: &ResourceInstance) -> Result<(), String> {
    // Convert to ResourceStatus for frontend compatibility
    let status = ResourceStatus::from(instance.clone());
    emit_lifecycle_event(app, events::SESSION_CREATED, &status)
}

/// Emit a state changed event.
pub fn emit_state_changed(
    app: &AppHandle,
    resource_id: &str,
    resource_type: &str,
    from_state: &str,
    to_state: &str,
    substate: Option<&str>,
    progress: Option<f32>,
) -> Result<(), String> {
    let payload = StateChangePayload {
        resource_id: resource_id.to_string(),
        resource_type: resource_type.to_string(),
        from_state: from_state.to_string(),
        to_state: to_state.to_string(),
        substate: substate.map(|s| s.to_string()),
        progress,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };
    emit_lifecycle_event(app, events::STATE_CHANGED, &payload)
}

/// Emit a session completed event.
pub fn emit_session_completed(app: &AppHandle, resource_id: &str) -> Result<(), String> {
    emit_lifecycle_event(app, events::SESSION_COMPLETED, &resource_id)
}

/// Emit a session failed event.
pub fn emit_session_failed(app: &AppHandle, resource_id: &str, error: &str) -> Result<(), String> {
    let payload = serde_json::json!({
        "resourceId": resource_id,
        "error": error,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    emit_lifecycle_event(app, events::SESSION_FAILED, &payload)
}

/// Emit a session stuck event.
pub fn emit_session_stuck(
    app: &AppHandle,
    resource_id: &str,
    recovery_attempts: u32,
) -> Result<(), String> {
    let payload = serde_json::json!({
        "resourceId": resource_id,
        "recoveryAttempts": recovery_attempts,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    emit_lifecycle_event(app, events::SESSION_STUCK, &payload)
}

/// Emit a recovery started event.
pub fn emit_recovery_started(
    app: &AppHandle,
    resource_id: &str,
    action: &str,
) -> Result<(), String> {
    let payload = serde_json::json!({
        "resourceId": resource_id,
        "action": action,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    emit_lifecycle_event(app, events::RECOVERY_STARTED, &payload)
}

/// Emit an intervention required event.
pub fn emit_intervention_required(
    app: &AppHandle,
    request_id: &str,
    resource_id: &str,
    error: &str,
    options: &[(String, String, String, bool)],
) -> Result<(), String> {
    let payload = InterventionPayload {
        request_id: request_id.to_string(),
        resource_id: resource_id.to_string(),
        error: error.to_string(),
        options: options
            .iter()
            .map(|(id, label, desc, destructive)| InterventionOptionPayload {
                id: id.clone(),
                label: label.clone(),
                description: desc.clone(),
                destructive: *destructive,
            })
            .collect(),
    };
    emit_lifecycle_event(app, events::INTERVENTION_REQUIRED, &payload)
}

/// Emit a progress update event.
pub fn emit_progress_updated(
    app: &AppHandle,
    resource_id: &str,
    progress: f32,
    substate: &str,
) -> Result<(), String> {
    let payload = serde_json::json!({
        "resourceId": resource_id,
        "progress": progress,
        "substate": substate,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });
    emit_lifecycle_event(app, events::PROGRESS_UPDATED, &payload)
}

/// Emit a resources updated event (batch update).
pub fn emit_resources_updated(
    app: &AppHandle,
    resources: &[ResourceInstance],
) -> Result<(), String> {
    // Convert to ResourceStatus for frontend compatibility
    let statuses: Vec<ResourceStatus> = resources
        .iter()
        .map(|r| ResourceStatus::from(r.clone()))
        .collect();
    emit_lifecycle_event(app, events::RESOURCES_UPDATED, &statuses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_constants() {
        assert_eq!(events::SESSION_CREATED, "lifecycle:session:created");
        assert_eq!(events::STATE_CHANGED, "lifecycle:state:changed");
        assert_eq!(events::SESSION_COMPLETED, "lifecycle:session:completed");
    }

    #[test]
    fn test_state_change_payload_serialization() {
        let payload = StateChangePayload {
            resource_id: "agent:test:123".to_string(),
            resource_type: "agent".to_string(),
            from_state: "idle".to_string(),
            to_state: "running".to_string(),
            substate: Some("thinking".to_string()),
            progress: Some(0.5),
            timestamp: "2026-02-21T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("resourceId"));
        assert!(json.contains("thinking"));
    }
}
