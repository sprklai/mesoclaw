use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::Result;

use super::message::ChannelMessage;

/// Status of a channel's connection.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ChannelStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error(String),
}

impl fmt::Display for ChannelStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "disconnected"),
            Self::Connecting => write!(f, "connecting"),
            Self::Connected => write!(f, "connected"),
            Self::Reconnecting => write!(f, "reconnecting"),
            Self::Error(e) => write!(f, "error: {e}"),
        }
    }
}

/// Events emitted by channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelEvent {
    Connected {
        channel: String,
    },
    Disconnected {
        channel: String,
        reason: String,
    },
    MessageReceived(ChannelMessage),
    MessageSent {
        channel: String,
        recipient: Option<String>,
    },
    Error {
        channel: String,
        error: String,
    },
}

/// Lifecycle management for a channel (connect/disconnect).
#[async_trait]
pub trait ChannelLifecycle: Send {
    fn display_name(&self) -> &str;
    async fn connect(&self) -> Result<()>;
    async fn disconnect(&self) -> Result<()>;
    fn status(&self) -> ChannelStatus;
    fn create_sender(&self) -> Box<dyn ChannelSender>;
}

/// Send-only handle for a channel, safe to share across tasks via Arc.
#[async_trait]
pub trait ChannelSender: Send + Sync {
    fn channel_type(&self) -> &str;
    async fn send_message(&self, message: ChannelMessage) -> Result<()>;
}

/// Combined channel trait: lifecycle + sender + listen + lifecycle hooks.
#[async_trait]
pub trait Channel: ChannelLifecycle + ChannelSender {
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()>;
    async fn health_check(&self) -> bool;

    /// Called when the agent starts processing a message. Show typing/status.
    async fn on_agent_start(&self, _recipient: Option<&str>) {}

    /// Called when the agent uses a tool. Update status message.
    async fn on_tool_use(&self, _tool_name: &str, _recipient: Option<&str>) {}

    /// Called when the agent completes processing. Cleanup status.
    async fn on_agent_complete(&self, _recipient: Option<&str>) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    // Mock channel for lifecycle hook tests
    struct MockLifecycleChannel;

    #[async_trait]
    impl ChannelSender for MockLifecycleChannel {
        fn channel_type(&self) -> &str {
            "mock"
        }
        async fn send_message(&self, _message: ChannelMessage) -> Result<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl ChannelLifecycle for MockLifecycleChannel {
        fn display_name(&self) -> &str {
            "mock"
        }
        async fn connect(&self) -> Result<()> {
            Ok(())
        }
        async fn disconnect(&self) -> Result<()> {
            Ok(())
        }
        fn status(&self) -> ChannelStatus {
            ChannelStatus::Connected
        }
        fn create_sender(&self) -> Box<dyn ChannelSender> {
            Box::new(MockLifecycleChannel)
        }
    }

    #[async_trait]
    impl Channel for MockLifecycleChannel {
        async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
            Ok(())
        }
        async fn health_check(&self) -> bool {
            true
        }
    }

    // 8.8.10 — Lifecycle hooks called in sequence don't panic
    #[tokio::test]
    async fn lifecycle_hooks_sequence() {
        let ch = MockLifecycleChannel;
        ch.on_agent_start(Some("user1")).await;
        ch.on_tool_use("web_search", Some("user1")).await;
        ch.on_tool_use("shell", Some("user1")).await;
        ch.on_agent_complete(Some("user1")).await;
    }

    // 8.8.11 — Lifecycle hooks with None recipient
    #[tokio::test]
    async fn lifecycle_hooks_none_recipient() {
        let ch = MockLifecycleChannel;
        ch.on_agent_start(None).await;
        ch.on_tool_use("test_tool", None).await;
        ch.on_agent_complete(None).await;
    }

    // CR.21 — Default on_agent_start is no-op (does not panic)
    #[tokio::test]
    async fn default_lifecycle_noop() {
        let ch = MockLifecycleChannel;
        ch.on_agent_start(Some("user1")).await;
        // No panic = pass
    }

    // CR.22 — Default on_tool_use is no-op
    #[tokio::test]
    async fn default_tool_use_noop() {
        let ch = MockLifecycleChannel;
        ch.on_tool_use("web_search", Some("user1")).await;
        // No panic = pass
    }

    // CR.23 — Default on_agent_complete is no-op
    #[tokio::test]
    async fn default_agent_complete_noop() {
        let ch = MockLifecycleChannel;
        ch.on_agent_complete(Some("user1")).await;
        // No panic = pass
    }

    #[test]
    fn status_default_disconnected() {
        let status = ChannelStatus::default();
        assert_eq!(status, ChannelStatus::Disconnected);
    }

    #[test]
    fn status_display() {
        assert_eq!(ChannelStatus::Disconnected.to_string(), "disconnected");
        assert_eq!(ChannelStatus::Connecting.to_string(), "connecting");
        assert_eq!(ChannelStatus::Connected.to_string(), "connected");
        assert_eq!(ChannelStatus::Reconnecting.to_string(), "reconnecting");
        assert_eq!(
            ChannelStatus::Error("timeout".into()).to_string(),
            "error: timeout"
        );
    }

    #[test]
    fn event_serde() {
        let event = ChannelEvent::Connected {
            channel: "telegram".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let parsed: ChannelEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, ChannelEvent::Connected { channel } if channel == "telegram"));

        let event2 = ChannelEvent::Disconnected {
            channel: "slack".into(),
            reason: "timeout".into(),
        };
        let json2 = serde_json::to_string(&event2).unwrap();
        assert!(json2.contains("slack"));

        let msg = ChannelMessage::new("discord", "hello");
        let event3 = ChannelEvent::MessageReceived(msg);
        let json3 = serde_json::to_string(&event3).unwrap();
        assert!(json3.contains("discord"));
    }
}
