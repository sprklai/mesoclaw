use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A message flowing through a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMessage {
    pub channel: String,
    pub sender: Option<String>,
    pub content: String,
    pub timestamp: String,
    pub metadata: HashMap<String, String>,
}

impl ChannelMessage {
    pub fn new(channel: &str, content: &str) -> Self {
        Self {
            channel: channel.to_string(),
            sender: None,
            content: content.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_sender(mut self, sender: &str) -> Self {
        self.sender = Some(sender.to_string());
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_serde() {
        let msg = ChannelMessage::new("telegram", "hello world");
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChannelMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.channel, "telegram");
        assert_eq!(parsed.content, "hello world");
        assert!(parsed.sender.is_none());
        assert!(!parsed.timestamp.is_empty());
    }

    #[test]
    fn message_metadata() {
        let mut meta = HashMap::new();
        meta.insert("thread_id".into(), "123".into());
        meta.insert("channel_id".into(), "C456".into());

        let msg = ChannelMessage::new("slack", "test")
            .with_sender("user1")
            .with_metadata(meta);

        assert_eq!(msg.sender.as_deref(), Some("user1"));
        assert_eq!(msg.metadata.get("thread_id").unwrap(), "123");
        assert_eq!(msg.metadata.get("channel_id").unwrap(), "C456");
        assert_eq!(msg.metadata.len(), 2);
    }
}
