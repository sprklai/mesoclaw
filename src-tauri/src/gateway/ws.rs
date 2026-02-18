use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};

use crate::event_bus::EventBus;

use super::routes::GatewayState;

/// WebSocket upgrade handler at `GET /api/v1/ws`.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(bus): State<GatewayState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, bus))
}

async fn handle_socket(mut socket: WebSocket, bus: Arc<dyn EventBus>) {
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
            // Accept commands from the client (fire-and-forget for now).
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(_text))) => {
                        // ## TODO: parse incoming commands from client (Phase 2.6)
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
