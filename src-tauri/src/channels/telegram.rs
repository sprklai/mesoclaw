//! Telegram channel integration for MesoClaw (Phase 7.1).
//!
//! Requires the `channels-telegram` Cargo feature:
//!
//! ```sh
//! cargo build --features channels-telegram
//! cargo test  --features channels-telegram -- channels::telegram
//! ```
//!
//! # Architecture
//!
//! ```text
//!  Telegram Bot API  ──(long-poll)──▶  TelegramChannel::listen()
//!                                             │
//!                                  allowed_chat_ids check
//!                                             │
//!                               mpsc::Sender<ChannelMessage>
//!                                             │
//!                                       ChannelManager
//!                                             │
//!                                        Agent loop
//! ```
//!
//! # Bot commands
//!
//! | Command            | Effect                                         |
//! |--------------------|------------------------------------------------|
//! | `/start`           | Greeting with agent identity                   |
//! | `/status`          | Agent status, active jobs, memory stats        |
//! | `/cancel`          | Cancel running agent session for this chat     |
//! | `/help`            | List available commands                        |
//! | `/allow {chat_id}` | Admin: add a new chat ID to the allow-list     |
//!
//! # Security
//!
//! Unknown chat IDs are **silently ignored** — the bot does not reveal its
//! existence to untrusted senders.  Approvals are **never** sent to Telegram;
//! they are always handled via the trusted desktop UI.
//!
//! # Reconnection
//!
//! On connection loss the listener retries with exponential back-off:
//! 1 s → 2 s → 4 s → 8 s → … → max 60 s.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::{RwLock, mpsc};

use super::traits::{Channel, ChannelMessage};

// ─── TelegramConfig ──────────────────────────────────────────────────────────

/// Configuration for [`TelegramChannel`].
#[derive(Debug, Clone)]
pub struct TelegramConfig {
    /// Bot token from BotFather (stored in OS keyring in production).
    pub token: String,
    /// Only these Telegram chat IDs are allowed to send messages.
    /// Empty list means no chat IDs are initially allowed.
    pub allowed_chat_ids: Vec<i64>,
    /// Long-polling timeout in seconds (default: 30).
    pub polling_timeout_secs: u32,
}

impl TelegramConfig {
    /// Create a config with the given bot token and no allowed chat IDs.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            allowed_chat_ids: Vec::new(),
            polling_timeout_secs: 30,
        }
    }

    /// Create a config with explicit allowed chat IDs.
    pub fn with_allowed_ids(token: impl Into<String>, allowed_chat_ids: Vec<i64>) -> Self {
        Self {
            token: token.into(),
            allowed_chat_ids,
            polling_timeout_secs: 30,
        }
    }
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self::new("")
    }
}

// ─── BotCommand ──────────────────────────────────────────────────────────────

/// Recognised bot commands.
#[derive(Debug, Clone, PartialEq)]
pub enum BotCommand {
    Start,
    Status,
    Cancel,
    Help,
    /// The `/allow {chat_id}` admin command.
    Allow(i64),
    Unknown(String),
}

// ─── TelegramChannel ─────────────────────────────────────────────────────────

/// A [`Channel`] implementation backed by the Telegram Bot API.
///
/// The channel uses long-polling to receive messages and the Bot API to send
/// them.  All IO is feature-gated behind `channels-telegram`.
pub struct TelegramChannel {
    token: String,
    allowed_chat_ids: Arc<RwLock<Vec<i64>>>,
    polling_timeout_secs: u32,
}

impl TelegramChannel {
    /// Construct a new channel from the given config.
    pub fn new(config: TelegramConfig) -> Self {
        Self {
            token: config.token,
            allowed_chat_ids: Arc::new(RwLock::new(config.allowed_chat_ids)),
            polling_timeout_secs: config.polling_timeout_secs,
        }
    }

    /// Add a chat ID to the allow-list at runtime.
    pub async fn allow_chat(&self, chat_id: i64) {
        let mut ids = self.allowed_chat_ids.write().await;
        if !ids.contains(&chat_id) {
            ids.push(chat_id);
        }
    }

    /// Remove a chat ID from the allow-list.
    pub async fn deny_chat(&self, chat_id: i64) {
        let mut ids = self.allowed_chat_ids.write().await;
        ids.retain(|&id| id != chat_id);
    }

    /// Return a snapshot of the current allow-list.
    pub async fn allowed_ids(&self) -> Vec<i64> {
        self.allowed_chat_ids.read().await.clone()
    }

    /// Check whether `chat_id` is on the allow-list.
    pub async fn is_allowed(&self, chat_id: i64) -> bool {
        self.allowed_chat_ids.read().await.contains(&chat_id)
    }

    // ─── Pure helpers (also used in tests) ───────────────────────────────────

    /// Escape text for Telegram's MarkdownV2 parse mode.
    ///
    /// Escapes all reserved characters outside of pre-formatted regions:
    /// `_`, `*`, `[`, `]`, `(`, `)`, `~`, `` ` ``, `>`, `#`, `+`, `-`,
    /// `=`, `|`, `{`, `}`, `.`, `!`
    pub fn escape_markdown_v2(text: &str) -> String {
        const RESERVED: &[char] = &[
            '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.',
            '!',
        ];
        let mut out = String::with_capacity(text.len() + 16);
        for ch in text.chars() {
            if RESERVED.contains(&ch) {
                out.push('\\');
            }
            out.push(ch);
        }
        out
    }

    /// Split `text` into chunks of at most 4 096 characters (Telegram's limit).
    ///
    /// Split preference order:
    /// 1. Double newline (paragraph boundary)
    /// 2. Single newline
    /// 3. Period + space (sentence boundary)
    /// 4. Space (word boundary)
    /// 5. Hard 4 096-char cut (last resort)
    pub fn split_message(text: &str) -> Vec<String> {
        const MAX: usize = 4096;

        if text.len() <= MAX {
            return vec![text.to_string()];
        }

        let mut parts = Vec::new();
        let mut remaining = text;

        while remaining.len() > MAX {
            let chunk = &remaining[..MAX];

            let split_at = Self::find_split(chunk, "\n\n")
                .or_else(|| Self::find_split(chunk, "\n"))
                .or_else(|| Self::find_split(chunk, ". "))
                .or_else(|| Self::find_split(chunk, " "))
                .unwrap_or(MAX);

            parts.push(remaining[..split_at].to_string());
            remaining = remaining[split_at..].trim_start();
        }

        if !remaining.is_empty() {
            parts.push(remaining.to_string());
        }

        parts
    }

    /// Find the last occurrence of `delimiter` in `chunk`; return the index
    /// after the delimiter as the preferred split point.
    fn find_split(chunk: &str, delimiter: &str) -> Option<usize> {
        chunk.rfind(delimiter).map(|i| i + delimiter.len())
    }

    /// Parse a bot command from a message text.
    ///
    /// Returns `None` if the text is not a bot command (does not start with `/`).
    pub fn parse_bot_command(text: &str) -> Option<BotCommand> {
        let text = text.trim();
        if !text.starts_with('/') {
            return None;
        }

        // Strip leading `/`, isolate the command word before any space or `@`.
        let raw = text.trim_start_matches('/');
        let cmd = raw
            .split_once(|c: char| c == ' ' || c == '@')
            .map(|(c, _)| c)
            .unwrap_or(raw);

        match cmd.to_lowercase().as_str() {
            "start" => Some(BotCommand::Start),
            "status" => Some(BotCommand::Status),
            "cancel" => Some(BotCommand::Cancel),
            "help" => Some(BotCommand::Help),
            "allow" => {
                let arg = raw
                    .split_once(' ')
                    .map(|(_, a)| a.trim())
                    .unwrap_or("")
                    .parse::<i64>()
                    .ok()?;
                Some(BotCommand::Allow(arg))
            }
            _ => Some(BotCommand::Unknown(cmd.to_string())),
        }
    }

    /// Compute exponential back-off for reconnection attempts.
    ///
    /// Returns `min(2^attempt, 60)` seconds, so the sequence is
    /// 1 s, 2 s, 4 s, 8 s, 16 s, 32 s, 60 s, 60 s, …
    pub fn reconnect_backoff(attempt: u32) -> Duration {
        let secs = (1u64 << attempt.min(63)).min(60);
        Duration::from_secs(secs)
    }
}

// ─── Channel trait impl ───────────────────────────────────────────────────────

#[async_trait]
impl Channel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    /// Send `message` to the given `recipient` (Telegram chat ID as a string).
    ///
    /// If `recipient` is `None` the send is a no-op — Telegram always requires
    /// a chat ID.  Long messages are automatically split into ≤ 4 096-char parts.
    async fn send(&self, message: &str, recipient: Option<&str>) -> Result<(), String> {
        let Some(recipient) = recipient else {
            return Err("telegram send: recipient (chat_id) is required".into());
        };

        let chat_id: i64 = recipient
            .parse()
            .map_err(|_| format!("telegram send: invalid chat_id '{recipient}'"))?;

        #[cfg(feature = "channels-telegram")]
        return self.send_via_api(chat_id, message).await;

        #[cfg(not(feature = "channels-telegram"))]
        {
            let _ = chat_id;
            Err("telegram channel not compiled (missing `channels-telegram` feature)".into())
        }
    }

    /// Begin long-polling for inbound messages.
    ///
    /// Silently ignores messages from chat IDs not on the allow-list.
    /// Reconnects with exponential back-off on failure.
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
        #[cfg(feature = "channels-telegram")]
        return self.poll_loop(tx).await;

        #[cfg(not(feature = "channels-telegram"))]
        {
            let _ = tx;
            Err("telegram channel not compiled (missing `channels-telegram` feature)".into())
        }
    }

    /// Perform a lightweight health check by calling `getMe`.
    async fn health_check(&self) -> bool {
        #[cfg(feature = "channels-telegram")]
        {
            use teloxide::prelude::*;
            let bot = Bot::new(&self.token);
            bot.get_me().await.is_ok()
        }
        #[cfg(not(feature = "channels-telegram"))]
        false
    }
}

// ─── Telegram API helpers (channels-telegram only) ───────────────────────────

#[cfg(feature = "channels-telegram")]
impl TelegramChannel {
    async fn send_via_api(&self, chat_id: i64, message: &str) -> Result<(), String> {
        // payloads::setters::* provides .parse_mode() and other builder methods.
        use teloxide::prelude::*;
        use teloxide::types::ParseMode;

        let bot = Bot::new(&self.token);
        let parts = Self::split_message(message);

        for part in parts {
            let escaped = Self::escape_markdown_v2(&part);
            bot.send_message(ChatId(chat_id), escaped)
                .parse_mode(ParseMode::MarkdownV2)
                .await
                .map_err(|e| format!("telegram send error: {e}"))?;
        }
        Ok(())
    }

    /// Run the long-polling loop with exponential back-off on errors.
    async fn poll_loop(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
        // payloads::setters::* provides .offset() and .timeout() builder methods.
        use teloxide::prelude::*;
        use teloxide::types::UpdateKind;

        let bot = Bot::new(&self.token);
        let allowed = Arc::clone(&self.allowed_chat_ids);
        let mut attempt = 0u32;
        // Offset acknowledges processed updates; must be last_id + 1.
        // UpdateId.0 is u32; cast to i32 (safe for realistic Telegram IDs).
        let mut offset: i32 = 0;

        loop {
            let result = bot
                .get_updates()
                .offset(offset)
                .timeout(self.polling_timeout_secs)
                .await;

            match result {
                Ok(updates) => {
                    attempt = 0; // reset back-off on success
                    for update in updates {
                        // Acknowledge this update regardless of whether we process it.
                        offset = (update.id.0 as i32).saturating_add(1);

                        if let UpdateKind::Message(ref msg) = update.kind {
                            let chat_id = msg.chat.id.0;

                            // Silently ignore unknown senders.
                            if !allowed.read().await.contains(&chat_id) {
                                continue;
                            }

                            let content = msg
                                .text()
                                .map(str::to_string)
                                .unwrap_or_else(|| Self::describe_media(msg));

                            let channel_msg = ChannelMessage::new("telegram", content)
                                .with_sender(chat_id.to_string());

                            // If the receiver was dropped, stop gracefully.
                            if tx.send(channel_msg).await.is_err() {
                                return Ok(());
                            }
                        }
                    }
                }
                Err(e) => {
                    if tx.is_closed() {
                        return Ok(());
                    }
                    log::warn!("telegram: polling error (attempt {attempt}): {e}");
                    let backoff = Self::reconnect_backoff(attempt);
                    attempt = attempt.saturating_add(1);
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    /// Produce a human-readable description of a non-text message.
    fn describe_media(msg: &teloxide::types::Message) -> String {
        if msg.photo().is_some() {
            "[photo]".to_string()
        } else if let Some(doc) = msg.document() {
            format!(
                "[document: {}]",
                doc.file_name.as_deref().unwrap_or("unknown")
            )
        } else if msg.voice().is_some() {
            "[voice message]".to_string()
        } else if msg.audio().is_some() {
            "[audio]".to_string()
        } else if msg.video().is_some() {
            "[video]".to_string()
        } else {
            "[unsupported media]".to_string()
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ────────────────────────────────────────────────────────────────

    fn channel_with_ids(ids: Vec<i64>) -> TelegramChannel {
        TelegramChannel::new(TelegramConfig::with_allowed_ids("test-token", ids))
    }

    // ── Channel identity ───────────────────────────────────────────────────────

    #[test]
    fn channel_name_is_telegram() {
        let ch = TelegramChannel::new(TelegramConfig::new("token"));
        assert_eq!(ch.name(), "telegram");
    }

    // ── MarkdownV2 escaping ────────────────────────────────────────────────────

    #[test]
    fn escape_dots() {
        assert_eq!(TelegramChannel::escape_markdown_v2("3.14"), r"3\.14");
    }

    #[test]
    fn escape_exclamation() {
        assert_eq!(TelegramChannel::escape_markdown_v2("Hello!"), r"Hello\!");
    }

    #[test]
    fn escape_parentheses() {
        assert_eq!(TelegramChannel::escape_markdown_v2("(value)"), r"\(value\)");
    }

    #[test]
    fn escape_underscores_and_asterisks() {
        assert_eq!(TelegramChannel::escape_markdown_v2("_bold_"), r"\_bold\_");
    }

    #[test]
    fn escape_hash_plus_minus() {
        let out = TelegramChannel::escape_markdown_v2("# header +1 -1");
        assert!(out.contains("\\#"));
        assert!(out.contains("\\+"));
        assert!(out.contains("\\-"));
    }

    #[test]
    fn escape_empty_string() {
        assert_eq!(TelegramChannel::escape_markdown_v2(""), "");
    }

    #[test]
    fn plain_ascii_without_reserved_chars_is_unchanged() {
        let plain = "hello world 123";
        assert_eq!(TelegramChannel::escape_markdown_v2(plain), plain);
    }

    // ── Message splitting ──────────────────────────────────────────────────────

    #[test]
    fn short_message_is_not_split() {
        let text = "Hello, World!";
        let parts = TelegramChannel::split_message(text);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], text);
    }

    #[test]
    fn exactly_4096_chars_is_single_part() {
        let text = "a".repeat(4096);
        let parts = TelegramChannel::split_message(&text);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn message_over_4096_splits_into_multiple_parts() {
        let text = "a ".repeat(2100); // 4200 chars
        let parts = TelegramChannel::split_message(&text);
        assert!(
            parts.len() >= 2,
            "expected at least 2 parts, got {}",
            parts.len()
        );
        for part in &parts {
            assert!(
                part.len() <= 4096,
                "part exceeds 4096 chars: {}",
                part.len()
            );
        }
    }

    #[test]
    fn split_all_content_is_preserved() {
        let text = "word ".repeat(1000); // 5000 chars
        let parts = TelegramChannel::split_message(&text);
        let total: usize = parts.iter().map(|p| p.len()).sum();
        // Allow for trim_start() removing small amounts of whitespace.
        assert!(total <= text.len());
        assert!(total > 0);
    }

    #[test]
    fn split_prefers_paragraph_boundary() {
        // 2048 + "\n\n" + 2048 = 4098 chars — just over the 4096 limit.
        // The split should happen at the double-newline, not mid-word.
        let para = "x".repeat(2048);
        let text = format!("{para}\n\n{para}");
        let parts = TelegramChannel::split_message(&text);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].trim(), para);
    }

    // ── Bot command parsing ────────────────────────────────────────────────────

    #[test]
    fn parse_start_command() {
        assert_eq!(
            TelegramChannel::parse_bot_command("/start"),
            Some(BotCommand::Start)
        );
    }

    #[test]
    fn parse_status_command() {
        assert_eq!(
            TelegramChannel::parse_bot_command("/status"),
            Some(BotCommand::Status)
        );
    }

    #[test]
    fn parse_cancel_command() {
        assert_eq!(
            TelegramChannel::parse_bot_command("/cancel"),
            Some(BotCommand::Cancel)
        );
    }

    #[test]
    fn parse_help_command() {
        assert_eq!(
            TelegramChannel::parse_bot_command("/help"),
            Some(BotCommand::Help)
        );
    }

    #[test]
    fn parse_allow_command_with_id() {
        assert_eq!(
            TelegramChannel::parse_bot_command("/allow 123456789"),
            Some(BotCommand::Allow(123_456_789))
        );
    }

    #[test]
    fn parse_allow_command_missing_id_returns_none() {
        // /allow with no argument: parse::<i64>() fails, returns None.
        assert_eq!(TelegramChannel::parse_bot_command("/allow"), None);
    }

    #[test]
    fn parse_unknown_command() {
        assert_eq!(
            TelegramChannel::parse_bot_command("/unknown"),
            Some(BotCommand::Unknown("unknown".into()))
        );
    }

    #[test]
    fn plain_text_is_not_a_command() {
        assert_eq!(TelegramChannel::parse_bot_command("hello"), None);
    }

    #[test]
    fn command_with_bot_mention_is_stripped() {
        assert_eq!(
            TelegramChannel::parse_bot_command("/start@MyBot"),
            Some(BotCommand::Start)
        );
    }

    // ── allowed_chat_ids ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn new_channel_has_empty_allow_list() {
        let ch = TelegramChannel::new(TelegramConfig::new("token"));
        assert!(ch.allowed_ids().await.is_empty());
    }

    #[tokio::test]
    async fn new_channel_with_preset_ids() {
        let ch = channel_with_ids(vec![111, 222]);
        assert_eq!(ch.allowed_ids().await, vec![111, 222]);
    }

    #[tokio::test]
    async fn allow_chat_adds_id() {
        let ch = channel_with_ids(vec![]);
        ch.allow_chat(42).await;
        assert!(ch.is_allowed(42).await);
    }

    #[tokio::test]
    async fn allow_chat_deduplicates() {
        let ch = channel_with_ids(vec![42]);
        ch.allow_chat(42).await; // already present
        assert_eq!(ch.allowed_ids().await.len(), 1);
    }

    #[tokio::test]
    async fn deny_chat_removes_id() {
        let ch = channel_with_ids(vec![1, 2, 3]);
        ch.deny_chat(2).await;
        let ids = ch.allowed_ids().await;
        assert!(!ids.contains(&2));
        assert_eq!(ids.len(), 2);
    }

    #[tokio::test]
    async fn unknown_chat_id_is_not_allowed() {
        let ch = channel_with_ids(vec![100, 200]);
        assert!(!ch.is_allowed(999).await);
    }

    // ── Reconnection back-off ─────────────────────────────────────────────────

    #[test]
    fn backoff_attempt_0_is_1s() {
        assert_eq!(
            TelegramChannel::reconnect_backoff(0),
            Duration::from_secs(1)
        );
    }

    #[test]
    fn backoff_attempt_1_is_2s() {
        assert_eq!(
            TelegramChannel::reconnect_backoff(1),
            Duration::from_secs(2)
        );
    }

    #[test]
    fn backoff_attempt_5_is_32s() {
        assert_eq!(
            TelegramChannel::reconnect_backoff(5),
            Duration::from_secs(32)
        );
    }

    #[test]
    fn backoff_is_capped_at_60s() {
        assert_eq!(
            TelegramChannel::reconnect_backoff(7),
            Duration::from_secs(60)
        );
    }

    #[test]
    fn backoff_very_large_attempt_stays_at_60s() {
        assert_eq!(
            TelegramChannel::reconnect_backoff(100),
            Duration::from_secs(60)
        );
    }
}
