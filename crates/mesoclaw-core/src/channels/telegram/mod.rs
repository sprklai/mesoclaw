pub mod config;
pub mod fmt;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

use async_trait::async_trait;
use teloxide::Bot;
use teloxide::payloads::{GetUpdatesSetters, SendMessageSetters};
use teloxide::requests::Requester;
use teloxide::types::{ChatId, MessageId, ParseMode, UpdateKind};
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, warn};

use crate::Result;
use crate::config::AppConfig;
use crate::credential::CredentialStore;
use crate::error::MesoError;

use super::message::ChannelMessage;
use super::traits::{Channel, ChannelLifecycle, ChannelSender, ChannelStatus};

use config::{BotCommand, DmPolicy, TelegramConfig, parse_bot_command};

// Status values stored as u8 for atomic access
const STATUS_DISCONNECTED: u8 = 0;
const STATUS_CONNECTING: u8 = 1;
const STATUS_CONNECTED: u8 = 2;

/// Telegram channel implementation using teloxide.
pub struct TelegramChannel {
    config: TelegramConfig,
    app_config: Arc<AppConfig>,
    display_name: String,
    status: AtomicU8,
    credentials: Arc<dyn CredentialStore>,
    bot: tokio::sync::OnceCell<Bot>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    /// Prevents multiple `listen()` tasks from running simultaneously.
    listening: AtomicBool,
    /// Maps chat_id -> status message ID for active agent processing.
    status_messages: Arc<tokio::sync::Mutex<HashMap<i64, MessageId>>>,
    /// Maps chat_id -> typing refresh abort handle.
    typing_handles: Arc<tokio::sync::Mutex<HashMap<i64, tokio::task::JoinHandle<()>>>>,
}

impl TelegramChannel {
    pub fn new(
        config: TelegramConfig,
        credentials: Arc<dyn CredentialStore>,
        app_config: Arc<AppConfig>,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            config,
            app_config,
            display_name: "telegram".to_string(),
            status: AtomicU8::new(STATUS_DISCONNECTED),
            credentials,
            bot: tokio::sync::OnceCell::new(),
            shutdown_tx,
            shutdown_rx,
            listening: AtomicBool::new(false),
            status_messages: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            typing_handles: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
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

    /// Handle a bot command with a canned response.
    async fn handle_bot_command(&self, cmd: BotCommand, chat_id: ChatId) {
        let bot = match self.bot.get() {
            Some(b) => b,
            None => return,
        };

        let response = match cmd {
            BotCommand::Start => "Welcome to MesoClaw! Send me any message and I'll respond with AI assistance.".to_string(),
            BotCommand::Help => "Available commands:\n/start - Welcome message\n/status - Check bot status\n/help - Show this help\n/cancel - Cancel current operation".to_string(),
            BotCommand::Status => {
                let status = Self::status_from_u8(self.status.load(Ordering::SeqCst));
                format!("Status: {status}")
            }
            BotCommand::Cancel => "Operation cancelled.".to_string(),
            _ => return, // Don't respond to unknown commands
        };

        if let Err(e) = bot.send_message(chat_id, response).await {
            warn!("Failed to send bot command response: {e}");
        }
    }

    fn status_from_u8(val: u8) -> ChannelStatus {
        match val {
            STATUS_CONNECTING => ChannelStatus::Connecting,
            STATUS_CONNECTED => ChannelStatus::Connected,
            _ => ChannelStatus::Disconnected,
        }
    }
}

/// Telegram max message size in bytes.
const TELEGRAM_MAX_MESSAGE_BYTES: usize = 4096;

#[async_trait]
impl ChannelSender for TelegramChannel {
    fn channel_type(&self) -> &str {
        "telegram"
    }

    async fn send_message(&self, message: ChannelMessage) -> Result<()> {
        let bot = self
            .bot
            .get()
            .ok_or_else(|| MesoError::Channel("telegram: not connected".into()))?;

        let chat_id_str = message
            .metadata
            .get("chat_id")
            .ok_or_else(|| MesoError::Channel("telegram: missing chat_id in metadata".into()))?;

        let chat_id: i64 = chat_id_str
            .parse()
            .map_err(|_| MesoError::Channel(format!("telegram: invalid chat_id: {chat_id_str}")))?;

        // Split oversized messages to respect Telegram's 4096 byte limit
        let parts = super::format::split_message(&message.content, TELEGRAM_MAX_MESSAGE_BYTES);
        for part in parts {
            bot.send_message(ChatId(chat_id), &part)
                .parse_mode(ParseMode::Html)
                .await
                .map_err(|e| MesoError::Channel(format!("telegram send failed: {e}")))?;
        }

        Ok(())
    }
}

#[async_trait]
impl ChannelLifecycle for TelegramChannel {
    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn connect(&self) -> Result<()> {
        self.status.store(STATUS_CONNECTING, Ordering::SeqCst);

        let token = self
            .credentials
            .get("channel:telegram:token")
            .await
            .map_err(|e| {
                self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
                MesoError::Channel(format!("telegram: credential error: {e}"))
            })?
            .ok_or_else(|| {
                self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
                MesoError::Channel("telegram: bot token not configured".into())
            })?;

        let bot = teloxide::Bot::new(token);

        // Validate bot token by calling getMe
        let me = bot.get_me().await.map_err(|e| {
            self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
            MesoError::Channel(format!("telegram: getMe failed: {e}"))
        })?;

        info!(
            "Telegram bot connected: @{}",
            me.username.as_deref().unwrap_or("unknown")
        );

        // Store bot in OnceCell
        let _ = self.bot.set(bot);

        self.status.store(STATUS_CONNECTED, Ordering::SeqCst);
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        let _ = self.shutdown_tx.send(true);
        self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
        info!("Telegram channel disconnected");
        Ok(())
    }

    fn status(&self) -> ChannelStatus {
        Self::status_from_u8(self.status.load(Ordering::SeqCst))
    }

    fn create_sender(&self) -> Box<dyn ChannelSender> {
        Box::new(TelegramSender {
            bot: self.bot.get().cloned(),
        })
    }
}

#[async_trait]
impl Channel for TelegramChannel {
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        // Prevent duplicate listen tasks
        if self.listening.swap(true, Ordering::SeqCst) {
            return Err(MesoError::Channel(
                "telegram: listen() already running".into(),
            ));
        }

        let bot = self
            .bot
            .get()
            .ok_or_else(|| {
                self.listening.store(false, Ordering::SeqCst);
                MesoError::Channel("telegram: not connected, call connect() first".into())
            })?
            .clone();

        let mut shutdown_rx = self.shutdown_rx.clone();

        // Use teloxide's polling to get updates
        let mut offset: i32 = 0;
        let timeout = self.config.polling_timeout_secs;

        let max_attempts = self.app_config.channel_reconnect_max_attempts;

        info!(
            "Telegram listen loop started (polling_timeout={}s, max_attempts={})",
            timeout, max_attempts
        );

        let mut attempt_count: u32 = 0;

        loop {
            tokio::select! {
                biased;

                Ok(()) = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Telegram listen loop: shutdown signal received");
                        break;
                    }
                }

                result = bot.get_updates().offset(offset).timeout(timeout) => {
                    match result {
                        Ok(updates) => {
                            // Reset attempt count on successful poll
                            attempt_count = 0;

                            for update in updates {
                                offset = update.id.as_offset();

                                if let UpdateKind::Message(msg) = update.kind
                                    && let Some(text) = msg.text()
                                {
                                    let chat_id = msg.chat.id.0;

                                    // Check DM policy
                                    if !self.is_chat_allowed(chat_id) {
                                        debug!("Telegram: dropping message from disallowed chat {chat_id}");
                                        continue;
                                    }

                                    // Check group mention filter
                                    if !self.should_process_group_message(text, chat_id) {
                                        debug!("Telegram: dropping group message without mention from chat {chat_id}");
                                        continue;
                                    }

                                    // Handle bot commands inline
                                    if let Some(cmd) = parse_bot_command(text) {
                                        match cmd {
                                            BotCommand::Start | BotCommand::Help | BotCommand::Status | BotCommand::Cancel => {
                                                self.handle_bot_command(cmd, msg.chat.id).await;
                                                continue;
                                            }
                                            _ => {} // Pass other commands to the AI agent
                                        }
                                    }

                                    let sender_name = msg
                                        .from
                                        .as_ref()
                                        .and_then(|u| u.username.clone())
                                        .unwrap_or_else(|| format!("user_{chat_id}"));

                                    let mut metadata = HashMap::new();
                                    metadata.insert("chat_id".into(), chat_id.to_string());
                                    metadata.insert("message_id".into(), msg.id.0.to_string());

                                    let channel_msg = ChannelMessage::new("telegram", text)
                                        .with_sender(&sender_name)
                                        .with_metadata(metadata);

                                    if let Err(e) = tx.send(channel_msg).await {
                                        error!("Telegram: failed to send to router: {e}");
                                        break;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            attempt_count += 1;
                            warn!("Telegram polling error (attempt {attempt_count}/{max_attempts}): {e}");

                            if attempt_count >= max_attempts {
                                error!("Telegram: max reconnect attempts ({max_attempts}) reached, giving up");
                                self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
                                break;
                            }

                            // Retry with exponential backoff based on attempt count
                            tokio::time::sleep(self.config.retry.delay_for(attempt_count)).await;
                        }
                    }
                }
            }
        }

        self.listening.store(false, Ordering::SeqCst);
        info!("Telegram listen loop stopped");
        Ok(())
    }

    async fn health_check(&self) -> bool {
        if self.status.load(Ordering::SeqCst) != STATUS_CONNECTED {
            return false;
        }
        if let Some(bot) = self.bot.get() {
            bot.get_me().await.is_ok()
        } else {
            false
        }
    }

    async fn on_agent_start(&self, recipient: Option<&str>) {
        let Some(bot) = self.bot.get() else { return };
        let Some(chat_id_str) = recipient else { return };
        let Ok(chat_id) = chat_id_str.parse::<i64>() else {
            return;
        };

        // 1. Send initial typing action
        let _ = bot
            .send_chat_action(ChatId(chat_id), teloxide::types::ChatAction::Typing)
            .await;

        // 2. Send status message
        if let Ok(msg) = bot
            .send_message(ChatId(chat_id), "Processing your request...")
            .await
        {
            let mut status = self.status_messages.lock().await;
            status.insert(chat_id, msg.id);
        }

        // 3. Spawn background typing refresh loop
        let refresh_secs = self.app_config.telegram_status_refresh_secs;
        let bot_clone = bot.clone();
        let status_messages = self.status_messages.clone();
        let handle = tokio::spawn(async move {
            let interval = std::time::Duration::from_secs(refresh_secs.into());
            loop {
                tokio::time::sleep(interval).await;
                // Stop if status message was removed (agent completed)
                let status = status_messages.lock().await;
                if !status.contains_key(&chat_id) {
                    break;
                }
                drop(status);
                let _ = bot_clone
                    .send_chat_action(ChatId(chat_id), teloxide::types::ChatAction::Typing)
                    .await;
            }
        });

        let mut handles = self.typing_handles.lock().await;
        handles.insert(chat_id, handle);
    }

    async fn on_tool_use(&self, tool_name: &str, recipient: Option<&str>) {
        if !self.app_config.telegram_show_tool_status {
            return;
        }
        let Some(bot) = self.bot.get() else { return };
        let Some(chat_id_str) = recipient else { return };
        let Ok(chat_id) = chat_id_str.parse::<i64>() else {
            return;
        };

        // Edit status message to show current tool
        let status = self.status_messages.lock().await;
        if let Some(&msg_id) = status.get(&chat_id) {
            let text = format!("Running: {tool_name}...");
            let _ = bot.edit_message_text(ChatId(chat_id), msg_id, text).await;
        }
    }

    async fn on_agent_complete(&self, recipient: Option<&str>) {
        let Some(bot) = self.bot.get() else { return };
        let Some(chat_id_str) = recipient else { return };
        let Ok(chat_id) = chat_id_str.parse::<i64>() else {
            return;
        };

        // 1. Remove from status_messages (stops typing loop)
        let msg_id = {
            let mut status = self.status_messages.lock().await;
            status.remove(&chat_id)
        };

        // 2. Abort typing refresh handle
        {
            let mut handles = self.typing_handles.lock().await;
            if let Some(handle) = handles.remove(&chat_id) {
                handle.abort();
            }
        }

        // 3. Delete status message
        if let Some(msg_id) = msg_id {
            let _ = bot.delete_message(ChatId(chat_id), msg_id).await;
        }
    }
}

/// Lightweight send-only handle for Telegram.
struct TelegramSender {
    bot: Option<Bot>,
}

#[async_trait]
impl ChannelSender for TelegramSender {
    fn channel_type(&self) -> &str {
        "telegram"
    }

    async fn send_message(&self, message: ChannelMessage) -> Result<()> {
        let bot = self
            .bot
            .as_ref()
            .ok_or_else(|| MesoError::Channel("telegram sender: not connected".into()))?;

        let chat_id_str = message
            .metadata
            .get("chat_id")
            .ok_or_else(|| MesoError::Channel("telegram: missing chat_id in metadata".into()))?;

        let chat_id: i64 = chat_id_str
            .parse()
            .map_err(|_| MesoError::Channel(format!("telegram: invalid chat_id: {chat_id_str}")))?;

        // Split oversized messages to respect Telegram's 4096 byte limit
        let parts = super::format::split_message(&message.content, TELEGRAM_MAX_MESSAGE_BYTES);
        for part in parts {
            bot.send_message(ChatId(chat_id), &part)
                .parse_mode(ParseMode::Html)
                .await
                .map_err(|e| MesoError::Channel(format!("telegram send failed: {e}")))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config::RetryPolicy;

    use crate::credential::InMemoryCredentialStore;

    fn test_credentials() -> Arc<dyn CredentialStore> {
        Arc::new(InMemoryCredentialStore::new())
    }

    fn test_app_config() -> Arc<AppConfig> {
        Arc::new(AppConfig::default())
    }

    fn test_config() -> TelegramConfig {
        TelegramConfig {
            allowed_chat_ids: vec![100, 200],
            polling_timeout_secs: 30,
            dm_policy: DmPolicy::Allowlist,
            retry: RetryPolicy::default(),
            require_group_mention: true,
            bot_username: Some("test_bot".into()),
        }
    }

    #[test]
    fn channel_type_telegram() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        assert_eq!(ch.channel_type(), "telegram");
    }

    #[test]
    fn display_name() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config())
            .with_name("my-telegram");
        assert_eq!(ch.display_name(), "my-telegram");
    }

    #[test]
    fn initial_status_disconnected() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        assert_eq!(ch.status(), ChannelStatus::Disconnected);
    }

    #[test]
    fn create_sender() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        let sender = ch.create_sender();
        assert_eq!(sender.channel_type(), "telegram");
    }

    #[test]
    fn allowlist_blocks_unknown() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        assert!(!ch.is_chat_allowed(999));
    }

    #[test]
    fn allowlist_permits_known() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        assert!(ch.is_chat_allowed(100));
        assert!(ch.is_chat_allowed(200));
    }

    #[test]
    fn open_policy_allows_all() {
        let mut cfg = test_config();
        cfg.dm_policy = DmPolicy::Open;
        let ch = TelegramChannel::new(cfg, test_credentials(), test_app_config());
        assert!(ch.is_chat_allowed(999));
        assert!(ch.is_chat_allowed(0));
    }

    #[test]
    fn disabled_policy_blocks_all() {
        let mut cfg = test_config();
        cfg.dm_policy = DmPolicy::Disabled;
        let ch = TelegramChannel::new(cfg, test_credentials(), test_app_config());
        assert!(!ch.is_chat_allowed(100)); // even allowed IDs
        assert!(!ch.is_chat_allowed(999));
    }

    #[test]
    fn group_mention_filter() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());

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
        let ch2 = TelegramChannel::new(cfg, test_credentials(), test_app_config());
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

    // 8.8.1 — Telegram on_agent_start does not panic
    #[tokio::test]
    async fn telegram_on_agent_start() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        ch.on_agent_start(Some("user1")).await;
    }

    // 8.8.2 — Telegram on_tool_use does not panic
    #[tokio::test]
    async fn telegram_on_tool_use() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        ch.on_tool_use("web_search", Some("user1")).await;
    }

    // 8.8.3 — Telegram on_agent_complete does not panic
    #[tokio::test]
    async fn telegram_on_agent_complete() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        ch.on_agent_complete(Some("user1")).await;
    }

    // 8.8.4b — Telegram lifecycle hooks work with None recipient
    #[tokio::test]
    async fn telegram_hooks_with_none_recipient() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        ch.on_agent_start(None).await;
        ch.on_tool_use("shell", None).await;
        ch.on_agent_complete(None).await;
    }

    // Connect fails without valid token in credential store
    #[tokio::test]
    async fn connect_fails_without_token() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        let result = ch.connect().await;
        assert!(result.is_err());
        assert_eq!(ch.status(), ChannelStatus::Disconnected);
    }

    // Send fails without connection
    #[tokio::test]
    async fn send_fails_without_connection() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        let msg = ChannelMessage::new("telegram", "test");
        let result = ch.send_message(msg).await;
        assert!(result.is_err());
    }

    // Listen fails without connection
    #[tokio::test]
    async fn listen_fails_without_connection() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        let (tx, _rx) = mpsc::channel(10);
        let result = ch.listen(tx).await;
        assert!(result.is_err());
    }

    // Disconnect sends shutdown signal
    #[tokio::test]
    async fn disconnect_sends_shutdown() {
        let ch = TelegramChannel::new(test_config(), test_credentials(), test_app_config());
        ch.disconnect().await.unwrap();
        assert_eq!(ch.status(), ChannelStatus::Disconnected);
        assert!(*ch.shutdown_rx.borrow());
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

    // WS2.5 — Telegram retry backoff increases with attempt count
    #[test]
    fn telegram_retry_backoff_increases() {
        let retry = RetryPolicy::default();
        let d0 = retry.delay_for(0);
        let d3 = retry.delay_for(3);
        assert!(d3 > d0, "Backoff should increase with attempt count");
    }
}
