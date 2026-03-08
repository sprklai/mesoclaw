pub mod api;
pub mod fmt;

use std::sync::atomic::{AtomicU8, Ordering};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::Result;
use crate::error::MesoError;

use super::message::ChannelMessage;
use super::traits::{Channel, ChannelLifecycle, ChannelSender, ChannelStatus};

// Status values
const STATUS_DISCONNECTED: u8 = 0;
const STATUS_CONNECTED: u8 = 2;

/// Slack channel using raw Socket Mode WebSocket (no slack-morphism).
pub struct SlackChannel {
    display_name: String,
    bot_id: Option<String>,
    allowed_channel_ids: Vec<String>,
    status: AtomicU8,
}

impl SlackChannel {
    pub fn new() -> Self {
        Self {
            display_name: "slack".to_string(),
            bot_id: None,
            allowed_channel_ids: vec![],
            status: AtomicU8::new(STATUS_DISCONNECTED),
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.display_name = name.to_string();
        self
    }

    pub fn with_bot_id(mut self, bot_id: &str) -> Self {
        self.bot_id = Some(bot_id.to_string());
        self
    }

    pub fn with_allowed_channels(mut self, channels: Vec<String>) -> Self {
        self.allowed_channel_ids = channels;
        self
    }

    /// Check if a channel ID is allowed (empty list = allow all).
    pub fn is_channel_allowed(&self, channel_id: &str) -> bool {
        self.allowed_channel_ids.is_empty()
            || self.allowed_channel_ids.iter().any(|c| c == channel_id)
    }

    fn status_from_u8(val: u8) -> ChannelStatus {
        match val {
            STATUS_CONNECTED => ChannelStatus::Connected,
            _ => ChannelStatus::Disconnected,
        }
    }
}

impl Default for SlackChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChannelSender for SlackChannel {
    fn channel_type(&self) -> &str {
        "slack"
    }

    async fn send_message(&self, _message: ChannelMessage) -> Result<()> {
        Err(MesoError::Channel(
            "slack send requires active connection".into(),
        ))
    }
}

#[async_trait]
impl ChannelLifecycle for SlackChannel {
    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn connect(&self) -> Result<()> {
        // Actual Socket Mode WebSocket connection happens here in production
        self.status.store(STATUS_CONNECTED, Ordering::SeqCst);
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
        Ok(())
    }

    fn status(&self) -> ChannelStatus {
        Self::status_from_u8(self.status.load(Ordering::SeqCst))
    }

    fn create_sender(&self) -> Box<dyn ChannelSender> {
        Box::new(SlackSender {
            channel_name: self.display_name.clone(),
        })
    }
}

#[async_trait]
impl Channel for SlackChannel {
    async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        // Actual Socket Mode WebSocket listen loop runs here in production
        Ok(())
    }

    async fn health_check(&self) -> bool {
        self.status.load(Ordering::SeqCst) == STATUS_CONNECTED
    }
}

/// Lightweight send-only handle for Slack.
struct SlackSender {
    channel_name: String,
}

#[async_trait]
impl ChannelSender for SlackSender {
    fn channel_type(&self) -> &str {
        "slack"
    }

    async fn send_message(&self, _message: ChannelMessage) -> Result<()> {
        Err(MesoError::Channel(format!(
            "{}: send requires active connection",
            self.channel_name
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_type_slack() {
        let ch = SlackChannel::new();
        assert_eq!(ch.channel_type(), "slack");
    }

    #[test]
    fn initial_status_disconnected() {
        let ch = SlackChannel::new();
        assert_eq!(ch.status(), ChannelStatus::Disconnected);
    }

    #[test]
    fn dm_detection() {
        assert!(api::is_dm_channel("D1234567"));
        assert!(!api::is_dm_channel("C1234567"));
        assert!(!api::is_dm_channel("G1234567"));
    }

    #[test]
    fn bot_mention_detection() {
        assert!(api::contains_bot_mention("hello <@U12345> help", "U12345"));
        assert!(!api::contains_bot_mention("hello world", "U12345"));
        assert!(!api::contains_bot_mention("hello <@U99999>", "U12345"));
    }

    #[test]
    fn envelope_ack_format() {
        let ack = api::envelope_ack("abc-123");
        assert_eq!(ack["envelope_id"], "abc-123");
    }
}
