use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{info, warn};

use crate::ai::adapter::{ToolCallEvent, ToolCallPhase};
use crate::ai::context::ContextEngine;
use crate::ai::resolve_agent;
use crate::gateway::state::AppState;

#[derive(Debug, Deserialize)]
struct WsRequest {
    prompt: String,
    session_id: Option<String>,
    model: Option<String>,
}

/// Tagged enum for all outbound WebSocket messages.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(crate) enum WsOutbound {
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "tool_call")]
    ToolCall {
        call_id: String,
        tool_name: String,
        args: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        call_id: String,
        tool_name: String,
        output: String,
        success: bool,
        duration_ms: u64,
    },
    #[serde(rename = "notification")]
    Notification {
        event_type: String,
        job_id: String,
        job_name: String,
        message: Option<String>,
        status: Option<String>,
        error: Option<String>,
    },
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error { error: String },
}

pub async fn ws_notifications(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_notifications(socket, state))
}

async fn handle_notifications(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.event_bus.subscribe();

    loop {
        tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(crate::event_bus::AppEvent::SchedulerNotification { job_id, job_name, message }) => {
                        let outbound = WsOutbound::Notification {
                            event_type: "scheduler_notification".into(),
                            job_id,
                            job_name,
                            message: Some(message),
                            status: None,
                            error: None,
                        };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::SchedulerJobCompleted { job_id, job_name, status, error }) => {
                        let outbound = WsOutbound::Notification {
                            event_type: "scheduler_job_completed".into(),
                            job_id,
                            job_name,
                            message: None,
                            status: Some(status),
                            error,
                        };
                        if let Ok(json) = serde_json::to_string(&outbound)
                            && socket.send(Message::Text(json.into())).await.is_err()
                        {
                            break;
                        }
                    }
                    Ok(crate::event_bus::AppEvent::Shutdown) => {
                        break;
                    }
                    Ok(_) => {
                        // Ignore other events on this endpoint
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("notification WS lagged by {n} messages");
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

pub async fn ws_chat(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn send_outbound(socket: &mut WebSocket, msg: &WsOutbound) {
    if let Ok(json) = serde_json::to_string(msg) {
        let _ = socket.send(Message::Text(json.into())).await;
    }
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    while let Some(Ok(msg)) = socket.recv().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };

        let request: WsRequest = match serde_json::from_str(&text) {
            Ok(r) => r,
            Err(e) => {
                send_outbound(
                    &mut socket,
                    &WsOutbound::Error {
                        error: format!("invalid JSON: {e}"),
                    },
                )
                .await;
                continue;
            }
        };

        // Build context: history + augmented preamble via ContextBuilder
        let (history, preamble) = match state
            .context_builder
            .build(request.session_id.as_deref(), &request.prompt)
            .await
        {
            Ok(ctx) => ctx,
            Err(e) => {
                send_outbound(
                    &mut socket,
                    &WsOutbound::Error {
                        error: format!("context build failed: {e}"),
                    },
                )
                .await;
                continue;
            }
        };

        // Also compose ContextEngine preamble for boot context / cached summaries
        let ctx_enabled = state
            .context_injection_enabled
            .load(std::sync::atomic::Ordering::Relaxed);
        let context_engine =
            ContextEngine::new(state.db.clone(), state.config.clone(), ctx_enabled);
        let (message_count, last_message_at, summary) = if let Some(ref sid) = request.session_id {
            state
                .session_manager
                .get_context_info(sid)
                .await
                .unwrap_or((0, None, None))
        } else {
            (0, None, None)
        };
        let level = context_engine.determine_context_level(
            message_count,
            last_message_at.as_ref(),
            summary.is_some(),
            false,
        );
        let model_display = request.model.as_deref().unwrap_or("default");
        let engine_preamble = match context_engine
            .compose(
                &level,
                &state.boot_context,
                model_display,
                request.session_id.as_deref(),
                summary.as_deref(),
            )
            .await
        {
            Ok(p) => p,
            Err(e) => {
                send_outbound(
                    &mut socket,
                    &WsOutbound::Error {
                        error: format!("context compose failed: {e}"),
                    },
                )
                .await;
                continue;
            }
        };

        // Merge preambles: ContextBuilder (identity + memory + user) + ContextEngine (boot + summaries)
        let merged_preamble = format!("{preamble}\n\n{engine_preamble}");
        info!(
            "WS chat: session={}, history={} msgs, preamble={}B, prompt='{}'",
            request.session_id.as_deref().unwrap_or("none"),
            history.len(),
            merged_preamble.len(),
            &request.prompt[..request.prompt.len().min(80)]
        );
        for (i, msg) in history.iter().enumerate() {
            let preview = match msg {
                rig::message::Message::User { content, .. } => {
                    format!("user: {:?}", content)
                }
                rig::message::Message::Assistant { content, .. } => {
                    format!("assistant: {:?}", content)
                }
            };
            info!("  history[{i}] {}", &preview[..preview.len().min(120)]);
        }

        // Create per-request broadcast channel for tool events
        let (tool_tx, mut tool_rx) = broadcast::channel::<ToolCallEvent>(32);

        let agent = match resolve_agent(
            request.model.as_deref(),
            &state,
            Some(tool_tx),
            Some(&merged_preamble),
        )
        .await
        {
            Ok(a) => a,
            Err(e) => {
                send_outbound(
                    &mut socket,
                    &WsOutbound::Error {
                        error: e.to_string(),
                    },
                )
                .await;
                continue;
            }
        };

        // Note: user message is stored by the frontend via POST /sessions/{id}/messages
        // before the WS stream starts. Do not duplicate here.

        // Spawn agent work in background with reasoning engine
        let prompt = request.prompt.clone();
        let reasoning_engine = state.reasoning_engine.clone();
        let (result_tx, mut result_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let result = reasoning_engine
                .chat(&agent, &prompt, history)
                .await
                .map(|r| r.response);
            let _ = result_tx.send(result);
        });

        // Collect tool events for DB persistence
        let mut tool_events = Vec::new();

        // Concurrently forward tool events and wait for agent result
        loop {
            tokio::select! {
                event = tool_rx.recv() => {
                    match event {
                        Ok(evt) => {
                            let outbound = match &evt.phase {
                                ToolCallPhase::Started { args } => WsOutbound::ToolCall {
                                    call_id: evt.call_id.clone(),
                                    tool_name: evt.tool_name.clone(),
                                    args: args.clone(),
                                },
                                ToolCallPhase::Completed { output, success, duration_ms } => WsOutbound::ToolResult {
                                    call_id: evt.call_id.clone(),
                                    tool_name: evt.tool_name.clone(),
                                    output: output.clone(),
                                    success: *success,
                                    duration_ms: *duration_ms,
                                },
                            };
                            send_outbound(&mut socket, &outbound).await;
                            tool_events.push(evt);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            // All senders dropped — agent is done, wait for result
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("tool event receiver lagged by {n} messages");
                        }
                    }
                }
                result = &mut result_rx => {
                    // Drain any remaining tool events that arrived before/during result
                    while let Ok(evt) = tool_rx.try_recv() {
                        let outbound = match &evt.phase {
                            ToolCallPhase::Started { args } => WsOutbound::ToolCall {
                                call_id: evt.call_id.clone(),
                                tool_name: evt.tool_name.clone(),
                                args: args.clone(),
                            },
                            ToolCallPhase::Completed { output, success, duration_ms } => WsOutbound::ToolResult {
                                call_id: evt.call_id.clone(),
                                tool_name: evt.tool_name.clone(),
                                output: output.clone(),
                                success: *success,
                                duration_ms: *duration_ms,
                            },
                        };
                        send_outbound(&mut socket, &outbound).await;
                        tool_events.push(evt);
                    }

                    match result {
                        Ok(Ok(response)) => {
                            send_outbound(&mut socket, &WsOutbound::Text { content: response.clone() }).await;

                            // Store assistant response and tool calls
                            if let Some(ref sid) = request.session_id {
                                info!(
                                    "WS: storing assistant response for session={sid}, len={}",
                                    response.len()
                                );
                                let msg = state
                                    .session_manager
                                    .append_message(sid, "assistant", &response)
                                    .await;

                                match &msg {
                                    Ok(m) => info!(
                                        "WS: assistant message stored OK: id={}, session={}",
                                        m.id, m.session_id
                                    ),
                                    Err(e) => warn!(
                                        "WS: FAILED to store assistant message for session={sid}: {e}"
                                    ),
                                }

                                if let Ok(msg) = msg && !tool_events.is_empty() {
                                    let _ = state
                                        .session_manager
                                        .store_tool_calls(&msg.id, sid, &tool_events)
                                        .await;
                                }

                                // Auto-extract facts from the conversation
                                let _ = state
                                    .context_builder
                                    .extract_facts(&request.prompt, &response, Some(sid))
                                    .await;
                            }

                            send_outbound(&mut socket, &WsOutbound::Done).await;
                        }
                        Ok(Err(e)) => {
                            send_outbound(&mut socket, &WsOutbound::Error { error: e.to_string() }).await;
                        }
                        Err(_) => {
                            send_outbound(&mut socket, &WsOutbound::Error { error: "agent task cancelled".into() }).await;
                        }
                    }
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures::{SinkExt, StreamExt};
    use serde_json::json;
    use tokio_tungstenite::tungstenite;

    use super::*;
    use crate::gateway::routes::build_router;
    use crate::gateway::state::AppState;

    async fn test_state() -> (tempfile::TempDir, Arc<AppState>) {
        crate::gateway::handlers::tests::test_state().await
    }

    /// Spawn an axum server on a random port and return the port number.
    async fn spawn_server(state: Arc<AppState>) -> u16 {
        let router = build_router(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        port
    }

    // 8.6.1.15 — WsOutbound::Notification serializes correctly
    #[test]
    fn ws_outbound_notification_serializes() {
        let msg = WsOutbound::Notification {
            event_type: "scheduler_notification".into(),
            job_id: "j1".into(),
            job_name: "test".into(),
            message: Some("hello".into()),
            status: None,
            error: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "notification");
        assert_eq!(json["event_type"], "scheduler_notification");
        assert_eq!(json["job_id"], "j1");
        assert_eq!(json["message"], "hello");
    }

    // 8.6.1.16 — WS notifications endpoint upgrade succeeds
    #[tokio::test]
    async fn ws_notifications_upgrade_succeeds() {
        let (_dir, state) = test_state().await;
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/notifications");
        let result = tokio_tungstenite::connect_async(&url).await;
        assert!(result.is_ok(), "Notification WS upgrade should succeed");
    }

    // 8.6.1.17 — WS notifications forwards scheduler events
    #[tokio::test]
    async fn ws_notifications_forwards_events() {
        let (_dir, state) = test_state().await;
        let bus = state.event_bus.clone();
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/notifications");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        // Give the WS handler time to subscribe
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Publish a scheduler notification event
        bus.publish(crate::event_bus::AppEvent::SchedulerNotification {
            job_id: "j1".into(),
            job_name: "test_job".into(),
            message: "hello from scheduler".into(),
        })
        .unwrap();

        // Read the forwarded message
        let resp = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next()).await;

        assert!(resp.is_ok(), "Should receive notification within timeout");
        let msg = resp.unwrap().unwrap().unwrap();
        let text = msg.into_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "notification");
        assert_eq!(parsed["event_type"], "scheduler_notification");
        assert_eq!(parsed["job_id"], "j1");
        assert_eq!(parsed["message"], "hello from scheduler");
    }

    // TV.11 — WsOutbound::Text serializes to {"type":"text","content":"..."}
    #[test]
    fn ws_outbound_text_serializes() {
        let msg = WsOutbound::Text {
            content: "hello".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["content"], "hello");
    }

    // TV.12 — WsOutbound::ToolCall serializes with call_id, tool_name, args
    #[test]
    fn ws_outbound_tool_call_serializes() {
        let msg = WsOutbound::ToolCall {
            call_id: "abc".into(),
            tool_name: "WebSearch".into(),
            args: json!({"query": "rust"}),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "tool_call");
        assert_eq!(json["call_id"], "abc");
        assert_eq!(json["tool_name"], "WebSearch");
        assert_eq!(json["args"]["query"], "rust");
    }

    // TV.13 — WsOutbound::ToolResult serializes with all fields
    #[test]
    fn ws_outbound_tool_result_serializes() {
        let msg = WsOutbound::ToolResult {
            call_id: "abc".into(),
            tool_name: "WebSearch".into(),
            output: "results".into(),
            success: true,
            duration_ms: 150,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["call_id"], "abc");
        assert_eq!(json["tool_name"], "WebSearch");
        assert_eq!(json["output"], "results");
        assert_eq!(json["success"], true);
        assert_eq!(json["duration_ms"], 150);
    }

    // TV.14 — WsOutbound::Done serializes to {"type":"done"}
    #[test]
    fn ws_outbound_done_serializes() {
        let msg = WsOutbound::Done;
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "done");
    }

    // TV.15 — WsOutbound::Error serializes with error field
    #[test]
    fn ws_outbound_error_serializes() {
        let msg = WsOutbound::Error {
            error: "oops".into(),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["error"], "oops");
    }

    // TV.16 — WS upgrade still succeeds
    #[tokio::test]
    async fn ws_upgrade_succeeds() {
        let (_dir, state) = test_state().await;
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/chat");
        let result = tokio_tungstenite::connect_async(&url).await;
        assert!(result.is_ok(), "WebSocket upgrade should succeed");
    }

    // TV.17 — WS invalid JSON still returns error
    #[tokio::test]
    async fn ws_invalid_json_returns_error() {
        let (_dir, state) = test_state().await;
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/chat");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        ws.send(tungstenite::Message::Text("not json".into()))
            .await
            .unwrap();

        let resp = ws.next().await.unwrap().unwrap();
        let text = resp.into_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
        assert!(parsed["error"].as_str().unwrap().contains("invalid JSON"));
    }

    // 4.2.3 — WS no agent returns error
    #[tokio::test]
    async fn ws_no_agent_returns_error() {
        let (_dir, state) = test_state().await;
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/chat");
        let (mut ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();

        let msg = serde_json::json!({"prompt": "hello"}).to_string();
        ws.send(tungstenite::Message::Text(msg.into()))
            .await
            .unwrap();

        let resp = ws.next().await.unwrap().unwrap();
        let text = resp.into_text().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["type"], "error");
        assert!(
            parsed["error"]
                .as_str()
                .unwrap()
                .contains("no agent configured")
        );
    }
}
