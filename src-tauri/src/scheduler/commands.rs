//! Tauri IPC commands for scheduler management.

use std::sync::Arc;

use tauri::State;

use crate::scheduler::{
    TokioScheduler,
    traits::{JobPayload, Schedule, ScheduledJob, Scheduler as _, SessionTarget},
};

/// List all registered scheduled jobs.
#[tauri::command]
#[tracing::instrument(name = "command.scheduler.list_jobs", skip(scheduler))]
pub async fn list_jobs_command(
    scheduler: State<'_, Arc<TokioScheduler>>,
) -> Result<Vec<ScheduledJob>, String> {
    Ok(scheduler.list_jobs().await)
}

/// Create a new scheduled job.
///
/// `schedule_json` must be a valid [`Schedule`] (e.g. `{"type":"interval","secs":1800}`).
/// `payload_json` must be a valid [`JobPayload`].
#[tauri::command]
#[tracing::instrument(name = "command.scheduler.create_job", skip(scheduler, schedule_json, payload_json), fields(name = %name))]
pub async fn create_job_command(
    name: String,
    schedule_json: serde_json::Value,
    payload_json: serde_json::Value,
    enabled: Option<bool>,
    scheduler: State<'_, Arc<TokioScheduler>>,
) -> Result<String, String> {
    let schedule: Schedule =
        serde_json::from_value(schedule_json).map_err(|e| format!("Invalid schedule: {e}"))?;
    let payload: JobPayload =
        serde_json::from_value(payload_json).map_err(|e| format!("Invalid payload: {e}"))?;

    let job = ScheduledJob {
        id: String::new(), // assigned by scheduler
        name,
        schedule,
        session_target: SessionTarget::Main,
        payload,
        enabled: enabled.unwrap_or(true),
        error_count: 0,
        next_run: None,
    };

    let id = scheduler.add_job(job).await;
    Ok(id)
}

/// Enable or disable a scheduled job.
///
/// Implemented as remove + re-add with the updated `enabled` flag because
/// `Scheduler` does not expose a direct mutation method.
#[tauri::command]
#[tracing::instrument(name = "command.scheduler.toggle_job", skip(scheduler), fields(job_id = %job_id, enabled))]
pub async fn toggle_job_command(
    job_id: String,
    enabled: bool,
    scheduler: State<'_, Arc<TokioScheduler>>,
) -> Result<(), String> {
    use crate::scheduler::traits::Scheduler as _;
    let jobs = scheduler.list_jobs().await;
    let mut job = jobs
        .into_iter()
        .find(|j| j.id == job_id)
        .ok_or_else(|| format!("Job '{}' not found", job_id))?;
    scheduler.remove_job(&job_id).await;
    job.enabled = enabled;
    scheduler.add_job(job).await;
    Ok(())
}

/// Delete a scheduled job by id.
#[tauri::command]
#[tracing::instrument(name = "command.scheduler.delete_job", skip(scheduler), fields(job_id = %job_id))]
pub async fn delete_job_command(
    job_id: String,
    scheduler: State<'_, Arc<TokioScheduler>>,
) -> Result<bool, String> {
    Ok(scheduler.remove_job(&job_id).await)
}

/// Retrieve execution history for a job.
#[tauri::command]
#[tracing::instrument(name = "command.scheduler.job_history", skip(scheduler), fields(job_id = %job_id))]
pub async fn job_history_command(
    job_id: String,
    scheduler: State<'_, Arc<TokioScheduler>>,
) -> Result<Vec<crate::scheduler::traits::JobExecution>, String> {
    Ok(scheduler.job_history(&job_id).await)
}
