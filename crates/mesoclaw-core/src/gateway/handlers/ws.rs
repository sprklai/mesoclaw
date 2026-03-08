use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::warn;

use crate::ai::adapter::{ToolCallEvent, ToolCallPhase};
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
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error { error: String },
}

pub async fn ws_chat(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn send_outbound(socket: &mut WebSocket, msg: &WsOutbound) {
    let json = serde_json::to_string(msg).unwrap();
    let _ = socket.send(Message::Text(json.into())).await;
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

        // Create per-request broadcast channel for tool events
        let (tool_tx, mut tool_rx) = broadcast::channel::<ToolCallEvent>(32);

        let agent = match resolve_agent(request.model.as_deref(), &state, Some(tool_tx)).await {
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

        // Spawn agent work in background
        let prompt = request.prompt.clone();
        let (result_tx, mut result_rx) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let result = agent.prompt(&prompt).await;
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
                                let msg = state
                                    .session_manager
                                    .append_message(sid, "assistant", &response)
                                    .await;

                                if let Ok(msg) = msg && !tool_events.is_empty() {
                                    let _ = state
                                        .session_manager
                                        .store_tool_calls(&msg.id, sid, &tool_events)
                                        .await;
                                }
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
