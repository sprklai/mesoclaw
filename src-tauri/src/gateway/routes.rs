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
