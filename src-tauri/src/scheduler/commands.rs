//! Tauri IPC commands for scheduler management.
//!
//! ## TODO (Phase 4.1 follow-up)
//! Wire commands to managed `Arc<TokioScheduler>` app state.  Requires
//! deciding how to initialise the scheduler at Tauri startup.

use serde_json::Value;

/// List all registered scheduler jobs.
#[tauri::command]
pub async fn list_jobs_command() -> Result<Vec<Value>, String> {
    // ## TODO: Resolve managed TokioScheduler from app state
    Err("Scheduler not yet wired to app state. Planned for Phase 4 follow-up.".to_string())
}

/// Create a new scheduled job.
#[tauri::command]
pub async fn create_job_command(
    _name: String,
    _schedule: Value,
    _payload: Value,
    _enabled: Option<bool>,
) -> Result<String, String> {
    // ## TODO: Resolve managed TokioScheduler from app state
    Err("Scheduler not yet wired to app state. Planned for Phase 4 follow-up.".to_string())
}

/// Enable or disable a job.
#[tauri::command]
pub async fn toggle_job_command(_job_id: String, _enabled: bool) -> Result<(), String> {
    // ## TODO: Resolve managed TokioScheduler from app state
    Err("Scheduler not yet wired to app state. Planned for Phase 4 follow-up.".to_string())
}

/// Delete a scheduled job.
#[tauri::command]
pub async fn delete_job_command(_job_id: String) -> Result<bool, String> {
    // ## TODO: Resolve managed TokioScheduler from app state
    Err("Scheduler not yet wired to app state. Planned for Phase 4 follow-up.".to_string())
}

/// Retrieve execution history for a job.
#[tauri::command]
pub async fn job_history_command(_job_id: String) -> Result<Vec<Value>, String> {
    // ## TODO: Resolve managed TokioScheduler from app state
    Err("Scheduler not yet wired to app state. Planned for Phase 4 follow-up.".to_string())
}
