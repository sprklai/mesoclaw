use std::sync::Arc;

use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    agent::agent_commands::SessionCancelMap,
    event_bus::{AppEvent, EventBus},
};

use super::routes::GatewayState;

/// WebSocket upgrade handler at `GET /api/v1/ws`.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<GatewayState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

// ─── Incoming command types ──────────────────────────────────────────────────

/// Envelope for all WebSocket commands sent by clients.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsCommand {
    /// Route a message to the agent session.
    AgentMessage {
        content: String,
        session_id: Option<String>,
    },
    /// Cancel a running agent session.
    CancelSession { session_id: String },
    /// Ping / keep-alive (no-op, triggers a pong ack).
    Ping,
}

// ─── Socket handler ──────────────────────────────────────────────────────────

async fn handle_socket(mut socket: WebSocket, state: GatewayState) {
    let bus: Arc<dyn EventBus> = state.bus.clone();
    let cancel_map = state.cancel_map.clone();
    let mut rx = bus.subscribe();

    loop {
        tokio::select! {
            // Forward bus events to the client.
            event = rx.recv() => {
                match event {
                    Ok(ev) => {
                        let payload = match serde_json::to_string(&ev) {
                            Ok(s) => s,
                            Err(e) => {
                                log::warn!("ws: failed to serialise event: {e}");
                                continue;
                            }
                        };
                        if socket.send(Message::Text(payload)).await.is_err() {
                            break; // Client disconnected.
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        log::warn!("ws handler lagged, missed {n} events");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            // Parse and dispatch commands from the client.
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        handle_client_command(&text, &bus, &cancel_map, &mut socket).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}

/// Parse a JSON command from the client and emit the appropriate event.
async fn handle_client_command(
    raw: &str,
    bus: &Arc<dyn EventBus>,
    cancel_map: &SessionCancelMap,
    socket: &mut WebSocket,
) {
    let cmd: WsCommand = match serde_json::from_str(raw) {
        Ok(c) => c,
        Err(e) => {
            let err_msg = serde_json::json!({
                "type": "error",
                "error": format!("invalid command: {e}"),
            });
            let _ = socket.send(Message::Text(err_msg.to_string())).await;
            return;
        }
    };

    match cmd {
        WsCommand::AgentMessage {
            content,
            session_id,
        } => {
            let event = AppEvent::ChannelMessage {
                channel: "ws".to_string(),
                from: session_id.unwrap_or_default(),
                content,
            };
            if let Err(e) = bus.publish(event) {
                log::warn!("ws: failed to publish agent_message event: {e}");
            }
        }
        WsCommand::CancelSession { session_id } => {
            // Set the atomic cancel flag for the session so the agent loop
            // exits on its next iteration.
            let flag = cancel_map
                .lock()
                .ok()
                .and_then(|m| m.get(&session_id).cloned());

            let (success, message) = match flag {
                Some(f) => {
                    f.store(true, std::sync::atomic::Ordering::SeqCst);
                    log::info!("ws: cancel signal sent for session {session_id}");
                    (
                        true,
                        format!("cancel signal sent for session '{session_id}'"),
                    )
                }
                None => {
                    log::warn!("ws: cancel requested for unknown session {session_id}");
                    (false, format!("unknown session '{session_id}'"))
                }
            };

            let ack = serde_json::json!({
                "type": "cancel_ack",
                "session_id": session_id,
                "success": success,
                "message": message,
            });
            let _ = socket.send(Message::Text(ack.to_string())).await;
        }
        WsCommand::Ping => {
            let pong = serde_json::json!({ "type": "pong" });
            let _ = socket.send(Message::Text(pong.to_string())).await;
        }
    }
}
