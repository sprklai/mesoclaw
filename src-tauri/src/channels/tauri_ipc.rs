//! `TauriIpcChannel` — wraps the Tauri event system as a [`Channel`].
//!
//! Inbound messages arrive via the EventBus (`AppEvent::AgentComplete` etc.);
//! outbound messages are emitted back through the `EventBus`.  This channel
//! is the **default** channel for desktop users interacting through the Tauri
//! frontend.

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::event_bus::{AppEvent, EventBus, EventFilter, EventType};

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
        // Subscribe to ChannelMessage events on the bus.  Only messages whose
        // `channel` field is `"tauri"` or `"tauri-ipc"` are forwarded — other
        // channels (Telegram, webhook, etc.) are handled by their own Channel
        // implementations and the channel-agent bridge in lib.rs.
        let mut rx = self
            .bus
            .subscribe_filtered(EventFilter::new(vec![EventType::ChannelMessage]));

        loop {
            match rx.recv().await {
                Ok(event) => {
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
    match event {
        // Forward ChannelMessage events that originated from the Tauri
        // desktop frontend.  The boot sequence in lib.rs publishes these
        // when inbound channel messages arrive.
        AppEvent::ChannelMessage {
            channel,
            from,
            content,
        } if channel == "tauri" || channel == "tauri-ipc" || channel == "tauri_ipc" => {
            let mut msg = ChannelMessage::new("tauri-ipc", content);
            if !from.is_empty() {
                msg = msg.with_sender(from);
            }
            Some(msg)
        }
        // All other events (including AgentComplete, which is an *output*
        // event emitted by send()) are ignored to prevent feedback loops.
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
