use axum::{
    Json,
    extract::State,
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::event_bus::EventBus;

pub type GatewayState = Arc<dyn EventBus>;

// ─── Health ───────────────────────────────────────────────────────────────────

pub async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "mesoclaw-daemon" }))
}

// ─── Agent sessions ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub system_prompt: Option<String>,
    pub provider_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub status: String,
}

pub async fn create_session(
    State(_bus): State<GatewayState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    // ## TODO: wire to actual agent session manager (Phase 3)
    let session_id = uuid::Uuid::new_v4().to_string();
    log::info!(
        "Gateway: create_session (provider={:?})",
        req.provider_id
    );
    (
        StatusCode::CREATED,
        Json(SessionResponse {
            session_id,
            status: "created".to_string(),
        }),
    )
}

pub async fn list_sessions(
    State(_bus): State<GatewayState>,
) -> impl IntoResponse {
    // ## TODO: return real sessions from agent session store (Phase 3)
    Json(json!({ "sessions": [] }))
}

// ─── Provider status ──────────────────────────────────────────────────────────

pub async fn provider_status(
    State(_bus): State<GatewayState>,
) -> impl IntoResponse {
    // ## TODO: query real provider health (Phase 3)
    Json(json!({ "providers": [] }))
}

// ─── Module management ────────────────────────────────────────────────────────

/// Summary of a registered sidecar module returned by the list endpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleSummary {
    pub id: String,
    pub module_type: String,
    pub runtime: String,
    pub status: String,
}

/// List all registered sidecar modules and their current status.
///
/// ## TODO: inject `Arc<ModuleRegistry>` via shared state once the module
/// registry is managed by the Tauri / daemon app state (Phase 6).
pub async fn list_modules(
    State(_bus): State<GatewayState>,
) -> impl IntoResponse {
    // ## TODO: query the ModuleRegistry once it is wired into GatewayState.
    Json(json!({ "modules": [] }))
}

/// Path parameters for module-specific routes.
#[derive(serde::Deserialize)]
pub struct ModuleId {
    pub id: String,
}

/// Return health status for a single module.
pub async fn module_health(
    State(_bus): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ModuleId>,
) -> impl IntoResponse {
    // ## TODO: delegate to SidecarModule::health_check() (Phase 6)
    Json(json!({ "id": params.id, "healthy": true, "note": "stub – Phase 6" }))
}

/// Start a service-type module.
pub async fn start_module(
    State(_bus): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ModuleId>,
) -> impl IntoResponse {
    // ## TODO: delegate to ModuleRegistry::start(id) (Phase 6)
    (
        StatusCode::ACCEPTED,
        Json(json!({ "id": params.id, "status": "starting", "note": "stub – Phase 6" })),
    )
}

/// Stop a running service-type module.
pub async fn stop_module(
    State(_bus): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ModuleId>,
) -> impl IntoResponse {
    // ## TODO: delegate to ModuleRegistry::stop(id) (Phase 6)
    (
        StatusCode::ACCEPTED,
        Json(json!({ "id": params.id, "status": "stopping", "note": "stub – Phase 6" })),
    )
}

/// Reload (re-discover) all modules from disk.
pub async fn reload_modules(
    State(_bus): State<GatewayState>,
) -> impl IntoResponse {
    // ## TODO: trigger ModuleRegistry::discover() then update shared state (Phase 6)
    Json(json!({ "status": "reloaded", "count": 0, "note": "stub – Phase 6" }))
}
