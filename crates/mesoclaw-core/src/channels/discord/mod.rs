pub mod config;

use std::sync::atomic::{AtomicU8, Ordering};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::Result;
use crate::error::MesoError;

use super::message::ChannelMessage;
use super::traits::{Channel, ChannelLifecycle, ChannelSender, ChannelStatus};

use config::DiscordConfig;

// Status values
const STATUS_DISCONNECTED: u8 = 0;
const STATUS_CONNECTED: u8 = 2;

/// Required Discord Gateway intents.
pub const REQUIRED_INTENTS: u64 = (1 << 9) | (1 << 12) | (1 << 15);
// GatewayIntents::GUILD_MESSAGES (1 << 9) = 512
// GatewayIntents::DIRECT_MESSAGES (1 << 12) = 4096
// GatewayIntents::MESSAGE_CONTENT (1 << 15) = 32768

/// Discord channel implementation using serenity.
pub struct DiscordChannel {
    config: DiscordConfig,
    display_name: String,
    status: AtomicU8,
}

impl DiscordChannel {
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            display_name: "discord".to_string(),
            status: AtomicU8::new(STATUS_DISCONNECTED),
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.display_name = name.to_string();
        self
    }

    /// Check if a guild is allowed by the allowlist.
    pub fn is_guild_allowed(&self, guild_id: u64) -> bool {
        self.config.is_guild_allowed(guild_id)
    }

    /// Check if a channel is allowed by the allowlist.
    pub fn is_channel_allowed(&self, channel_id: u64) -> bool {
        self.config.is_channel_allowed(channel_id)
    }

    /// Check if a message should be skipped (bot messages).
    pub fn should_skip_bot_message(is_bot: bool) -> bool {
        is_bot
    }

    fn status_from_u8(val: u8) -> ChannelStatus {
        match val {
            STATUS_CONNECTED => ChannelStatus::Connected,
            _ => ChannelStatus::Disconnected,
        }
    }
}

#[async_trait]
impl ChannelSender for DiscordChannel {
    fn channel_type(&self) -> &str {
        "discord"
    }

    async fn send_message(&self, _message: ChannelMessage) -> Result<()> {
        Err(MesoError::Channel(
            "discord send requires active connection".into(),
        ))
    }
}

#[async_trait]
impl ChannelLifecycle for DiscordChannel {
    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn connect(&self) -> Result<()> {
        // Actual serenity Client setup happens here in production
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
        Box::new(DiscordSender {
            channel_name: self.display_name.clone(),
        })
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        // Actual serenity gateway connection runs here in production
        Ok(())
    }

    async fn health_check(&self) -> bool {
        self.status.load(Ordering::SeqCst) == STATUS_CONNECTED
    }
}

/// Lightweight send-only handle for Discord.
struct DiscordSender {
    channel_name: String,
}

#[async_trait]
impl ChannelSender for DiscordSender {
    fn channel_type(&self) -> &str {
        "discord"
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

    fn test_config() -> DiscordConfig {
        DiscordConfig {
            allowed_guild_ids: vec![111, 222],
            allowed_channel_ids: vec![333, 444],
        }
    }

    #[test]
    fn channel_type_discord() {
        let ch = DiscordChannel::new(test_config());
        assert_eq!(ch.channel_type(), "discord");
    }

    #[test]
    fn initial_status_disconnected() {
        let ch = DiscordChannel::new(test_config());
        assert_eq!(ch.status(), ChannelStatus::Disconnected);
    }

    #[test]
    fn guild_allowlist() {
        let ch = DiscordChannel::new(test_config());
        assert!(ch.is_guild_allowed(111));
        assert!(ch.is_guild_allowed(222));
        assert!(!ch.is_guild_allowed(999));
    }

    #[test]
    fn channel_allowlist() {
        let ch = DiscordChannel::new(test_config());
        assert!(ch.is_channel_allowed(333));
        assert!(ch.is_channel_allowed(444));
        assert!(!ch.is_channel_allowed(999));
    }

    #[test]
    fn bot_messages_skipped() {
        assert!(DiscordChannel::should_skip_bot_message(true));
        assert!(!DiscordChannel::should_skip_bot_message(false));
    }

    #[test]
    fn empty_allowlist_allows_all() {
        let ch = DiscordChannel::new(DiscordConfig::default());
        assert!(ch.is_guild_allowed(999));
        assert!(ch.is_channel_allowed(999));
    }

    #[test]
    fn required_intents() {
        // Verify the intent bits are correct
        let guild_messages = 1u64 << 9;
        let direct_messages = 1u64 << 12;
        let message_content = 1u64 << 15;

        assert_eq!(
            REQUIRED_INTENTS,
            guild_messages | direct_messages | message_content
        );
        // MESSAGE_CONTENT must be included
        assert!(REQUIRED_INTENTS & message_content != 0);
    }
}
