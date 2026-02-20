//! Discord channel integration for MesoClaw (Phase 7.2).
//!
//! Requires the `channels-discord` Cargo feature:
//!
//! ```sh
//! cargo build --features channels-discord
//! cargo test  --features channels-discord -- channels::discord
//! ```
//!
//! # Architecture
//!
//! ```text
//!  Discord Gateway (WebSocket)  ──────────▶  DiscordChannel::listen()
//!                                                    │
//!                                        allowed_guild_ids / channel_ids check
//!                                                    │
//!                                      mpsc::Sender<ChannelMessage>
//!                                                    │
//!                                              ChannelManager
//!                                                    │
//!                                             Agent loop
//! ```
//!
//! # Discord Developer Portal setup
//!
//! 1. Go to <https://discord.com/developers/applications>
//! 2. Create a New Application → Bot
//! 3. Copy the Bot Token (stored in OS keyring)
//! 4. Under "Privileged Gateway Intents" enable **Message Content Intent**
//! 5. Use OAuth2 URL Generator to invite the bot with `bot` + `Send Messages` scopes
//!
//! # Security
//!
//! Only guild IDs and channel IDs on the allow-lists receive responses.
//! Unknown sources are **silently ignored**.
//!
//! # Keyring keys
//!
//! | Key | Description |
//! |-----|-------------|
//! | `channel:discord:token` | Bot token from Discord Developer Portal |
//! | `channel:discord:allowed_guild_ids` | Comma-separated u64 guild (server) IDs |
//! | `channel:discord:allowed_channel_ids` | Comma-separated u64 channel IDs |

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::traits::{Channel, ChannelMessage};

// ─── DiscordConfig ────────────────────────────────────────────────────────────

/// Configuration for [`DiscordChannel`].
#[derive(Debug, Clone)]
pub struct DiscordConfig {
    /// Bot token from the Discord Developer Portal.
    pub token: String,
    /// Only messages from these guild (server) IDs are forwarded.
    /// Empty list means all guilds are accepted.
    pub allowed_guild_ids: Vec<u64>,
    /// Only messages from these channel IDs are forwarded.
    /// Empty list means all channels are accepted.
    pub allowed_channel_ids: Vec<u64>,
}

impl DiscordConfig {
    /// Create a config with the given bot token and no allow-list restrictions.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            allowed_guild_ids: Vec::new(),
            allowed_channel_ids: Vec::new(),
        }
    }

    /// Create a config with explicit guild and channel allow-lists.
    pub fn with_allowlists(
        token: impl Into<String>,
        allowed_guild_ids: Vec<u64>,
        allowed_channel_ids: Vec<u64>,
    ) -> Self {
        Self {
            token: token.into(),
            allowed_guild_ids,
            allowed_channel_ids,
        }
    }
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self::new("")
    }
}

// ─── DiscordChannel ───────────────────────────────────────────────────────────

/// A [`Channel`] implementation backed by the Discord Bot API via serenity.
///
/// Uses the WebSocket gateway to receive messages and the HTTP API to send them.
/// All IO is feature-gated behind `channels-discord`.
pub struct DiscordChannel {
    token: String,
    allowed_guild_ids: Vec<u64>,
    allowed_channel_ids: Vec<u64>,
}

impl DiscordChannel {
    /// Construct a new channel from the given config.
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            token: config.token,
            allowed_guild_ids: config.allowed_guild_ids,
            allowed_channel_ids: config.allowed_channel_ids,
        }
    }

    /// Check whether `guild_id` passes the allow-list (empty = allow all).
    pub fn is_guild_allowed(&self, guild_id: u64) -> bool {
        self.allowed_guild_ids.is_empty() || self.allowed_guild_ids.contains(&guild_id)
    }

    /// Check whether `channel_id` passes the allow-list (empty = allow all).
    pub fn is_channel_allowed(&self, channel_id: u64) -> bool {
        self.allowed_channel_ids.is_empty() || self.allowed_channel_ids.contains(&channel_id)
    }
}

// ─── Channel trait impl ───────────────────────────────────────────────────────

#[async_trait]
impl Channel for DiscordChannel {
    fn name(&self) -> &str {
        "discord"
    }

    /// Send `message` to the given Discord channel ID.
    ///
    /// `recipient` must be a string-encoded Discord channel snowflake ID.
    async fn send(&self, message: &str, recipient: Option<&str>) -> Result<(), String> {
        let Some(recipient) = recipient else {
            return Err("discord send: recipient (channel_id) is required".into());
        };

        let channel_id: u64 = recipient
            .parse()
            .map_err(|_| format!("discord send: invalid channel_id '{recipient}'"))?;

        #[cfg(feature = "channels-discord")]
        return self.send_via_http(channel_id, message).await;

        #[cfg(not(feature = "channels-discord"))]
        {
            let _ = channel_id;
            Err("discord channel not compiled (missing `channels-discord` feature)".into())
        }
    }

    /// Connect to the Discord gateway and begin receiving messages.
    ///
    /// Requires the **Message Content Intent** to be enabled in the Developer Portal.
    /// Silently ignores messages from guilds/channels not on the allow-lists.
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
        #[cfg(feature = "channels-discord")]
        return self.run_gateway(tx).await;

        #[cfg(not(feature = "channels-discord"))]
        {
            let _ = tx;
            Err("discord channel not compiled (missing `channels-discord` feature)".into())
        }
    }

    /// Perform a lightweight health check by calling `get_current_user`.
    async fn health_check(&self) -> bool {
        #[cfg(feature = "channels-discord")]
        {
            use serenity::http::Http;
            let http = Http::new(&self.token);
            http.get_current_user().await.is_ok()
        }
        #[cfg(not(feature = "channels-discord"))]
        false
    }
}

// ─── Discord API helpers (channels-discord only) ─────────────────────────────

#[cfg(feature = "channels-discord")]
impl DiscordChannel {
    /// Send a message to a Discord channel via the HTTP API.
    async fn send_via_http(&self, channel_id: u64, message: &str) -> Result<(), String> {
        use serenity::http::Http;
        use serenity::model::id::ChannelId;

        let http = Http::new(&self.token);
        ChannelId::new(channel_id)
            .say(&http, message)
            .await
            .map_err(|e| format!("discord send error: {e}"))?;
        Ok(())
    }

    /// Connect to the Discord WebSocket gateway and forward messages via `tx`.
    ///
    /// Defines an inline `EventHandler` that checks allow-lists and normalises
    /// inbound `Message` events into [`ChannelMessage`]s.
    async fn run_gateway(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
        use serenity::all::ClientBuilder;
        use serenity::model::channel::Message;
        use serenity::model::gateway::Ready;
        use serenity::prelude::*;

        struct Handler {
            tx: mpsc::Sender<ChannelMessage>,
            allowed_guild_ids: Vec<u64>,
            allowed_channel_ids: Vec<u64>,
        }

        #[serenity::async_trait]
        impl EventHandler for Handler {
            async fn ready(&self, _ctx: Context, ready: Ready) {
                log::info!("discord: connected as {}", ready.user.name);
            }

            async fn message(&self, _ctx: Context, msg: Message) {
                // Ignore bot messages.
                if msg.author.bot {
                    return;
                }

                // Guild allow-list check.
                if let Some(guild_id) = msg.guild_id
                    && !self.allowed_guild_ids.is_empty()
                    && !self.allowed_guild_ids.contains(&guild_id.get())
                {
                    return;
                }

                // Channel allow-list check.
                if !self.allowed_channel_ids.is_empty()
                    && !self.allowed_channel_ids.contains(&msg.channel_id.get())
                {
                    return;
                }

                let channel_msg = ChannelMessage::new("discord", msg.content.clone())
                    .with_sender(msg.author.name.clone());

                if self.tx.send(channel_msg).await.is_err() {
                    // Receiver dropped — channel was shut down.
                }
            }
        }

        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let mut client = ClientBuilder::new(&self.token, intents)
            .event_handler(Handler {
                tx,
                allowed_guild_ids: self.allowed_guild_ids.clone(),
                allowed_channel_ids: self.allowed_channel_ids.clone(),
            })
            .await
            .map_err(|e| format!("discord: failed to build client: {e}"))?;

        client
            .start()
            .await
            .map_err(|e| format!("discord: gateway error: {e}"))
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn channel() -> DiscordChannel {
        DiscordChannel::new(DiscordConfig::new("test-token"))
    }

    fn channel_with_lists(guild_ids: Vec<u64>, channel_ids: Vec<u64>) -> DiscordChannel {
        DiscordChannel::new(DiscordConfig::with_allowlists(
            "test-token",
            guild_ids,
            channel_ids,
        ))
    }

    // ── Identity ───────────────────────────────────────────────────────────────

    #[test]
    fn channel_name_is_discord() {
        assert_eq!(channel().name(), "discord");
    }

    // ── Config construction ───────────────────────────────────────────────────

    #[test]
    fn config_new_has_empty_allowlists() {
        let cfg = DiscordConfig::new("token");
        assert_eq!(cfg.token, "token");
        assert!(cfg.allowed_guild_ids.is_empty());
        assert!(cfg.allowed_channel_ids.is_empty());
    }

    #[test]
    fn config_with_allowlists_stores_ids() {
        let cfg = DiscordConfig::with_allowlists("tok", vec![111, 222], vec![333]);
        assert_eq!(cfg.allowed_guild_ids, vec![111, 222]);
        assert_eq!(cfg.allowed_channel_ids, vec![333]);
    }

    #[test]
    fn config_default_has_empty_token() {
        let cfg = DiscordConfig::default();
        assert!(cfg.token.is_empty());
    }

    // ── Guild allow-list ───────────────────────────────────────────────────────

    #[test]
    fn empty_guild_allowlist_allows_all() {
        let ch = channel_with_lists(vec![], vec![]);
        assert!(ch.is_guild_allowed(999_999));
        assert!(ch.is_guild_allowed(0));
    }

    #[test]
    fn guild_allowlist_allows_only_listed_ids() {
        let ch = channel_with_lists(vec![100, 200], vec![]);
        assert!(ch.is_guild_allowed(100));
        assert!(ch.is_guild_allowed(200));
        assert!(!ch.is_guild_allowed(300));
    }

    // ── Channel allow-list ────────────────────────────────────────────────────

    #[test]
    fn empty_channel_allowlist_allows_all() {
        let ch = channel_with_lists(vec![], vec![]);
        assert!(ch.is_channel_allowed(777_777));
    }

    #[test]
    fn channel_allowlist_blocks_unlisted_channels() {
        let ch = channel_with_lists(vec![], vec![42, 43]);
        assert!(ch.is_channel_allowed(42));
        assert!(!ch.is_channel_allowed(44));
    }

    // ── send() without feature ────────────────────────────────────────────────

    #[tokio::test]
    async fn send_without_recipient_returns_error() {
        let ch = channel();
        let result = ch.send("hello", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("recipient"));
    }

    #[tokio::test]
    async fn send_with_invalid_channel_id_returns_error() {
        let ch = channel();
        let result = ch.send("hello", Some("not-a-number")).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid channel_id"));
    }

    // ── health_check without feature ──────────────────────────────────────────

    #[tokio::test]
    async fn health_check_returns_false_without_feature() {
        let ch = channel();
        // Without channels-discord feature, always false.
        #[cfg(not(feature = "channels-discord"))]
        assert!(!ch.health_check().await);
        // With feature, this would make a real network call — skip in unit tests.
        #[cfg(feature = "channels-discord")]
        let _ = ch; // silence unused warning
    }
}
