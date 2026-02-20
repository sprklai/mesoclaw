//! Slack channel integration for MesoClaw (Phase 7.4).
//!
//! Requires the `channels-slack` Cargo feature:
//!
//! ```sh
//! cargo build --features channels-slack
//! cargo test  --features channels-slack -- channels::slack
//! ```
//!
//! # Architecture
//!
//! ```text
//!  Slack Socket Mode (WSS)  ──────────────▶  SlackChannel::listen()
//!                                                   │
//!                                    allowed_channel_ids check
//!                                                   │
//!                                     mpsc::Sender<ChannelMessage>
//!                                                   │
//!                                            ChannelManager
//!                                                   │
//!                                           Agent loop
//! ```
//!
//! # Slack App setup
//!
//! 1. Go to <https://api.slack.com/apps> → Create New App → From scratch
//! 2. Under **Socket Mode** → Enable Socket Mode → generate App-Level Token (`xapp-…`)
//! 3. Under **OAuth & Permissions** → add Bot Token Scopes:
//!    - `channels:history`, `groups:history`, `im:history`, `mpim:history`
//!    - `chat:write`
//! 4. Install to Workspace → copy the Bot User OAuth Token (`xoxb-…`)
//! 5. Under **Event Subscriptions** → Subscribe to Bot Events:
//!    `message.channels`, `message.im`, `message.groups`
//!
//! # Why Socket Mode?
//!
//! Socket Mode uses an outbound WebSocket connection, so no public
//! HTTPS endpoint is required — ideal for desktop applications.
//!
//! # Keyring keys
//!
//! | Key | Description |
//! |-----|-------------|
//! | `channel:slack:bot_token` | Bot User OAuth Token (`xoxb-…`) |
//! | `channel:slack:app_token` | App-Level Token for Socket Mode (`xapp-…`) |
//! | `channel:slack:allowed_channel_ids` | Comma-separated Slack channel IDs |

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::traits::{Channel, ChannelMessage};

// ─── SlackConfig ──────────────────────────────────────────────────────────────

/// Configuration for [`SlackChannel`].
#[derive(Debug, Clone)]
pub struct SlackConfig {
    /// Bot User OAuth Token (`xoxb-…`) for API calls and posting.
    pub bot_token: String,
    /// App-Level Token (`xapp-…`) for Socket Mode WebSocket connection.
    pub app_token: String,
    /// Only messages from these Slack channel IDs are forwarded.
    /// Empty list means all channels are accepted.
    pub allowed_channel_ids: Vec<String>,
}

impl SlackConfig {
    /// Create a config with the given tokens and no allow-list restrictions.
    pub fn new(bot_token: impl Into<String>, app_token: impl Into<String>) -> Self {
        Self {
            bot_token: bot_token.into(),
            app_token: app_token.into(),
            allowed_channel_ids: Vec::new(),
        }
    }

    /// Create a config with an explicit channel allow-list.
    pub fn with_allowed_channels(
        bot_token: impl Into<String>,
        app_token: impl Into<String>,
        allowed_channel_ids: Vec<String>,
    ) -> Self {
        Self {
            bot_token: bot_token.into(),
            app_token: app_token.into(),
            allowed_channel_ids,
        }
    }
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self::new("", "")
    }
}

// ─── SlackChannel ─────────────────────────────────────────────────────────────

/// A [`Channel`] implementation backed by the Slack Web API and Socket Mode.
///
/// Uses slack-morphism for API calls and Socket Mode WebSocket connection.
/// All IO is feature-gated behind `channels-slack`.
pub struct SlackChannel {
    bot_token: String,
    app_token: String,
    allowed_channel_ids: Vec<String>,
}

impl SlackChannel {
    /// Construct a new channel from the given config.
    pub fn new(config: SlackConfig) -> Self {
        Self {
            bot_token: config.bot_token,
            app_token: config.app_token,
            allowed_channel_ids: config.allowed_channel_ids,
        }
    }

    /// Check whether a Slack channel ID passes the allow-list (empty = allow all).
    pub fn is_channel_allowed(&self, channel_id: &str) -> bool {
        self.allowed_channel_ids.is_empty()
            || self.allowed_channel_ids.iter().any(|c| c == channel_id)
    }
}

// ─── Channel trait impl ───────────────────────────────────────────────────────

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    /// Send `message` to the given Slack channel ID.
    ///
    /// `recipient` must be a Slack channel ID (e.g. `C01234567AB`).
    async fn send(&self, message: &str, recipient: Option<&str>) -> Result<(), String> {
        let Some(channel_id) = recipient else {
            return Err("slack send: recipient (channel_id) is required".into());
        };

        #[cfg(feature = "channels-slack")]
        return self.post_message(channel_id, message).await;

        #[cfg(not(feature = "channels-slack"))]
        {
            let _ = channel_id;
            Err("slack channel not compiled (missing `channels-slack` feature)".into())
        }
    }

    /// Connect via Socket Mode and begin receiving Slack messages.
    ///
    /// Silently ignores messages from channels not on the allow-list.
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
        #[cfg(feature = "channels-slack")]
        return self.run_socket_mode(tx).await;

        #[cfg(not(feature = "channels-slack"))]
        {
            let _ = tx;
            Err("slack channel not compiled (missing `channels-slack` feature)".into())
        }
    }

    /// Perform a health check by calling `auth.test`.
    async fn health_check(&self) -> bool {
        #[cfg(feature = "channels-slack")]
        {
            self.auth_test().await
        }
        #[cfg(not(feature = "channels-slack"))]
        false
    }
}

// ─── Slack API helpers (channels-slack only) ──────────────────────────────────

#[cfg(feature = "channels-slack")]
impl SlackChannel {
    /// Post a message to the given Slack channel via `chat.postMessage`.
    async fn post_message(&self, channel_id: &str, message: &str) -> Result<(), String> {
        use slack_morphism::hyper_tokio::SlackClientHyperConnector;
        use slack_morphism::prelude::*;

        let connector =
            SlackClientHyperConnector::new().map_err(|e| format!("slack: connector error: {e}"))?;
        let client = SlackClient::new(connector);
        let token = SlackApiToken::new(SlackApiTokenValue::new(self.bot_token.clone()));
        let session = client.open_session(&token);

        let request = SlackApiChatPostMessageRequest::new(
            SlackChannelId::new(channel_id.to_string()),
            SlackMessageContent::new().with_text(message.to_string()),
        );

        session
            .chat_post_message(&request)
            .await
            .map_err(|e| format!("slack send error: {e}"))?;

        Ok(())
    }

    /// Listen for Slack messages using Socket Mode (no public URL needed).
    ///
    /// State (mpsc sender + allow-list) is threaded through the user_state
    /// storage so the function-pointer callback can access it.
    async fn run_socket_mode(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
        use slack_morphism::hyper_tokio::{
            SlackClientHyperConnector, SlackClientHyperHttpsConnector,
        };
        use slack_morphism::prelude::*;
        use std::sync::Arc;

        /// Per-listener state stored in `SlackClientEventsUserState`.
        struct SlackListenState {
            tx: mpsc::Sender<ChannelMessage>,
            allowed_channel_ids: Vec<String>,
        }

        // Function-pointer callback (no closure captures allowed here).
        async fn handle_push_events(
            ev: SlackPushEventCallback,
            _client: Arc<SlackClient<SlackClientHyperHttpsConnector>>,
            states: SlackClientEventsUserState,
        ) -> UserCallbackResult<()> {
            if let SlackEventCallbackBody::Message(msg_event) = &ev.event {
                // Skip bot messages.
                if msg_event.sender.bot_id.is_some() {
                    return Ok(());
                }

                // Clone what we need while holding the lock briefly.
                let (tx, allowed) = {
                    let guard = states.read().await;
                    match guard.get_user_state::<SlackListenState>() {
                        Some(s) => (s.tx.clone(), s.allowed_channel_ids.clone()),
                        None => return Ok(()),
                    }
                    // guard drops here, lock released
                };

                let channel_id = msg_event
                    .origin
                    .channel
                    .as_ref()
                    .map(|c| c.to_string())
                    .unwrap_or_default();

                // Channel allow-list check.
                if !allowed.is_empty() && !allowed.iter().any(|c| c == &channel_id) {
                    return Ok(());
                }

                let content = msg_event
                    .content
                    .as_ref()
                    .and_then(|c| c.text.as_ref())
                    .cloned()
                    .unwrap_or_default();

                let sender = msg_event
                    .sender
                    .user
                    .as_ref()
                    .map(|u| u.to_string())
                    .unwrap_or_default();

                let channel_msg = ChannelMessage::new("slack", content).with_sender(sender);
                let _ = tx.send(channel_msg).await;
            }
            Ok(())
        }

        let connector =
            SlackClientHyperConnector::new().map_err(|e| format!("slack: connector error: {e}"))?;
        let client = Arc::new(SlackClient::new(connector));

        let env = SlackClientEventsListenerEnvironment::new(Arc::clone(&client));
        env.user_state
            .write()
            .await
            .set_user_state(SlackListenState {
                tx,
                allowed_channel_ids: self.allowed_channel_ids.clone(),
            });
        let env = Arc::new(env);

        let callbacks = SlackSocketModeListenerCallbacks::<SlackClientHyperHttpsConnector>::new()
            .with_push_events(handle_push_events);

        let listener = SlackClientSocketModeListener::new(
            &SlackClientSocketModeConfig::new(),
            Arc::clone(&env),
            callbacks,
        );

        let app_token = SlackApiToken::new(SlackApiTokenValue::new(self.app_token.clone()));

        listener
            .listen_for(&app_token)
            .await
            .map_err(|e| format!("slack: socket mode listen error: {e}"))?;

        listener.serve().await;
        Ok(())
    }

    /// Call `auth.test` to verify the bot token is valid.
    async fn auth_test(&self) -> bool {
        use slack_morphism::hyper_tokio::SlackClientHyperConnector;
        use slack_morphism::prelude::*;

        let Ok(connector) = SlackClientHyperConnector::new() else {
            return false;
        };
        let client = SlackClient::new(connector);
        let token = SlackApiToken::new(SlackApiTokenValue::new(self.bot_token.clone()));
        let session = client.open_session(&token);
        session.auth_test().await.is_ok()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn channel() -> SlackChannel {
        SlackChannel::new(SlackConfig::new("xoxb-test-token", "xapp-test-token"))
    }

    fn channel_with_allowlist(channels: Vec<String>) -> SlackChannel {
        SlackChannel::new(SlackConfig::with_allowed_channels(
            "xoxb-test",
            "xapp-test",
            channels,
        ))
    }

    // ── Identity ───────────────────────────────────────────────────────────────

    #[test]
    fn channel_name_is_slack() {
        assert_eq!(channel().name(), "slack");
    }

    // ── Config construction ───────────────────────────────────────────────────

    #[test]
    fn config_new_stores_tokens() {
        let cfg = SlackConfig::new("xoxb-abc", "xapp-xyz");
        assert_eq!(cfg.bot_token, "xoxb-abc");
        assert_eq!(cfg.app_token, "xapp-xyz");
        assert!(cfg.allowed_channel_ids.is_empty());
    }

    #[test]
    fn config_with_allowed_channels_stores_list() {
        let ids = vec!["C01".to_string(), "C02".to_string()];
        let cfg = SlackConfig::with_allowed_channels("xoxb", "xapp", ids.clone());
        assert_eq!(cfg.allowed_channel_ids, ids);
    }

    #[test]
    fn config_default_has_empty_tokens() {
        let cfg = SlackConfig::default();
        assert!(cfg.bot_token.is_empty());
        assert!(cfg.app_token.is_empty());
    }

    // ── Channel allow-list ────────────────────────────────────────────────────

    #[test]
    fn empty_allowlist_accepts_all_channels() {
        let ch = channel_with_allowlist(vec![]);
        assert!(ch.is_channel_allowed("C01234567"));
        assert!(ch.is_channel_allowed("G98765432"));
    }

    #[test]
    fn allowlist_blocks_unlisted_channels() {
        let ch = channel_with_allowlist(vec!["C01".to_string()]);
        assert!(ch.is_channel_allowed("C01"));
        assert!(!ch.is_channel_allowed("C02"));
    }

    #[test]
    fn allowlist_is_case_sensitive() {
        let ch = channel_with_allowlist(vec!["C01AbC".to_string()]);
        assert!(!ch.is_channel_allowed("c01abc"));
    }

    // ── send() without feature ────────────────────────────────────────────────

    #[tokio::test]
    async fn send_without_recipient_returns_error() {
        let ch = channel();
        let result = ch.send("hello", None).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("recipient"));
    }

    // ── health_check without feature ──────────────────────────────────────────

    #[tokio::test]
    async fn health_check_returns_false_without_feature() {
        let ch = channel();
        #[cfg(not(feature = "channels-slack"))]
        assert!(!ch.health_check().await);
        #[cfg(feature = "channels-slack")]
        let _ = ch;
    }
}
