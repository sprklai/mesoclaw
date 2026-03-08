use serde::{Deserialize, Serialize};

use crate::Result;
use crate::error::MesoError;

/// Maximum frame size in bytes (1 MB).
const MAX_FRAME_SIZE: usize = 1_048_576;

/// Handshake message for external connector processes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorHandshake {
    pub protocol_version: u32,
    pub connector_name: String,
    pub channel_type: String,
}

/// Wire protocol frames for external connector processes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConnectorFrame {
    SendMessage {
        channel: String,
        content: String,
        recipient: Option<String>,
    },
    MessageReceived {
        channel: String,
        sender: Option<String>,
        content: String,
    },
    StatusUpdate {
        channel: String,
        status: String,
    },
    HealthCheck {
        channel: String,
    },
    Error {
        channel: String,
        error: String,
    },
}

impl ConnectorFrame {
    /// Serialize to JSON, validating max frame size.
    pub fn to_json(&self) -> Result<String> {
        let json = serde_json::to_string(self)?;
        if json.len() > MAX_FRAME_SIZE {
            return Err(MesoError::Channel(format!(
                "frame exceeds max size: {} > {MAX_FRAME_SIZE}",
                json.len()
            )));
        }
        Ok(json)
    }

    /// Deserialize from JSON, validating max frame size.
    pub fn from_json(json: &str) -> Result<Self> {
        if json.len() > MAX_FRAME_SIZE {
            return Err(MesoError::Channel(format!(
                "frame exceeds max size: {} > {MAX_FRAME_SIZE}",
                json.len()
            )));
        }
        serde_json::from_str(json).map_err(|e| MesoError::Channel(format!("invalid frame: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_serde() {
        let hs = ConnectorHandshake {
            protocol_version: 1,
            connector_name: "test-connector".into(),
            channel_type: "telegram".into(),
        };
        let json = serde_json::to_string(&hs).unwrap();
        let parsed: ConnectorHandshake = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.protocol_version, 1);
        assert_eq!(parsed.connector_name, "test-connector");
        assert_eq!(parsed.channel_type, "telegram");
    }

    #[test]
    fn frame_send_message() {
        let frame = ConnectorFrame::SendMessage {
            channel: "slack".into(),
            content: "hello".into(),
            recipient: Some("C123".into()),
        };
        let json = frame.to_json().unwrap();
        let parsed = ConnectorFrame::from_json(&json).unwrap();
        assert!(matches!(
            parsed,
            ConnectorFrame::SendMessage {
                channel,
                content,
                recipient,
            } if channel == "slack" && content == "hello" && recipient == Some("C123".into())
        ));
    }

    #[test]
    fn frame_message_received() {
        let frame = ConnectorFrame::MessageReceived {
            channel: "discord".into(),
            sender: Some("user1".into()),
            content: "test message".into(),
        };
        let json = frame.to_json().unwrap();
        let parsed = ConnectorFrame::from_json(&json).unwrap();
        assert!(matches!(
            parsed,
            ConnectorFrame::MessageReceived {
                channel,
                sender,
                content,
            } if channel == "discord" && sender == Some("user1".into()) && content == "test message"
        ));
    }

    #[test]
    fn frame_max_size() {
        let huge_content = "x".repeat(MAX_FRAME_SIZE + 1);
        let frame = ConnectorFrame::SendMessage {
            channel: "test".into(),
            content: huge_content,
            recipient: None,
        };
        let result = frame.to_json();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max size"));
    }

    #[test]
    fn frame_invalid_json() {
        let result = ConnectorFrame::from_json("not valid json{{{");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid frame"));
    }
}
