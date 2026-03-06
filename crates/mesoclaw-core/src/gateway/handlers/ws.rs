use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::gateway::state::AppState;

#[derive(Debug, Deserialize)]
struct WsRequest {
    prompt: String,
    session_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct WsChunk {
    r#type: String,
    content: Option<String>,
    error: Option<String>,
}

pub async fn ws_chat(
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
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
                let err_chunk = WsChunk {
                    r#type: "error".into(),
                    content: None,
                    error: Some(format!("invalid JSON: {e}")),
                };
                let _ = socket
                    .send(Message::Text(
                        serde_json::to_string(&err_chunk).unwrap().into(),
                    ))
                    .await;
                continue;
            }
        };

        let agent = match state.agent.as_ref() {
            Some(a) => a,
            None => {
                let err_chunk = WsChunk {
                    r#type: "error".into(),
                    content: None,
                    error: Some("no agent configured".into()),
                };
                let _ = socket
                    .send(Message::Text(
                        serde_json::to_string(&err_chunk).unwrap().into(),
                    ))
                    .await;
                continue;
            }
        };

        // Store user message if session provided
        if let Some(ref sid) = request.session_id {
            let _ = state
                .session_manager
                .append_message(sid, "user", &request.prompt)
                .await;
        }

        // For Phase 3, we use prompt() (non-streaming) and send as single chunk + done.
        // True streaming will use rig's stream() in a future iteration.
        match agent.prompt(&request.prompt).await {
            Ok(response) => {
                let chunk = WsChunk {
                    r#type: "text".into(),
                    content: Some(response.clone()),
                    error: None,
                };
                let _ = socket
                    .send(Message::Text(serde_json::to_string(&chunk).unwrap().into()))
                    .await;

                // Store assistant response
                if let Some(ref sid) = request.session_id {
                    let _ = state
                        .session_manager
                        .append_message(sid, "assistant", &response)
                        .await;
                }

                let done = WsChunk {
                    r#type: "done".into(),
                    content: None,
                    error: None,
                };
                let _ = socket
                    .send(Message::Text(serde_json::to_string(&done).unwrap().into()))
                    .await;
            }
            Err(e) => {
                let err_chunk = WsChunk {
                    r#type: "error".into(),
                    content: None,
                    error: Some(e.to_string()),
                };
                let _ = socket
                    .send(Message::Text(
                        serde_json::to_string(&err_chunk).unwrap().into(),
                    ))
                    .await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite;

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

    // 4.2.1 — WS upgrade succeeds
    #[tokio::test]
    async fn ws_upgrade_succeeds() {
        let (_dir, state) = test_state().await;
        let port = spawn_server(state).await;

        let url = format!("ws://127.0.0.1:{port}/ws/chat");
        let result = tokio_tungstenite::connect_async(&url).await;
        assert!(result.is_ok(), "WebSocket upgrade should succeed");
    }

    // 4.2.2 — WS invalid JSON returns error
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
