use std::sync::Arc;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use crate::channels::message::ChannelMessage;
use crate::gateway::state::AppState;

#[derive(Serialize, Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ChannelInfo {
    pub name: String,
    pub status: String,
}

#[derive(Serialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct ChannelHealthResponse {
    pub name: String,
    pub healthy: bool,
}

#[derive(Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct SendMessageRequest {
    pub content: String,
    pub recipient: Option<String>,
}

/// GET /channels -- list registered channels with status
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/channels", tag = "Channels",
    responses((status = 200, description = "List of registered channels", body = Vec<ChannelInfo>))
))]
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
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/channels/{name}/status", tag = "Channels",
    params(("name" = String, Path, description = "Channel name")),
    responses(
        (status = 200, description = "Channel status", body = ChannelInfo),
        (status = 404, description = "Channel not found")
    )
))]
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
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/channels/{name}/send", tag = "Channels",
    params(("name" = String, Path, description = "Channel name")),
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Message sent"),
        (status = 500, description = "Send failed")
    )
))]
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
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/channels/{name}/connect", tag = "Channels",
    params(("name" = String, Path, description = "Channel name")),
    responses(
        (status = 200, description = "Channel connected", body = ChannelInfo),
        (status = 400, description = "Unknown channel or missing credentials")
    )
))]
pub async fn connect_channel(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ChannelInfo>, (StatusCode, String)> {
    let channel: Arc<dyn crate::channels::traits::Channel> = match name.as_str() {
        #[cfg(feature = "channels-telegram")]
        "telegram" => {
            let token = state
                .credentials
                .get("channel:telegram:token")
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        "Telegram bot token not configured".to_string(),
                    )
                })?;

            let mut tg_config = crate::channels::telegram::config::TelegramConfig::from_app_config(
                &state.config.load(),
            );

            // Parse allowed_chat_ids from credentials if stored
            if let Ok(Some(ids_str)) = state
                .credentials
                .get("channel:telegram:allowed_chat_ids")
                .await
            {
                tg_config.allowed_chat_ids = ids_str
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i64>().ok())
                    .collect();
            }

            // The token is needed by the channel but TelegramChannel reads it
            // from credentials at connect time via the bot API.
            // Store it back so it's available (it already is, but ensure consistency).
            let _ = token; // token already in credential store

            Arc::new(crate::channels::telegram::TelegramChannel::new(
                tg_config,
                state.credentials.clone(),
                state.config.load_full(),
            ))
        }
        #[cfg(feature = "channels-slack")]
        "slack" => {
            state
                .credentials
                .get("channel:slack:bot_token")
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        "Slack bot token not configured".to_string(),
                    )
                })?;

            Arc::new(crate::channels::slack::SlackChannel::new(
                state.credentials.clone(),
            ))
        }
        #[cfg(feature = "channels-discord")]
        "discord" => {
            state
                .credentials
                .get("channel:discord:token")
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .ok_or_else(|| {
                    (
                        StatusCode::BAD_REQUEST,
                        "Discord bot token not configured".to_string(),
                    )
                })?;

            let dc_config = crate::channels::discord::config::DiscordConfig::from_app_config(
                &state.config.load(),
            );
            Arc::new(crate::channels::discord::DiscordChannel::new(
                dc_config,
                state.credentials.clone(),
            ))
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Unknown or unsupported channel: {name}"),
            ));
        }
    };

    state
        .channel_registry
        .register_or_replace(channel.clone())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Err(e) = channel.connect().await {
        tracing::warn!("Channel {name} connect failed: {e}");
    } else {
        // Spawn listen task after successful connect
        #[cfg(feature = "gateway")]
        if let Some(ref router) = state.channel_router {
            let tx = router.sender();
            let ch = channel.clone();
            let ch_name = name.clone();
            tokio::spawn(async move {
                if let Err(e) = ch.listen(tx).await {
                    tracing::error!("Channel {ch_name} listen failed: {e}");
                }
            });
        }
    }

    let status = state
        .channel_registry
        .status(&name)
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".into());

    Ok(Json(ChannelInfo { name, status }))
}

/// POST /channels/:name/disconnect -- disconnect channel
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/channels/{name}/disconnect", tag = "Channels",
    params(("name" = String, Path, description = "Channel name")),
    responses(
        (status = 200, description = "Channel disconnected"),
        (status = 404, description = "Channel not found")
    )
))]
pub async fn disconnect_channel(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let channel = state
        .channel_registry
        .get_channel(&name)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Channel not found: {name}")))?;

    if let Err(e) = channel.disconnect().await {
        tracing::warn!("Channel {name} disconnect failed: {e}");
    }

    state.channel_registry.unregister(&name);
    Ok(StatusCode::OK)
}

/// Query params for listing channel sessions.
#[derive(Deserialize)]
pub struct ChannelSessionsQuery {
    pub source: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Query params for paginated messages.
#[derive(Deserialize)]
pub struct ChannelMessagesQuery {
    pub limit: Option<usize>,
    pub before: Option<String>,
}

/// GET /channels/sessions — list channel conversations
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/channels/sessions", tag = "Channels",
    params(
        ("source" = Option<String>, Query, description = "Filter by channel source"),
        ("limit" = Option<usize>, Query, description = "Page size"),
        ("offset" = Option<usize>, Query, description = "Offset")
    ),
    responses((status = 200, description = "Channel sessions"))
))]
pub async fn list_channel_sessions(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(query): axum::extract::Query<ChannelSessionsQuery>,
) -> Json<Vec<crate::ai::session::SessionSummary>> {
    let limit = query
        .limit
        .unwrap_or(state.config.load().inbox_sessions_page_size);
    let offset = query.offset.unwrap_or(0);
    let sessions = state
        .session_manager
        .list_channel_sessions(query.source.as_deref(), limit, offset)
        .await
        .unwrap_or_default();
    Json(sessions)
}

/// GET /channels/sessions/:id/messages — paginated messages for a channel conversation
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/channels/sessions/{id}/messages", tag = "Channels",
    params(
        ("id" = String, Path, description = "Session ID"),
        ("limit" = Option<usize>, Query, description = "Page size"),
        ("before" = Option<String>, Query, description = "Cursor for pagination")
    ),
    responses(
        (status = 200, description = "Paginated messages"),
        (status = 404, description = "Session not found")
    )
))]
pub async fn list_channel_messages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<ChannelMessagesQuery>,
) -> Result<Json<Vec<crate::ai::session::Message>>, StatusCode> {
    let limit = query.limit.unwrap_or(state.config.load().inbox_page_size);
    let messages = state
        .session_manager
        .get_messages_paginated(&id, limit, query.before.as_deref())
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    Ok(Json(messages))
}

/// Incoming webhook message body.
#[derive(Deserialize)]
#[cfg_attr(feature = "api-docs", derive(utoipa::ToSchema))]
pub struct WebhookMessageRequest {
    pub content: String,
    pub sender: Option<String>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

/// POST /channels/:name/message -- webhook: inject message into router pipeline
#[cfg_attr(feature = "api-docs", utoipa::path(
    post, path = "/channels/{name}/message", tag = "Channels",
    params(("name" = String, Path, description = "Channel name")),
    responses(
        (status = 202, description = "Message accepted"),
        (status = 404, description = "Channel not found")
    )
))]
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
#[cfg_attr(feature = "api-docs", utoipa::path(
    get, path = "/channels/{name}/health", tag = "Channels",
    params(("name" = String, Path, description = "Channel name")),
    responses(
        (status = 200, description = "Channel health", body = ChannelHealthResponse),
        (status = 404, description = "Channel not found")
    )
))]
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

    // IN.15 — list_channel_sessions returns empty when no channel sessions
    #[tokio::test]
    async fn list_channel_sessions_empty() {
        let (_dir, state) = test_state().await;
        let app = Router::new()
            .route("/channels/sessions", get(list_channel_sessions))
            .with_state(state);

        let req = Request::get("/channels/sessions")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert!(sessions.is_empty());
    }

    // IN.16 — list_channel_sessions returns channel sessions only
    #[tokio::test]
    async fn list_channel_sessions_returns_channel_only() {
        let (_dir, state) = test_state().await;

        // Create a web session and a channel session
        state
            .session_manager
            .create_session("Web Chat")
            .await
            .unwrap();
        state
            .session_manager
            .create_session_with_channel_key("TG #123", "telegram", "telegram:123")
            .await
            .unwrap();

        let app = Router::new()
            .route("/channels/sessions", get(list_channel_sessions))
            .with_state(state);

        let req = Request::get("/channels/sessions")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let sessions: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0]["source"], "telegram");
    }

    // IN.17 — list_channel_messages with pagination
    #[tokio::test]
    async fn list_channel_messages_paginated() {
        let (_dir, state) = test_state().await;
        let session = state
            .session_manager
            .create_session_with_channel_key("TG #1", "telegram", "telegram:1")
            .await
            .unwrap();

        for i in 0..5 {
            state
                .session_manager
                .append_message(&session.id, "user", &format!("msg {i}"))
                .await
                .unwrap();
        }

        let app = Router::new()
            .route(
                "/channels/sessions/{id}/messages",
                get(list_channel_messages),
            )
            .with_state(state);

        let req = Request::get(&format!(
            "/channels/sessions/{}/messages?limit=3",
            session.id
        ))
        .body(Body::empty())
        .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let messages: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(messages.len(), 3);
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
