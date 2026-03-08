use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::channels::message::ChannelMessage;
use crate::gateway::state::AppState;

#[derive(Serialize, Deserialize)]
pub struct ChannelInfo {
    pub name: String,
    pub status: String,
}

#[derive(Serialize)]
pub struct ChannelHealthResponse {
    pub name: String,
    pub healthy: bool,
}

#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub recipient: Option<String>,
}

/// GET /channels -- list registered channels with status
pub async fn list_channels(State(state): State<Arc<AppState>>) -> Json<Vec<ChannelInfo>> {
    let registry = state.channel_registry.as_ref();
    let names = registry.list();
    let channels: Vec<ChannelInfo> = names
        .into_iter()
        .map(|name| {
            let status = registry
                .status(&name)
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".into());
            ChannelInfo { name, status }
        })
        .collect();
    Json(channels)
}

/// GET /channels/:name/status -- single channel status
pub async fn channel_status(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ChannelInfo>, StatusCode> {
    let registry = state.channel_registry.as_ref();
    match registry.status(&name) {
        Some(status) => Ok(Json(ChannelInfo {
            name,
            status: status.to_string(),
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// POST /channels/:name/send -- send message via channel
pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let msg = ChannelMessage::new(&name, &req.content);
    state
        .channel_registry
        .send(&name, msg)
        .await
        .map(|_| StatusCode::OK)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// POST /channels/:name/connect -- connect channel
pub async fn connect_channel(
    State(_state): State<Arc<AppState>>,
    Path(_name): Path<String>,
) -> StatusCode {
    // Channel connection is managed at boot time
    StatusCode::OK
}

/// POST /channels/:name/disconnect -- disconnect channel
pub async fn disconnect_channel(
    State(_state): State<Arc<AppState>>,
    Path(_name): Path<String>,
) -> StatusCode {
    StatusCode::OK
}

/// GET /channels/:name/health -- health check
pub async fn health_check(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ChannelHealthResponse>, StatusCode> {
    let registry = state.channel_registry.as_ref();
    if registry.status(&name).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }
    let health = registry.health_all().await;
    let healthy = health.get(&name).copied().unwrap_or(false);
    Ok(Json(ChannelHealthResponse { name, healthy }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::get;
    use tower::ServiceExt;

    async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    fn channel_router(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/channels", get(list_channels))
            .route("/channels/{name}/status", get(channel_status))
            .route("/channels/{name}/health", get(health_check))
            .with_state(state)
    }

    #[tokio::test]
    async fn list_channels_empty() {
        let (_dir, state) = test_state().await;
        let app = channel_router(state);

        let req = Request::get("/channels").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let channels: Vec<ChannelInfo> = serde_json::from_slice(&body).unwrap();
        assert!(channels.is_empty());
    }

    #[tokio::test]
    async fn channel_status_unknown() {
        let (_dir, state) = test_state().await;
        let app = channel_router(state);

        let req = Request::get("/channels/nonexistent/status")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn health_check_unknown() {
        let (_dir, state) = test_state().await;
        let app = channel_router(state);

        let req = Request::get("/channels/nonexistent/health")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }
}
