pub mod api;
pub mod fmt;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::{mpsc, watch};
use tracing::{debug, error, info, warn};

use crate::Result;
use crate::credential::CredentialStore;
use crate::error::MesoError;

use super::message::ChannelMessage;
use super::traits::{Channel, ChannelLifecycle, ChannelSender, ChannelStatus};

// Status values
const STATUS_DISCONNECTED: u8 = 0;
const STATUS_CONNECTING: u8 = 1;
const STATUS_CONNECTED: u8 = 2;

/// Slack channel using raw Socket Mode WebSocket (no slack-morphism).
pub struct SlackChannel {
    display_name: String,
    /// Bot user ID for echo-loop prevention. Set at build time via `with_bot_id()`,
    /// or auto-resolved from `auth.test` during `connect()`.
    bot_id: Arc<tokio::sync::OnceCell<String>>,
    allowed_channel_ids: Vec<String>,
    status: AtomicU8,
    credentials: Arc<dyn CredentialStore>,
    http_client: reqwest::Client,
    bot_token: tokio::sync::OnceCell<String>,
    app_token: tokio::sync::OnceCell<String>,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    max_reconnect_attempts: u32,
}

impl SlackChannel {
    pub fn new(credentials: Arc<dyn CredentialStore>) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            display_name: "slack".to_string(),
            bot_id: Arc::new(tokio::sync::OnceCell::new()),
            allowed_channel_ids: vec![],
            status: AtomicU8::new(STATUS_DISCONNECTED),
            credentials,
            http_client: reqwest::Client::new(),
            bot_token: tokio::sync::OnceCell::new(),
            app_token: tokio::sync::OnceCell::new(),
            shutdown_tx,
            shutdown_rx,
            max_reconnect_attempts: 10,
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.display_name = name.to_string();
        self
    }

    pub fn with_bot_id(mut self, bot_id: &str) -> Self {
        self.bot_id = Arc::new(tokio::sync::OnceCell::new());
        let _ = self.bot_id.set(bot_id.to_string());
        self
    }

    pub fn with_allowed_channels(mut self, channels: Vec<String>) -> Self {
        self.allowed_channel_ids = channels;
        self
    }

    pub fn with_max_reconnect_attempts(mut self, n: u32) -> Self {
        self.max_reconnect_attempts = n;
        self
    }

    /// Check if a channel ID is allowed (empty list = allow all).
    pub fn is_channel_allowed(&self, channel_id: &str) -> bool {
        self.allowed_channel_ids.is_empty()
            || self.allowed_channel_ids.iter().any(|c| c == channel_id)
    }

    fn status_from_u8(val: u8) -> ChannelStatus {
        match val {
            STATUS_CONNECTING => ChannelStatus::Connecting,
            STATUS_CONNECTED => ChannelStatus::Connected,
            _ => ChannelStatus::Disconnected,
        }
    }

    /// Get a WebSocket URL from Slack's apps.connections.open API.
    async fn get_ws_url(
        http_client: &reqwest::Client,
        app_token: &str,
    ) -> std::result::Result<String, MesoError> {
        let resp = http_client
            .post("https://slack.com/api/apps.connections.open")
            .bearer_auth(app_token)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .send()
            .await
            .map_err(|e| MesoError::Channel(format!("slack: connections.open failed: {e}")))?;

        let body: serde_json::Value = resp.json().await.map_err(|e| {
            MesoError::Channel(format!("slack: connections.open parse failed: {e}"))
        })?;

        if !body["ok"].as_bool().unwrap_or(false) {
            let err = body["error"].as_str().unwrap_or("unknown");
            return Err(MesoError::Channel(format!(
                "slack: connections.open error: {err}"
            )));
        }

        body["url"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| MesoError::Channel("slack: no url in connections.open response".into()))
    }
}

impl Default for SlackChannel {
    fn default() -> Self {
        // Default requires a credential store — use InMemory for convenience
        Self::new(Arc::new(crate::credential::InMemoryCredentialStore::new()))
    }
}

#[async_trait]
impl ChannelSender for SlackChannel {
    fn channel_type(&self) -> &str {
        "slack"
    }

    async fn send_message(&self, message: ChannelMessage) -> Result<()> {
        let bot_token = self
            .bot_token
            .get()
            .ok_or_else(|| MesoError::Channel("slack: not connected".into()))?;

        let channel_id = message
            .metadata
            .get("channel_id")
            .ok_or_else(|| MesoError::Channel("slack: missing channel_id in metadata".into()))?;

        let formatted = super::slack::fmt::to_mrkdwn(&message.content);
        let mut payload = api::post_message_payload(channel_id, &formatted);

        // Thread reply if thread_ts is present
        if let Some(thread_ts) = message.metadata.get("thread_ts") {
            payload["thread_ts"] = serde_json::Value::String(thread_ts.clone());
        }

        let resp = self
            .http_client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(bot_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MesoError::Channel(format!("slack send failed: {e}")))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MesoError::Channel(format!("slack send parse failed: {e}")))?;

        if !body["ok"].as_bool().unwrap_or(false) {
            let err = body["error"].as_str().unwrap_or("unknown");
            return Err(MesoError::Channel(format!("slack send error: {err}")));
        }

        Ok(())
    }
}

#[async_trait]
impl ChannelLifecycle for SlackChannel {
    fn display_name(&self) -> &str {
        &self.display_name
    }

    async fn connect(&self) -> Result<()> {
        self.status.store(STATUS_CONNECTING, Ordering::SeqCst);

        // Fetch bot_token
        let bot_token = self
            .credentials
            .get("channel:slack:bot_token")
            .await
            .map_err(|e| MesoError::Channel(format!("slack: credential error: {e}")))?
            .ok_or_else(|| {
                self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
                MesoError::Channel("slack: bot_token not configured".into())
            })?;

        // Fetch app_token
        let app_token = self
            .credentials
            .get("channel:slack:app_token")
            .await
            .map_err(|e| MesoError::Channel(format!("slack: credential error: {e}")))?
            .ok_or_else(|| {
                self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
                MesoError::Channel("slack: app_token not configured".into())
            })?;

        // Validate bot token via auth.test
        let resp = self
            .http_client
            .post("https://slack.com/api/auth.test")
            .bearer_auth(&bot_token)
            .send()
            .await
            .map_err(|e| {
                self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
                MesoError::Channel(format!("slack: auth.test failed: {e}"))
            })?;

        let body: serde_json::Value = resp.json().await.map_err(|e| {
            self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
            MesoError::Channel(format!("slack: auth.test parse failed: {e}"))
        })?;

        if !body["ok"].as_bool().unwrap_or(false) {
            self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
            let err = body["error"].as_str().unwrap_or("unknown");
            return Err(MesoError::Channel(format!("slack: auth.test error: {err}")));
        }

        let bot_user_id = body["user_id"].as_str().unwrap_or("").to_string();
        if !bot_user_id.is_empty() {
            let _ = self.bot_id.set(bot_user_id.clone());
        }
        info!("Slack bot connected as user_id={bot_user_id}");

        let _ = self.bot_token.set(bot_token);
        let _ = self.app_token.set(app_token);

        self.status.store(STATUS_CONNECTED, Ordering::SeqCst);
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        let _ = self.shutdown_tx.send(true);
        self.status.store(STATUS_DISCONNECTED, Ordering::SeqCst);
        info!("Slack channel disconnected");
        Ok(())
    }

    fn status(&self) -> ChannelStatus {
        Self::status_from_u8(self.status.load(Ordering::SeqCst))
    }

    fn create_sender(&self) -> Box<dyn ChannelSender> {
        Box::new(SlackSender {
            bot_token: self.bot_token.get().cloned(),
            http_client: self.http_client.clone(),
        })
    }
}

#[async_trait]
impl Channel for SlackChannel {
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
        let app_token = self
            .app_token
            .get()
            .ok_or_else(|| MesoError::Channel("slack: not connected, call connect() first".into()))?
            .clone();

        let bot_id = self.bot_id.clone();
        let bot_id_ref = bot_id.get().cloned();
        let mut shutdown_rx = self.shutdown_rx.clone();
        let http_client = self.http_client.clone();
        let max_attempts = self.max_reconnect_attempts;

        info!("Slack listen loop started (Socket Mode)");

        let mut reconnect_attempts: u32 = 0;

        loop {
            // Check shutdown before attempting connection
            if *shutdown_rx.borrow() {
                break;
            }

            // Get WebSocket URL
            let ws_url = match Self::get_ws_url(&http_client, &app_token).await {
                Ok(url) => {
                    reconnect_attempts = 0;
                    url
                }
                Err(e) => {
                    reconnect_attempts += 1;
                    if reconnect_attempts > max_attempts {
                        error!("Slack: max reconnect attempts reached, giving up");
                        return Err(e);
                    }
                    warn!("Slack: failed to get WS URL (attempt {reconnect_attempts}): {e}");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            // Connect WebSocket
            let ws_stream = match tokio_tungstenite::connect_async(&ws_url).await {
                Ok((stream, _)) => stream,
                Err(e) => {
                    reconnect_attempts += 1;
                    if reconnect_attempts > max_attempts {
                        error!("Slack: max reconnect attempts reached, giving up");
                        return Err(MesoError::Channel(format!("slack: ws connect failed: {e}")));
                    }
                    warn!("Slack: WS connect failed (attempt {reconnect_attempts}): {e}");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("Slack Socket Mode connected");
            let (mut write, mut read) = ws_stream.split();

            loop {
                tokio::select! {
                    biased;

                    Ok(()) = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            info!("Slack listen loop: shutdown signal received");
                            let _ = futures::SinkExt::close(&mut write).await;
                            return Ok(());
                        }
                    }

                    msg_opt = read.next() => {
                        match msg_opt {
                            Some(Ok(ws_msg)) => {
                                if let tokio_tungstenite::tungstenite::Message::Text(text) = ws_msg
                                    && let Ok(envelope) = serde_json::from_str::<serde_json::Value>(&text)
                                {
                                    // ACK the envelope immediately
                                    if let Some(envelope_id) = envelope["envelope_id"].as_str() {
                                        let ack = api::envelope_ack(envelope_id);
                                        let ack_msg = tokio_tungstenite::tungstenite::Message::Text(
                                            serde_json::to_string(&ack).unwrap_or_default().into()
                                        );
                                        if let Err(e) = futures::SinkExt::send(&mut write, ack_msg).await {
                                            warn!("Slack: failed to ACK envelope: {e}");
                                        }
                                    }

                                    // Process events_api type envelopes
                                    if envelope["type"].as_str() == Some("events_api")
                                        && let Some(event) = envelope["payload"]["event"].as_object()
                                        && event.get("type").and_then(|t| t.as_str()) == Some("message")
                                        && event.get("subtype").is_none()
                                    {
                                        let text_content = event.get("text").and_then(|t| t.as_str()).unwrap_or("");
                                        let user = event.get("user").and_then(|u| u.as_str()).unwrap_or("");
                                        let channel_id = event.get("channel").and_then(|c| c.as_str()).unwrap_or("");
                                        let thread_ts = event.get("thread_ts").and_then(|t| t.as_str());
                                        let ts = event.get("ts").and_then(|t| t.as_str()).unwrap_or("");

                                        // Skip bot's own messages (echo-loop prevention)
                                        if let Some(ref bid) = bot_id_ref
                                            && user == bid.as_str()
                                        {
                                            continue;
                                        }
                                        // Also skip if bot_id field present (covers bot messages)
                                        if event.get("bot_id").is_some() {
                                            continue;
                                        }

                                        // Check channel allowlist
                                        if !self.is_channel_allowed(channel_id) {
                                            debug!("Slack: dropping message from disallowed channel {channel_id}");
                                            continue;
                                        }

                                        // For non-DM channels, require bot mention
                                        if !api::is_dm_channel(channel_id)
                                            && let Some(ref bid) = bot_id_ref
                                            && !api::contains_bot_mention(text_content, bid)
                                        {
                                            continue;
                                        }

                                        let mut metadata = HashMap::new();
                                        metadata.insert("channel_id".into(), channel_id.to_string());
                                        if let Some(tts) = thread_ts {
                                            metadata.insert("thread_ts".into(), tts.to_string());
                                        } else {
                                            // Reply in thread using the message's ts
                                            metadata.insert("thread_ts".into(), ts.to_string());
                                        }

                                        let channel_msg = ChannelMessage::new("slack", text_content)
                                            .with_sender(user)
                                            .with_metadata(metadata);

                                        if let Err(e) = tx.send(channel_msg).await {
                                            error!("Slack: failed to send to router: {e}");
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => {
                                warn!("Slack WS error: {e}");
                                break; // Reconnect
                            }
                            None => {
                                info!("Slack WS stream ended, reconnecting...");
                                break; // Reconnect
                            }
                        }
                    }
                }
            }

            // Auto-reconnect with backoff
            reconnect_attempts += 1;
            if reconnect_attempts > max_attempts {
                error!("Slack: max reconnect attempts reached, giving up");
                return Err(MesoError::Channel(
                    "slack: max reconnect attempts reached".into(),
                ));
            }
            warn!(
                "Slack: reconnecting (attempt {}/{})",
                reconnect_attempts, max_attempts
            );
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        info!("Slack listen loop stopped");
        Ok(())
    }

    async fn health_check(&self) -> bool {
        if self.status.load(Ordering::SeqCst) != STATUS_CONNECTED {
            return false;
        }
        if let Some(token) = self.bot_token.get() {
            let resp = self
                .http_client
                .post("https://slack.com/api/auth.test")
                .bearer_auth(token)
                .send()
                .await;
            if let Ok(r) = resp
                && let Ok(body) = r.json::<serde_json::Value>().await
            {
                return body["ok"].as_bool().unwrap_or(false);
            }
        }
        false
    }

    async fn on_agent_start(&self, _recipient: Option<&str>) {
        debug!("slack: on_agent_start");
    }

    async fn on_tool_use(&self, tool_name: &str, _recipient: Option<&str>) {
        debug!("slack: on_tool_use ({tool_name})");
    }

    async fn on_agent_complete(&self, _recipient: Option<&str>) {
        debug!("slack: on_agent_complete");
    }
}

/// Lightweight send-only handle for Slack.
struct SlackSender {
    bot_token: Option<String>,
    http_client: reqwest::Client,
}

#[async_trait]
impl ChannelSender for SlackSender {
    fn channel_type(&self) -> &str {
        "slack"
    }

    async fn send_message(&self, message: ChannelMessage) -> Result<()> {
        let bot_token = self
            .bot_token
            .as_ref()
            .ok_or_else(|| MesoError::Channel("slack sender: not connected".into()))?;

        let channel_id = message
            .metadata
            .get("channel_id")
            .ok_or_else(|| MesoError::Channel("slack: missing channel_id in metadata".into()))?;

        let formatted = super::slack::fmt::to_mrkdwn(&message.content);
        let mut payload = api::post_message_payload(channel_id, &formatted);

        if let Some(thread_ts) = message.metadata.get("thread_ts") {
            payload["thread_ts"] = serde_json::Value::String(thread_ts.clone());
        }

        let resp = self
            .http_client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(bot_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| MesoError::Channel(format!("slack send failed: {e}")))?;

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| MesoError::Channel(format!("slack send parse failed: {e}")))?;

        if !body["ok"].as_bool().unwrap_or(false) {
            let err = body["error"].as_str().unwrap_or("unknown");
            return Err(MesoError::Channel(format!("slack send error: {err}")));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::credential::InMemoryCredentialStore;

    fn test_credentials() -> Arc<dyn CredentialStore> {
        Arc::new(InMemoryCredentialStore::new())
    }

    // 8.8.5 — Slack on_agent_start does not panic
    #[tokio::test]
    async fn slack_on_agent_start() {
        let ch = SlackChannel::new(test_credentials());
        ch.on_agent_start(Some("user1")).await;
    }

    // 8.8.6 — Slack on_tool_use does not panic
    #[tokio::test]
    async fn slack_on_tool_use() {
        let ch = SlackChannel::new(test_credentials());
        ch.on_tool_use("web_search", Some("user1")).await;
    }

    // 8.8.7 — Slack on_agent_complete does not panic
    #[tokio::test]
    async fn slack_on_agent_complete() {
        let ch = SlackChannel::new(test_credentials());
        ch.on_agent_complete(None).await;
    }

    #[test]
    fn channel_type_slack() {
        let ch = SlackChannel::new(test_credentials());
        assert_eq!(ch.channel_type(), "slack");
    }

    #[test]
    fn initial_status_disconnected() {
        let ch = SlackChannel::new(test_credentials());
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

    // Connect fails without token
    #[tokio::test]
    async fn connect_fails_without_token() {
        let ch = SlackChannel::new(test_credentials());
        let result = ch.connect().await;
        assert!(result.is_err());
        assert_eq!(ch.status(), ChannelStatus::Disconnected);
    }

    // Send fails without connection
    #[tokio::test]
    async fn send_fails_without_connection() {
        let ch = SlackChannel::new(test_credentials());
        let msg = ChannelMessage::new("slack", "test");
        let result = ch.send_message(msg).await;
        assert!(result.is_err());
    }

    // Listen fails without connection
    #[tokio::test]
    async fn listen_fails_without_connection() {
        let ch = SlackChannel::new(test_credentials());
        let (tx, _rx) = mpsc::channel(10);
        let result = ch.listen(tx).await;
        assert!(result.is_err());
    }

    // Disconnect sends shutdown signal
    #[tokio::test]
    async fn disconnect_sends_shutdown() {
        let ch = SlackChannel::new(test_credentials());
        ch.disconnect().await.unwrap();
        assert_eq!(ch.status(), ChannelStatus::Disconnected);
        assert!(*ch.shutdown_rx.borrow());
    }

    // Channel allowlist
    #[test]
    fn channel_allowlist_filters() {
        let ch = SlackChannel::new(test_credentials())
            .with_allowed_channels(vec!["C123".into(), "C456".into()]);
        assert!(ch.is_channel_allowed("C123"));
        assert!(ch.is_channel_allowed("C456"));
        assert!(!ch.is_channel_allowed("C789"));
    }

    #[test]
    fn empty_allowlist_allows_all() {
        let ch = SlackChannel::new(test_credentials());
        assert!(ch.is_channel_allowed("C123"));
        assert!(ch.is_channel_allowed("anything"));
    }

    // WS2.2a — with_bot_id sets the OnceCell for echo-loop prevention
    #[test]
    fn with_bot_id_sets_once_cell() {
        let ch = SlackChannel::new(test_credentials()).with_bot_id("U12345");
        assert_eq!(ch.bot_id.get(), Some(&"U12345".to_string()));
    }

    // WS2.2b — bot_id is None by default (resolved during connect)
    #[test]
    fn bot_id_default_is_none() {
        let ch = SlackChannel::new(test_credentials());
        assert!(ch.bot_id.get().is_none());
    }
}
