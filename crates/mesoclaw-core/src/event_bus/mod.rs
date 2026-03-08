use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    SessionCreated { session_id: String },
    SessionDeleted { session_id: String },
    MessageReceived { session_id: String, role: String },
    StreamChunk { session_id: String, content: String },
    StreamDone { session_id: String },
    ToolExecutionStarted { tool_name: String },
    ToolExecutionCompleted { tool_name: String, success: bool },
    ProviderChanged { provider: String, model: String },
    MemoryStored { key: String },
    ConfigUpdated,
    GatewayStarted { port: u16 },
    ChannelConnected { channel: String },
    ChannelDisconnected { channel: String, reason: String },
    ChannelMessageReceived { channel: String, sender: String },
    Shutdown,
}

#[async_trait]
pub trait EventBus: Send + Sync {
    fn publish(&self, event: AppEvent) -> Result<()>;
    fn subscribe(&self) -> broadcast::Receiver<AppEvent>;
}

pub struct TokioBroadcastBus {
    sender: broadcast::Sender<AppEvent>,
}

impl TokioBroadcastBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
}

#[async_trait]
impl EventBus for TokioBroadcastBus {
    fn publish(&self, event: AppEvent) -> Result<()> {
        // Ignore error when there are no subscribers — this is expected during startup
        let _ = self.sender.send(event);
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_and_subscribe() {
        let bus = TokioBroadcastBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::ConfigUpdated).unwrap();

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, AppEvent::ConfigUpdated));
    }

    #[tokio::test]
    async fn multi_subscriber_fanout() {
        let bus = TokioBroadcastBus::new(16);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        bus.publish(AppEvent::Shutdown).unwrap();

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert!(matches!(e1, AppEvent::Shutdown));
        assert!(matches!(e2, AppEvent::Shutdown));
    }

    #[tokio::test]
    async fn channel_connected_event() {
        let bus = TokioBroadcastBus::new(16);
        let mut rx = bus.subscribe();

        bus.publish(AppEvent::ChannelConnected {
            channel: "telegram".into(),
        })
        .unwrap();

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, AppEvent::ChannelConnected { channel } if channel == "telegram"));
    }

    #[tokio::test]
    async fn publish_without_subscribers_is_ok() {
        let bus = TokioBroadcastBus::new(16);
        let result = bus.publish(AppEvent::ConfigUpdated);
        assert!(result.is_ok());
    }
}
