use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{
    agent::{agent_commands::SessionCancelMap, session_router::SessionRouter},
    database::DbPool,
    database::models::ai_provider::AIProviderData,
    database::schema::ai_providers,
    event_bus::{AppEvent, EventBus},
    identity::{IdentityLoader, types::IDENTITY_FILES, types::IdentityFileInfo},
    lifecycle::{LifecycleSupervisor, ResourceId},
    memory::{
        store::InMemoryStore,
        traits::{Memory as _, MemoryCategory},
    },
    modules::{ModuleRegistry, SidecarModule, SidecarTool},
    scheduler::{
        TokioScheduler,
        traits::{JobPayload, Schedule, ScheduledJob, Scheduler as _, SessionTarget},
    },
};

// ─── Shared gateway state ─────────────────────────────────────────────────────

/// All state shared across gateway route handlers.
#[derive(Clone)]
pub struct GatewayState {
    pub bus: Arc<dyn EventBus>,
    pub sessions: Arc<SessionRouter>,
    pub modules: Arc<ModuleRegistry>,
    pub db_pool: DbPool,
    pub identity_loader: Arc<IdentityLoader>,
    pub memory: Arc<InMemoryStore>,
    pub scheduler: Arc<TokioScheduler>,
    /// Shared cancel flags for running agent sessions.
    pub cancel_map: SessionCancelMap,
    /// Lifecycle supervisor for resource management.
    pub lifecycle: Arc<LifecycleSupervisor>,
}

// ─── Health ───────────────────────────────────────────────────────────────────

pub async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "mesoclaw-daemon" }))
}

// ─── Agent sessions ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub system_prompt: Option<String>,
    pub provider_id: Option<String>,
    /// Optional session channel context — defaults to "user" (desktop IPC).
    pub channel: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub status: String,
}

#[tracing::instrument(name = "gateway.create_session", skip_all, fields(channel = ?req.channel))]
pub async fn create_session(
    State(state): State<GatewayState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    let channel = req.channel.as_deref().unwrap_or("user");
    let key = state.sessions.resolve(channel, req.context.as_deref());
    match state.sessions.get_or_create(key.clone()) {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({
                "session_id": key.to_string(),
                "status": "created"
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        ),
    }
}

pub async fn list_sessions(State(state): State<GatewayState>) -> impl IntoResponse {
    let keys: Vec<String> = state
        .sessions
        .list_keys()
        .iter()
        .map(|k| k.to_string())
        .collect();
    Json(json!({ "sessions": keys, "count": keys.len() }))
}

// ─── Provider status ──────────────────────────────────────────────────────────

pub async fn provider_status(State(state): State<GatewayState>) -> impl IntoResponse {
    let mut conn = match state.db_pool.get() {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "error", "error": format!("database error: {e}") })),
            )
                .into_response();
        }
    };

    let providers = ai_providers::table
        .filter(ai_providers::is_active.eq(1))
        .load::<crate::database::models::ai_provider::AIProvider>(&mut conn)
        .map(|rows| {
            rows.into_iter()
                .map(|p| {
                    let data = AIProviderData::from(p);
                    json!({
                        "id": data.id,
                        "name": data.name,
                        "isActive": data.is_active,
                        "requiresApiKey": data.requires_api_key,
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Json(json!({ "status": "ok", "providers": providers, "count": providers.len() }))
        .into_response()
}

// ─── Module management ────────────────────────────────────────────────────────

/// Summary of a registered sidecar module returned by the list endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleSummary {
    pub id: String,
}

/// List all registered sidecar modules.
pub async fn list_modules(State(state): State<GatewayState>) -> impl IntoResponse {
    let modules: Vec<serde_json::Value> = state
        .modules
        .ids()
        .iter()
        .map(|id| json!({ "id": id }))
        .collect();
    Json(json!({ "modules": modules, "count": modules.len() }))
}

/// Path parameters for module-specific routes.
#[derive(Debug, serde::Deserialize)]
pub struct ModuleId {
    pub id: String,
}

/// Return health status for a single module.
#[tracing::instrument(name = "gateway.module_health", skip(state), fields(id = %params.id))]
pub async fn module_health(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ModuleId>,
) -> impl IntoResponse {
    match state.modules.get(&params.id) {
        Some(_) => (
            StatusCode::OK,
            Json(json!({ "id": params.id, "healthy": true, "registered": true })),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "id": params.id, "healthy": false, "error": "module not found" })),
        )
            .into_response(),
    }
}

/// Start a service-type module via the SidecarModule trait.
#[tracing::instrument(name = "gateway.start_module", skip(state), fields(id = %params.id))]
pub async fn start_module(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ModuleId>,
) -> impl IntoResponse {
    let module: Arc<SidecarTool> = match state.modules.get(&params.id) {
        Some(m) => m,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "id": params.id, "error": "module not found" })),
            )
                .into_response();
        }
    };
    match SidecarModule::start(module.as_ref()).await {
        Ok(()) => Json(json!({ "id": params.id, "status": "started" })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "id": params.id, "error": e })),
        )
            .into_response(),
    }
}

/// Stop a running service-type module via the SidecarModule trait.
#[tracing::instrument(name = "gateway.stop_module", skip(state), fields(id = %params.id))]
pub async fn stop_module(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ModuleId>,
) -> impl IntoResponse {
    let module: Arc<SidecarTool> = match state.modules.get(&params.id) {
        Some(m) => m,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "id": params.id, "error": "module not found" })),
            )
                .into_response();
        }
    };
    match SidecarModule::stop(module.as_ref()).await {
        Ok(()) => Json(json!({ "id": params.id, "status": "stopped" })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "id": params.id, "error": e })),
        )
            .into_response(),
    }
}

/// Reload (re-discover) all modules from disk.
///
/// Scans `~/.mesoclaw/modules/` and updates the in-memory registry: newly
/// added manifests are registered, deleted ones are removed.  Modules already
/// known to `ToolRegistry` (from startup) remain callable; **newly added**
/// modules will appear in the registry but won't be callable by the agent loop
/// until the daemon restarts (ToolRegistry requires `&mut self`).
pub async fn reload_modules(State(state): State<GatewayState>) -> impl IntoResponse {
    let (added, removed) = state.modules.reload();
    let ids = state.modules.ids();
    Json(json!({
        "status": "ok",
        "added": added,
        "removed": removed,
        "current_modules": ids,
        "count": ids.len(),
        "note": if added > 0 {
            "New modules discovered. Restart daemon to make them callable by the agent loop."
        } else {
            "Module registry is up to date."
        },
    }))
}

// ─── Identity endpoints ──────────────────────────────────────────────────────

/// Path parameter for identity file routes.
#[derive(serde::Deserialize)]
pub struct IdentityFileParam {
    pub file: String,
}

/// Request body for updating an identity file.
#[derive(Debug, Deserialize)]
pub struct UpdateIdentityRequest {
    pub content: String,
}

/// `GET /api/v1/identity` — list all canonical identity files with metadata.
pub async fn list_identity_files(State(state): State<GatewayState>) -> impl IntoResponse {
    // Ensure identity is loaded (trigger a read).
    let _ = state.identity_loader.get();

    let files: Vec<IdentityFileInfo> = IDENTITY_FILES
        .iter()
        .map(|(file_name, description)| IdentityFileInfo {
            name: file_name.trim_end_matches(".md").to_string(),
            file_name: file_name.to_string(),
            description: description.to_string(),
        })
        .collect();

    Json(json!({ "files": files, "count": files.len() }))
}

/// `GET /api/v1/identity/{file}` — return the raw content of one identity file.
pub async fn get_identity_file(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<IdentityFileParam>,
) -> impl IntoResponse {
    match state.identity_loader.get_file(&params.file) {
        Ok(content) => Json(json!({ "file": params.file, "content": content })).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "file": params.file, "error": e })),
        )
            .into_response(),
    }
}

/// `PUT /api/v1/identity/{file}` — overwrite one identity file and hot-reload.
pub async fn update_identity_file(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<IdentityFileParam>,
    Json(req): Json<UpdateIdentityRequest>,
) -> impl IntoResponse {
    match state
        .identity_loader
        .update_file(&params.file, &req.content)
    {
        Ok(()) => Json(json!({ "file": params.file, "status": "updated" })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "file": params.file, "error": e })),
        )
            .into_response(),
    }
}

// ─── Memory endpoints ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct StoreMemoryRequest {
    pub key: String,
    pub content: String,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchMemoryQuery {
    pub q: String,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct MemoryKey {
    pub key: String,
}

/// `GET /api/v1/memory` — list all stored memory entries.
pub async fn list_memory(State(state): State<GatewayState>) -> impl IntoResponse {
    match state.memory.recall("", 1000).await {
        Ok(entries) => Json(json!({ "entries": entries, "count": entries.len() })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

/// `POST /api/v1/memory` — store a new memory entry.
pub async fn store_memory(
    State(state): State<GatewayState>,
    Json(req): Json<StoreMemoryRequest>,
) -> impl IntoResponse {
    let category = match req.category.as_deref() {
        Some("daily") => MemoryCategory::Daily,
        Some("conversation") => MemoryCategory::Conversation,
        Some(other) => MemoryCategory::Custom(other.to_string()),
        None => MemoryCategory::Core,
    };
    match state.memory.store(&req.key, &req.content, category).await {
        Ok(()) => Json(json!({ "key": req.key, "status": "stored" })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

/// `GET /api/v1/memory/search?q=...&limit=...` — hybrid search.
pub async fn search_memory(
    State(state): State<GatewayState>,
    axum::extract::Query(params): axum::extract::Query<SearchMemoryQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(10);
    match state.memory.recall(&params.q, limit).await {
        Ok(entries) => Json(json!({ "entries": entries, "count": entries.len() })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

/// `DELETE /api/v1/memory/{key}` — remove a memory entry by key.
pub async fn forget_memory(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<MemoryKey>,
) -> impl IntoResponse {
    match state.memory.forget(&params.key).await {
        Ok(true) => Json(json!({ "key": params.key, "status": "forgotten" })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "key": params.key, "error": "entry not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e })),
        )
            .into_response(),
    }
}

// ─── Approval endpoint ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ApprovalRequest {
    pub approved: bool,
}

#[derive(Debug, Deserialize)]
pub struct ActionId {
    pub action_id: String,
}

/// `POST /api/v1/approval/{action_id}` — approve or deny a pending agent action.
///
/// Publishes an `ApprovalResponse` event to the EventBus so the waiting agent
/// loop can resume or abort accordingly.
pub async fn send_approval(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ActionId>,
    Json(req): Json<ApprovalRequest>,
) -> impl IntoResponse {
    let event = AppEvent::ApprovalResponse {
        action_id: params.action_id.clone(),
        approved: req.approved,
    };
    match state.bus.publish(event) {
        Ok(()) => Json(json!({
            "action_id": params.action_id,
            "approved": req.approved,
            "status": "published"
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("failed to publish approval: {e}") })),
        )
            .into_response(),
    }
}

// ─── Scheduler endpoints ──────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateJobRequest {
    pub name: String,
    pub schedule: serde_json::Value,
    pub payload: serde_json::Value,
    pub enabled: Option<bool>,
    pub active_hours: Option<serde_json::Value>,
    pub delete_after_run: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ToggleJobRequest {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct JobIdPath {
    pub job_id: String,
}

/// `GET /api/v1/scheduler/jobs` — list all scheduled jobs.
pub async fn list_scheduler_jobs(State(state): State<GatewayState>) -> impl IntoResponse {
    let jobs = state.scheduler.list_jobs().await;
    Json(json!({ "jobs": jobs, "count": jobs.len() }))
}

/// `POST /api/v1/scheduler/jobs` — create a new scheduled job.
pub async fn create_scheduler_job(
    State(state): State<GatewayState>,
    Json(req): Json<CreateJobRequest>,
) -> impl IntoResponse {
    let schedule: Schedule = match serde_json::from_value(req.schedule) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("invalid schedule: {e}") })),
            )
                .into_response();
        }
    };
    let payload: JobPayload = match serde_json::from_value(req.payload) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("invalid payload: {e}") })),
            )
                .into_response();
        }
    };
    let active_hours = req
        .active_hours
        .and_then(|v| serde_json::from_value(v).ok());
    let job = ScheduledJob {
        id: String::new(),
        name: req.name,
        schedule,
        session_target: SessionTarget::Main,
        payload,
        enabled: req.enabled.unwrap_or(true),
        error_count: 0,
        next_run: None,
        active_hours,
        delete_after_run: req.delete_after_run.unwrap_or(false),
    };
    let id = state.scheduler.add_job(job).await;
    (
        StatusCode::CREATED,
        Json(json!({ "id": id, "status": "created" })),
    )
        .into_response()
}

/// `PUT /api/v1/scheduler/jobs/{job_id}/toggle` — enable or disable a job.
pub async fn toggle_scheduler_job(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<JobIdPath>,
    Json(req): Json<ToggleJobRequest>,
) -> impl IntoResponse {
    let jobs = state.scheduler.list_jobs().await;
    let Some(mut job) = jobs.into_iter().find(|j| j.id == params.job_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("job '{}' not found", params.job_id) })),
        )
            .into_response();
    };
    state.scheduler.remove_job(&params.job_id).await;
    job.enabled = req.enabled;
    state.scheduler.add_job(job).await;
    Json(json!({ "id": params.job_id, "enabled": req.enabled, "status": "updated" }))
        .into_response()
}

/// `DELETE /api/v1/scheduler/jobs/{job_id}` — delete a scheduled job.
pub async fn delete_scheduler_job(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<JobIdPath>,
) -> impl IntoResponse {
    if state.scheduler.remove_job(&params.job_id).await {
        Json(json!({ "id": params.job_id, "status": "deleted" })).into_response()
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("job '{}' not found", params.job_id) })),
        )
            .into_response()
    }
}

/// `GET /api/v1/scheduler/jobs/{job_id}/history` — execution history for a job.
pub async fn scheduler_job_history(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<JobIdPath>,
) -> impl IntoResponse {
    let history = state.scheduler.job_history(&params.job_id).await;
    Json(json!({ "job_id": params.job_id, "history": history, "count": history.len() }))
}

// ─── Lifecycle endpoints ──────────────────────────────────────────────────────

/// Resource status for API responses.
#[derive(Debug, Serialize)]
pub struct LifecycleResourceStatus {
    pub id: String,
    pub resource_type: String,
    pub state: String,
    pub substate: Option<String>,
    pub progress: Option<f32>,
    pub created_at: String,
    pub recovery_attempts: u32,
    pub escalation_tier: u8,
}

impl From<crate::lifecycle::ResourceInstance> for LifecycleResourceStatus {
    fn from(instance: crate::lifecycle::ResourceInstance) -> Self {
        let (state, substate, progress) = match &instance.state {
            crate::lifecycle::ResourceState::Idle => ("idle".to_string(), None, None),
            crate::lifecycle::ResourceState::Running {
                substate, progress, ..
            } => ("running".to_string(), Some(substate.clone()), *progress),
            crate::lifecycle::ResourceState::Stuck { .. } => ("stuck".to_string(), None, None),
            crate::lifecycle::ResourceState::Recovering { .. } => {
                ("recovering".to_string(), None, None)
            }
            crate::lifecycle::ResourceState::Completed { .. } => {
                ("completed".to_string(), None, None)
            }
            crate::lifecycle::ResourceState::Failed { .. } => ("failed".to_string(), None, None),
        };

        Self {
            id: instance.id.to_string(),
            resource_type: instance.resource_type.to_string(),
            state,
            substate,
            progress,
            created_at: instance.created_at.to_rfc3339(),
            recovery_attempts: instance.recovery_attempts,
            escalation_tier: instance.current_escalation_tier,
        }
    }
}

/// `GET /api/v1/lifecycle` — list all tracked resources.
pub async fn list_lifecycle_resources(State(state): State<GatewayState>) -> impl IntoResponse {
    let resources = state.lifecycle.get_all_resources().await;
    let statuses: Vec<LifecycleResourceStatus> = resources
        .into_iter()
        .map(LifecycleResourceStatus::from)
        .collect();
    Json(json!({ "resources": statuses, "count": statuses.len() }))
}

#[derive(Debug, Deserialize)]
pub struct ResourceIdPath {
    pub id: String,
}

/// `GET /api/v1/lifecycle/{id}` — get a specific resource.
pub async fn get_lifecycle_resource(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ResourceIdPath>,
) -> impl IntoResponse {
    let id = match ResourceId::parse(&params.id) {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Invalid resource ID: {}", e) })),
            )
                .into_response();
        }
    };

    match state.lifecycle.get_resource(&id).await {
        Some(instance) => {
            Json(json!({ "resource": LifecycleResourceStatus::from(instance) })).into_response()
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Resource '{}' not found", params.id) })),
        )
            .into_response(),
    }
}

/// `POST /api/v1/lifecycle/{id}/stop` — gracefully stop a resource.
pub async fn stop_lifecycle_resource(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ResourceIdPath>,
) -> impl IntoResponse {
    let id = match ResourceId::parse(&params.id) {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Invalid resource ID: {}", e) })),
            )
                .into_response();
        }
    };

    match state.lifecycle.stop_resource(&id).await {
        Ok(()) => Json(json!({ "id": params.id, "status": "stopped" })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// `POST /api/v1/lifecycle/{id}/kill` — force kill a resource.
pub async fn kill_lifecycle_resource(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ResourceIdPath>,
) -> impl IntoResponse {
    let id = match ResourceId::parse(&params.id) {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Invalid resource ID: {}", e) })),
            )
                .into_response();
        }
    };

    match state.lifecycle.kill_resource(&id).await {
        Ok(()) => Json(json!({ "id": params.id, "status": "killed" })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// `POST /api/v1/lifecycle/{id}/retry` — retry a stuck resource.
pub async fn retry_lifecycle_resource(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ResourceIdPath>,
) -> impl IntoResponse {
    let id = match ResourceId::parse(&params.id) {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": format!("Invalid resource ID: {}", e) })),
            )
                .into_response();
        }
    };

    match state.lifecycle.recover_resource(&id).await {
        Ok(_result) => Json(json!({ "id": params.id, "status": "retrying" })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
