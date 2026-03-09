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

/// Incoming webhook message body.
#[derive(Deserialize)]
pub struct WebhookMessageRequest {
    pub content: String,
    pub sender: Option<String>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// POST /channels/:name/message -- webhook: inject message into router pipeline
pub async fn webhook_message(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<WebhookMessageRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Validate channel exists
    if state.channel_registry.status(&name).is_none() {
        return Err((StatusCode::NOT_FOUND, format!("channel not found: {name}")));
    }

    // Build channel message
    let mut msg = ChannelMessage::new(&name, &req.content);
    if let Some(sender) = req.sender {
        msg = msg.with_sender(&sender);
    }
    if !req.metadata.is_empty() {
        msg = msg.with_metadata(req.metadata);
    }

    // Process through the router pipeline directly
    #[cfg(feature = "gateway")]
    {
        let state_clone = state.clone();
        tokio::spawn(async move {
            crate::channels::router::ChannelRouter::handle_message_static(msg, &state_clone).await;
        });
    }

    Ok(StatusCode::ACCEPTED)
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

    // 8.7.13 — webhook endpoint returns 404 for unknown channel
    #[tokio::test]
    async fn webhook_unknown_channel_404() {
        let (_dir, state) = test_state().await;
        let app = Router::new()
            .route(
                "/channels/{name}/message",
                axum::routing::post(webhook_message),
            )
            .with_state(state);

        let body = serde_json::json!({
            "content": "hello",
            "sender": "user1"
        });
        let req = Request::post("/channels/nonexistent/message")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
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

    // 8.7.13 — Webhook message for a registered channel returns 202
    #[tokio::test]
    async fn channel_message_webhook_202() {
        use crate::channels::traits::{
            Channel, ChannelLifecycle, ChannelSender, ChannelStatus as CS,
        };
        use async_trait::async_trait;

        // Minimal mock channel for registry
        struct MockCh;

        #[async_trait]
        impl ChannelSender for MockCh {
            fn channel_type(&self) -> &str {
                "test"
            }
            async fn send_message(&self, _msg: ChannelMessage) -> crate::Result<()> {
                Ok(())
            }
        }

        #[async_trait]
        impl ChannelLifecycle for MockCh {
            fn display_name(&self) -> &str {
                "test"
            }
            async fn connect(&self) -> crate::Result<()> {
                Ok(())
            }
            async fn disconnect(&self) -> crate::Result<()> {
                Ok(())
            }
            fn status(&self) -> CS {
                CS::Connected
            }
            fn create_sender(&self) -> Box<dyn ChannelSender> {
                Box::new(MockCh)
            }
        }

        #[async_trait]
        impl Channel for MockCh {
            async fn listen(
                &self,
                _tx: tokio::sync::mpsc::Sender<ChannelMessage>,
            ) -> crate::Result<()> {
                Ok(())
            }
            async fn health_check(&self) -> bool {
                true
            }
        }

        let (_dir, state) = test_state().await;
        state
            .channel_registry
            .register(std::sync::Arc::new(MockCh))
            .unwrap();

        let app = Router::new()
            .route(
                "/channels/{name}/message",
                axum::routing::post(webhook_message),
            )
            .with_state(state);

        let body = serde_json::json!({
            "content": "hello from webhook",
            "sender": "user1"
        });
        let req = Request::post("/channels/test/message")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
    }

    // 8.7.14 — Webhook message with missing content returns 422 (invalid JSON)
    #[tokio::test]
    async fn channel_message_webhook_invalid_400() {
        let (_dir, state) = test_state().await;

        let app = Router::new()
            .route(
                "/channels/{name}/message",
                axum::routing::post(webhook_message),
            )
            .with_state(state);

        // Send JSON with missing required 'content' field
        let body = serde_json::json!({
            "sender": "user1"
        });
        let req = Request::post("/channels/test/message")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        // Missing required field 'content' causes axum JSON deserialization to return 422
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
