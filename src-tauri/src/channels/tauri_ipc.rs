//! `TauriIpcChannel` — wraps the Tauri event system as a [`Channel`].
//!
//! Inbound messages arrive via the EventBus (`AppEvent::AgentComplete` etc.);
//! outbound messages are emitted back through the `EventBus`.  This channel
//! is the **default** channel for desktop users interacting through the Tauri
//! frontend.

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::event_bus::{AppEvent, EventBus};

use super::traits::{Channel, ChannelMessage};

use std::sync::Arc;

// ─── TauriIpcChannel ──────────────────────────────────────────────────────────

/// Channels agent output and user input through the Tauri EventBus.
pub struct TauriIpcChannel {
    bus: Arc<dyn EventBus>,
}

impl TauriIpcChannel {
    pub fn new(bus: Arc<dyn EventBus>) -> Self {
        Self { bus }
    }
}

#[async_trait]
impl Channel for TauriIpcChannel {
    fn name(&self) -> &str {
        "tauri-ipc"
    }

    async fn send(&self, message: &str, _recipient: Option<&str>) -> Result<(), String> {
        // Publish the agent response to the event bus so the frontend receives it.
        self.bus.publish(AppEvent::AgentComplete {
            session_id: "tauri".to_string(),
            message: message.to_string(),
        })
    }

    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
        // Subscribe to user-input events on the bus.
        // ## TODO (6.1): Subscribe to AgentTurn events from EventBus once a
        //                user-message event is added to AppEvent (Phase 6+).
        let mut rx = self.bus.subscribe();
        loop {
            match rx.recv().await {
                Ok(event) => {
                    // Convert relevant events into ChannelMessages.
                    if let Some(msg) = event_to_channel_message(event)
                        && tx.send(msg).await.is_err()
                    {
                        // Receiver dropped — stop listening.
                        break;
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    // Some events were missed under backpressure; continue.
                }
            }
        }
        Ok(())
    }

    async fn health_check(&self) -> bool {
        // The IPC channel is healthy as long as the event bus is alive.
        true
    }
}

fn event_to_channel_message(event: AppEvent) -> Option<ChannelMessage> {
    // `AgentComplete` is an *output* event emitted by `send()`.  Re-ingesting
    // it here would create a feedback loop where every agent response is
    // immediately re-queued as a new inbound message.  We return `None` for
    // all events until a dedicated user-turn event is added to `AppEvent`
    // (Phase 6+), at which point this match should handle only that variant.
    //
    // ## TODO (6.1): match on AppEvent::UserTurn (or equivalent) once added.
    match event {
        AppEvent::AgentComplete { .. } => None,
        _ => None,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::TokioBroadcastBus;

    fn make_channel() -> TauriIpcChannel {
        let bus = Arc::new(TokioBroadcastBus::new());
        TauriIpcChannel::new(bus)
    }

    #[test]
    fn name_is_tauri_ipc() {
        let ch = make_channel();
        assert_eq!(ch.name(), "tauri-ipc");
    }

    #[tokio::test]
    async fn health_check_always_true() {
        let ch = make_channel();
        assert!(ch.health_check().await);
    }

    #[tokio::test]
    async fn send_publishes_to_event_bus() {
        let bus = Arc::new(TokioBroadcastBus::new());
        let mut rx = bus.subscribe();
        let ch = TauriIpcChannel::new(bus);

        ch.send("hello", None).await.unwrap();

        // Give the bus a tick.
        let event = rx.try_recv();
        assert!(event.is_ok(), "event should be published on send");
    }

    #[test]
    fn channel_message_new_has_correct_fields() {
        let msg = ChannelMessage::new("tauri-ipc", "test content");
        assert_eq!(msg.channel, "tauri-ipc");
        assert_eq!(msg.content, "test content");
        assert!(msg.sender.is_none());
    }

    #[test]
    fn channel_message_with_sender() {
        let msg = ChannelMessage::new("ch", "body").with_sender("user-123");
        assert_eq!(msg.sender.unwrap(), "user-123");
    }
}
