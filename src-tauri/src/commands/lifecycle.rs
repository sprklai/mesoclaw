//! Tauri IPC commands for lifecycle management.
//!
//! These commands expose the lifecycle supervisor functionality to the frontend.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::lifecycle::{
    InterventionResolution, LifecycleSupervisor, ResourceConfig, ResourceId, ResourceInstance,
    ResourceState, ResourceType, SupervisorStats, UserInterventionRequest,
};

/// Resource status for frontend display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceStatus {
    pub id: String,
    pub resource_type: String,
    pub state: String,
    pub created_at: String,
    pub recovery_attempts: u32,
    pub escalation_tier: u8,
    pub progress: Option<f32>,
    pub substate: Option<String>,
}

impl From<ResourceInstance> for ResourceStatus {
    fn from(instance: ResourceInstance) -> Self {
        let (state, progress, substate) = match &instance.state {
            ResourceState::Idle => ("idle".to_string(), None, None),
            ResourceState::Running {
                substate, progress, ..
            } => ("running".to_string(), *progress, Some(substate.clone())),
            ResourceState::Stuck {
                recovery_attempts, ..
            } => (
                "stuck".to_string(),
                None,
                Some(format!("recovery_attempts={}", recovery_attempts)),
            ),
            ResourceState::Recovering { action, .. } => (
                "recovering".to_string(),
                None,
                Some(format!("{:?}", action)),
            ),
            ResourceState::Completed { .. } => ("completed".to_string(), None, None),
            ResourceState::Failed {
                error, terminal, ..
            } => (
                "failed".to_string(),
                None,
                Some(format!("terminal={}, error={}", terminal, error)),
            ),
        };

        Self {
            id: instance.id.to_string(),
            resource_type: instance.resource_type.to_string(),
            state,
            created_at: instance.created_at.to_rfc3339(),
            recovery_attempts: instance.recovery_attempts,
            escalation_tier: instance.current_escalation_tier,
            progress,
            substate,
        }
    }
}

/// Get the status of all tracked resources.
#[tauri::command]
pub async fn get_all_resources_command(
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<Vec<ResourceStatus>, String> {
    let resources = supervisor.get_all_resources().await;
    Ok(resources.into_iter().map(ResourceStatus::from).collect())
}

/// Get resources filtered by type.
#[tauri::command]
pub async fn get_resources_by_type_command(
    resource_type: String,
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<Vec<ResourceStatus>, String> {
    let rt = parse_resource_type(&resource_type)?;
    let resources = supervisor.get_resources_by_type(rt).await;
    Ok(resources.into_iter().map(ResourceStatus::from).collect())
}

/// Get the status of a specific resource.
#[tauri::command]
pub async fn get_resource_status_command(
    resource_id: String,
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<ResourceStatus, String> {
    let id = ResourceId::parse(&resource_id).map_err(|e| format!("Invalid resource ID: {}", e))?;

    let instance = supervisor
        .get_resource(&id)
        .await
        .ok_or_else(|| format!("Resource not found: {}", resource_id))?;

    Ok(ResourceStatus::from(instance))
}

/// Get all stuck resources.
#[tauri::command]
pub async fn get_stuck_resources_command(
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<Vec<ResourceStatus>, String> {
    let resources = supervisor.get_stuck_resources().await;
    Ok(resources.into_iter().map(ResourceStatus::from).collect())
}

/// Trigger recovery for a stuck resource.
#[tauri::command]
pub async fn retry_resource_command(
    resource_id: String,
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<(), String> {
    let id = ResourceId::parse(&resource_id).map_err(|e| format!("Invalid resource ID: {}", e))?;

    supervisor
        .recover_resource(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Gracefully stop a resource.
#[tauri::command]
pub async fn stop_resource_command(
    resource_id: String,
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<(), String> {
    let id = ResourceId::parse(&resource_id).map_err(|e| format!("Invalid resource ID: {}", e))?;

    supervisor
        .stop_resource(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Force kill a resource.
#[tauri::command]
pub async fn kill_resource_command(
    resource_id: String,
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<(), String> {
    let id = ResourceId::parse(&resource_id).map_err(|e| format!("Invalid resource ID: {}", e))?;

    supervisor
        .kill_resource(&id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Record a heartbeat for a resource.
#[tauri::command]
pub async fn record_resource_heartbeat_command(
    resource_id: String,
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<(), String> {
    let id = ResourceId::parse(&resource_id).map_err(|e| format!("Invalid resource ID: {}", e))?;

    supervisor.record_heartbeat(&id).await;
    Ok(())
}

/// Update resource progress.
#[tauri::command]
pub async fn update_resource_progress_command(
    resource_id: String,
    progress: f32,
    substate: String,
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<(), String> {
    let id = ResourceId::parse(&resource_id).map_err(|e| format!("Invalid resource ID: {}", e))?;

    if !supervisor.update_progress(&id, progress, substate).await {
        return Err(format!("Resource not found: {}", resource_id));
    }

    Ok(())
}

/// Get pending user intervention requests.
#[tauri::command]
pub async fn get_pending_interventions_command(
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<Vec<UserInterventionRequest>, String> {
    let requests = supervisor.get_pending_interventions().await;
    Ok(requests)
}

/// Resolve a user intervention request.
#[tauri::command]
pub async fn resolve_intervention_command(
    request_id: String,
    selected_option: String,
    additional_data: Option<serde_json::Value>,
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<(), String> {
    let resolution = InterventionResolution {
        request_id: request_id.clone(),
        selected_option,
        additional_data,
    };

    supervisor
        .resolve_intervention(&request_id, resolution)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Get supervisor statistics.
#[tauri::command]
pub async fn get_supervisor_stats_command(
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<SupervisorStats, String> {
    let stats = supervisor.get_stats().await;
    Ok(stats)
}

/// Spawn a new resource.
#[tauri::command]
pub async fn spawn_resource_command(
    resource_type: String,
    config: ResourceConfig,
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<String, String> {
    let rt = parse_resource_type(&resource_type)?;

    let id = supervisor
        .spawn_resource(rt, config)
        .await
        .map_err(|e| e.to_string())?;

    Ok(id.to_string())
}

/// Check if monitoring is active.
#[tauri::command]
pub async fn is_lifecycle_monitoring_command(
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<bool, String> {
    Ok(supervisor.is_monitoring().await)
}

/// Get transition history for a resource.
#[tauri::command]
pub async fn get_resource_history_command(
    resource_id: String,
    supervisor: State<'_, Arc<LifecycleSupervisor>>,
) -> Result<Vec<crate::lifecycle::StateTransition>, String> {
    let id = ResourceId::parse(&resource_id).map_err(|e| format!("Invalid resource ID: {}", e))?;

    let history = supervisor.get_transition_history(&id).await;
    Ok(history)
}

/// Parse a resource type string.
fn parse_resource_type(s: &str) -> Result<ResourceType, String> {
    match s.to_lowercase().as_str() {
        "agent" => Ok(ResourceType::Agent),
        "channel" => Ok(ResourceType::Channel),
        "tool" => Ok(ResourceType::Tool),
        "scheduler_job" | "schedulerjob" | "scheduler" => Ok(ResourceType::SchedulerJob),
        "subagent" => Ok(ResourceType::Subagent),
        "gateway_handler" | "gatewayhandler" | "gateway" => Ok(ResourceType::GatewayHandler),
        "memory_operation" | "memoryoperation" | "memory" => Ok(ResourceType::MemoryOperation),
        other if other.starts_with("custom:") => Ok(ResourceType::Custom(
            other.strip_prefix("custom:").unwrap_or(other).to_string(),
        )),
        other => Err(format!("Unknown resource type: {}", other)),
    }
}
