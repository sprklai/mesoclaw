pub mod config;
pub mod fmt;

use std::sync::atomic::{AtomicU8, Ordering};

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::Result;
use crate::error::MesoError;

use super::message::ChannelMessage;
use super::traits::{Channel, ChannelLifecycle, ChannelSender, ChannelStatus};

use config::{DmPolicy, TelegramConfig};

// Status values stored as u8 for atomic access
const STATUS_DISCONNECTED: u8 = 0;
const STATUS_CONNECTING: u8 = 1;
const STATUS_CONNECTED: u8 = 2;

/// Telegram channel implementation using teloxide.
pub struct TelegramChannel {
    config: TelegramConfig,
    display_name: String,
    status: AtomicU8,
}

impl TelegramChannel {
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            config,
            display_name: "telegram".to_string(),
            status: AtomicU8::new(STATUS_DISCONNECTED),
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.display_name = name.to_string();
        self
    }

    /// Check if a chat ID is allowed by the current DM policy.
    pub fn is_chat_allowed(&self, chat_id: i64) -> bool {
        match self.config.dm_policy {
            DmPolicy::Open => true,
            DmPolicy::Disabled => false,
            DmPolicy::Allowlist => self.config.allowed_chat_ids.contains(&chat_id),
        }
    }

    /// Check if a message from a group should be processed.
    /// Groups have negative chat IDs in Telegram.
    pub fn should_process_group_message(&self, text: &str, chat_id: i64) -> bool {
        if chat_id >= 0 {
            // Not a group message
            return true;
        }

        if !self.config.require_group_mention {
            return true;
        }

        // Require bot command or @mention
        if text.starts_with('/') {
            return true;
        }

        if let Some(ref username) = self.config.bot_username
            && text.contains(&format!("@{username}"))
        {
            return true;
        }

        false
    }

    fn status_from_u8(val: u8) -> ChannelStatus {
        match val {
            STATUS_CONNECTING => ChannelStatus::Connecting,
            STATUS_CONNECTED => ChannelStatus::Connected,
            _ => ChannelStatus::Disconnected,
        }
    }
}

#[async_trait]
impl ChannelSender for TelegramChannel {
    fn channel_type(&self) -> &str {
        "telegram"
    }

    async fn send_message(&self, _message: ChannelMessage) -> Result<()> {
        // Actual sending requires a teloxide Bot instance with token.
        // This is wired up during connect() when the bot is created.
        Err(MesoError::Channel(
            "telegram send requires active connection".into(),
        ))
    }
}

#[async_trait]
impl ChannelLifecycle for TelegramChannel {
    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn connect(&self) -> Result<()> {
        self.status.store(STATUS_CONNECTING, Ordering::SeqCst);
        // Actual teloxide bot setup happens here in production
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
        Box::new(TelegramSender {
            channel_name: self.display_name.clone(),
        })
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        // Actual teloxide long-polling loop runs here in production
        Ok(())
    }

    async fn health_check(&self) -> bool {
        self.status.load(Ordering::SeqCst) == STATUS_CONNECTED
    }
}

/// Lightweight send-only handle for Telegram.
struct TelegramSender {
    channel_name: String,
}

#[async_trait]
impl ChannelSender for TelegramSender {
    fn channel_type(&self) -> &str {
        "telegram"
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
    use config::{BotCommand, parse_bot_command};

    fn test_config() -> TelegramConfig {
        TelegramConfig {
            allowed_chat_ids: vec![100, 200],
            polling_timeout_secs: 30,
            dm_policy: DmPolicy::Allowlist,
            retry: config::RetryPolicy::default(),
            require_group_mention: true,
            bot_username: Some("test_bot".into()),
        }
    }

    #[test]
    fn channel_type_telegram() {
        let ch = TelegramChannel::new(test_config());
        assert_eq!(ch.channel_type(), "telegram");
    }

    #[test]
    fn display_name() {
        let ch = TelegramChannel::new(test_config()).with_name("my-telegram");
        assert_eq!(ch.display_name(), "my-telegram");
    }

    #[test]
    fn initial_status_disconnected() {
        let ch = TelegramChannel::new(test_config());
        assert_eq!(ch.status(), ChannelStatus::Disconnected);
    }

    #[test]
    fn create_sender() {
        let ch = TelegramChannel::new(test_config());
        let sender = ch.create_sender();
        assert_eq!(sender.channel_type(), "telegram");
    }

    #[test]
    fn allowlist_blocks_unknown() {
        let ch = TelegramChannel::new(test_config());
        assert!(!ch.is_chat_allowed(999));
    }

    #[test]
    fn allowlist_permits_known() {
        let ch = TelegramChannel::new(test_config());
        assert!(ch.is_chat_allowed(100));
        assert!(ch.is_chat_allowed(200));
    }

    #[test]
    fn open_policy_allows_all() {
        let mut cfg = test_config();
        cfg.dm_policy = DmPolicy::Open;
        let ch = TelegramChannel::new(cfg);
        assert!(ch.is_chat_allowed(999));
        assert!(ch.is_chat_allowed(0));
    }

    #[test]
    fn disabled_policy_blocks_all() {
        let mut cfg = test_config();
        cfg.dm_policy = DmPolicy::Disabled;
        let ch = TelegramChannel::new(cfg);
        assert!(!ch.is_chat_allowed(100)); // even allowed IDs
        assert!(!ch.is_chat_allowed(999));
    }

    #[test]
    fn group_mention_filter() {
        let ch = TelegramChannel::new(test_config());

        // DM (positive chat_id) always processed
        assert!(ch.should_process_group_message("hello", 100));

        // Group (negative chat_id) without mention or command — blocked
        assert!(!ch.should_process_group_message("hello", -100));

        // Group with bot command — allowed
        assert!(ch.should_process_group_message("/status", -100));

        // Group with @mention — allowed
        assert!(ch.should_process_group_message("hey @test_bot help", -100));

        // Group without require_group_mention — always allowed
        let mut cfg = test_config();
        cfg.require_group_mention = false;
        let ch2 = TelegramChannel::new(cfg);
        assert!(ch2.should_process_group_message("hello", -100));
    }

    #[test]
    fn cmd_start() {
        assert_eq!(parse_bot_command("/start"), Some(BotCommand::Start));
        assert_eq!(parse_bot_command("/start@mybot"), Some(BotCommand::Start));
    }

    #[test]
    fn cmd_status() {
        assert_eq!(parse_bot_command("/status"), Some(BotCommand::Status));
    }

    #[test]
    fn cmd_help() {
        assert_eq!(parse_bot_command("/help"), Some(BotCommand::Help));
    }

    #[test]
    fn cmd_cancel() {
        assert_eq!(parse_bot_command("/cancel"), Some(BotCommand::Cancel));
    }

    #[test]
    fn cmd_unknown_ignored() {
        // Non-command text returns None
        assert!(parse_bot_command("hello world").is_none());
        assert!(parse_bot_command("not a command").is_none());

        // Unknown command returns Unknown variant
        let result = parse_bot_command("/foobar");
        assert!(matches!(result, Some(BotCommand::Unknown(cmd)) if cmd == "foobar"));
    }
}
