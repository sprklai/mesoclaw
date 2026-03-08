use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::Result;

use super::message::ChannelMessage;

/// Status of a channel's connection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelStatus {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
    Error(String),
}

impl Default for ChannelStatus {
    fn default() -> Self {
        Self::Disconnected
    }
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

/// Combined channel trait: lifecycle + sender + listen.
#[async_trait]
pub trait Channel: ChannelLifecycle + ChannelSender {
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()>;
    async fn health_check(&self) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

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
