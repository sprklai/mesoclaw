use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::{
    agent::session_router::{SessionKey, SessionRouter},
    database::DbPool,
    database::models::ai_provider::AIProviderData,
    database::schema::ai_providers,
    event_bus::EventBus,
    identity::IdentityLoader,
    modules::ModuleRegistry,
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

    Json(json!({ "status": "ok", "providers": providers, "count": providers.len() })).into_response()
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
#[derive(serde::Deserialize)]
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
        Some(_) => Json(json!({ "id": params.id, "healthy": true, "registered": true })),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({ "id": params.id, "healthy": false, "error": "module not found" })),
        )
            .into_response(),
    }
}

/// Start a service-type module.
///
/// Full start/stop lifecycle requires `SidecarService` management, which is
/// wired in Phase 4.4.  Returns 501 until that wiring is complete.
#[tracing::instrument(name = "gateway.start_module", skip(state), fields(id = %params.id))]
pub async fn start_module(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ModuleId>,
) -> impl IntoResponse {
    if state.modules.get(&params.id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "id": params.id, "error": "module not found" })),
        );
    }
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(
            json!({ "id": params.id, "error": "module lifecycle (start/stop) requires SidecarService wiring" }),
        ),
    )
}

/// Stop a running service-type module.
#[tracing::instrument(name = "gateway.stop_module", skip(state), fields(id = %params.id))]
pub async fn stop_module(
    State(state): State<GatewayState>,
    axum::extract::Path(params): axum::extract::Path<ModuleId>,
) -> impl IntoResponse {
    if state.modules.get(&params.id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "id": params.id, "error": "module not found" })),
        );
    }
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(
            json!({ "id": params.id, "error": "module lifecycle (start/stop) requires SidecarService wiring" }),
        ),
    )
}

/// Reload (re-discover) all modules from disk.
///
/// ModuleRegistry::discover() requires a mutable ToolRegistry and rebuilds
/// the map from scratch; a live reload while tools are executing is unsafe
/// without quiescing.  Returns 501 until a safe reload protocol is designed.
pub async fn reload_modules(State(_state): State<GatewayState>) -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(
            json!({ "error": "hot module reload not yet safe; restart the daemon to re-discover modules" }),
        ),
    )
}
