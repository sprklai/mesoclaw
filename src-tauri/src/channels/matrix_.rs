//! Matrix channel integration for MesoClaw (Phase 7.3).
//!
//! Requires the `channels-matrix` Cargo feature:
//!
//! ```sh
//! cargo build --features channels-matrix
//! cargo test  --features channels-matrix -- channels::matrix_
//! ```
//!
//! # Architecture
//!
//! ```text
//!  Matrix homeserver (sync loop)  ──────────▶  MatrixChannel::listen()
//!                                                     │
//!                                         allowed_room_ids check
//!                                                     │
//!                                       mpsc::Sender<ChannelMessage>
//!                                                     │
//!                                               ChannelManager
//!                                                     │
//!                                              Agent loop
//! ```
//!
//! # Matrix / Element setup
//!
//! 1. Create a bot account on any Matrix homeserver (e.g. matrix.org)
//! 2. Generate an access token via Element's Developer Tools or the CS API:
//!    `POST /_matrix/client/v3/login` with `"type":"m.login.password"`
//! 3. Copy the returned `access_token` (stored in OS keyring)
//! 4. Invite the bot account to rooms it should monitor
//!
//! # Matrix as a bridge hub
//!
//! Matrix supports protocol bridges to WhatsApp, Slack, IRC, Signal, and more
//! via bridge bots.  A single Matrix account can aggregate messages from all
//! these platforms into one channel — enabling MesoClaw to reach them through
//! this single integration.
//!
//! # Keyring keys
//!
//! | Key | Description |
//! |-----|-------------|
//! | `channel:matrix:homeserver_url` | Full homeserver URL (e.g. `https://matrix.org`) |
//! | `channel:matrix:username` | Bot MXID (e.g. `@bot:matrix.org`) |
//! | `channel:matrix:access_token` | Access token from login response |
//! | `channel:matrix:allowed_room_ids` | Comma-separated room IDs (e.g. `!abc:matrix.org`) |

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::traits::{Channel, ChannelMessage};

// ─── MatrixConfig ─────────────────────────────────────────────────────────────

/// Configuration for [`MatrixChannel`].
#[derive(Debug, Clone)]
pub struct MatrixConfig {
    /// Full URL of the Matrix homeserver (e.g. `https://matrix.org`).
    pub homeserver_url: String,
    /// Bot user MXID (e.g. `@mybot:matrix.org`).
    pub username: String,
    /// Access token obtained from the Matrix login API.
    pub access_token: String,
    /// Only messages from these room IDs are forwarded.
    /// Empty list means all rooms are accepted.
    pub allowed_room_ids: Vec<String>,
}

impl MatrixConfig {
    /// Create a config with the given homeserver, username, and access token.
    pub fn new(
        homeserver_url: impl Into<String>,
        username: impl Into<String>,
        access_token: impl Into<String>,
    ) -> Self {
        Self {
            homeserver_url: homeserver_url.into(),
            username: username.into(),
            access_token: access_token.into(),
            allowed_room_ids: Vec::new(),
        }
    }

    /// Create a config with an explicit room allow-list.
    pub fn with_allowed_rooms(
        homeserver_url: impl Into<String>,
        username: impl Into<String>,
        access_token: impl Into<String>,
        allowed_room_ids: Vec<String>,
    ) -> Self {
        Self {
            homeserver_url: homeserver_url.into(),
            username: username.into(),
            access_token: access_token.into(),
            allowed_room_ids,
        }
    }
}

impl Default for MatrixConfig {
    fn default() -> Self {
        Self::new("", "", "")
    }
}

// ─── MatrixChannel ────────────────────────────────────────────────────────────

/// A [`Channel`] implementation backed by the Matrix Client-Server API.
///
/// Uses matrix-sdk for the sync loop and HTTP sending.
/// All IO is feature-gated behind `channels-matrix`.
pub struct MatrixChannel {
    homeserver_url: String,
    username: String,
    access_token: String,
    allowed_room_ids: Vec<String>,
}

impl MatrixChannel {
    /// Construct a new channel from the given config.
    pub fn new(config: MatrixConfig) -> Self {
        Self {
            homeserver_url: config.homeserver_url,
            username: config.username,
            access_token: config.access_token,
            allowed_room_ids: config.allowed_room_ids,
        }
    }

    /// Check whether a room ID passes the allow-list (empty = allow all).
    pub fn is_room_allowed(&self, room_id: &str) -> bool {
        self.allowed_room_ids.is_empty() || self.allowed_room_ids.iter().any(|r| r == room_id)
    }
}

// ─── Channel trait impl ───────────────────────────────────────────────────────

#[async_trait]
impl Channel for MatrixChannel {
    fn name(&self) -> &str {
        "matrix"
    }

    /// Send `message` to the given Matrix room ID.
    ///
    /// `recipient` must be a Matrix room ID (e.g. `!abc123:matrix.org`).
    async fn send(&self, message: &str, recipient: Option<&str>) -> Result<(), String> {
        let Some(room_id_str) = recipient else {
            return Err("matrix send: recipient (room_id) is required".into());
        };

        #[cfg(feature = "channels-matrix")]
        return self.send_to_room(room_id_str, message).await;

        #[cfg(not(feature = "channels-matrix"))]
        {
            let _ = room_id_str;
            Err("matrix channel not compiled (missing `channels-matrix` feature)".into())
        }
    }

    /// Begin the Matrix sync loop and forward inbound room messages.
    ///
    /// Silently ignores rooms not on the allow-list.
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
        #[cfg(feature = "channels-matrix")]
        return self.run_sync_loop(tx).await;

        #[cfg(not(feature = "channels-matrix"))]
        {
            let _ = tx;
            Err("matrix channel not compiled (missing `channels-matrix` feature)".into())
        }
    }

    /// Perform a health check by calling `/whoami`.
    async fn health_check(&self) -> bool {
        #[cfg(feature = "channels-matrix")]
        {
            self.whoami_check().await
        }
        #[cfg(not(feature = "channels-matrix"))]
        false
    }
}

// ─── Matrix API helpers (channels-matrix only) ───────────────────────────────

#[cfg(feature = "channels-matrix")]
impl MatrixChannel {
    /// Build a logged-in Matrix client using the stored access token.
    async fn build_client(&self) -> Result<matrix_sdk::Client, String> {
        use matrix_sdk::Client;
        use matrix_sdk::SessionMeta;
        use matrix_sdk::matrix_auth::{MatrixSession, MatrixSessionTokens};
        use matrix_sdk::ruma::{DeviceId, UserId};

        let client = Client::builder()
            .homeserver_url(&self.homeserver_url)
            .build()
            .await
            .map_err(|e| format!("matrix: failed to build client: {e}"))?;

        let user_id = UserId::parse(&self.username)
            .map_err(|e| format!("matrix: invalid username '{}': {e}", self.username))?;

        let session = MatrixSession {
            meta: SessionMeta {
                user_id,
                device_id: DeviceId::new(),
            },
            tokens: MatrixSessionTokens {
                access_token: self.access_token.clone(),
                refresh_token: None,
            },
        };

        client
            .restore_session(session)
            .await
            .map_err(|e| format!("matrix: restore_session failed: {e}"))?;

        Ok(client)
    }

    /// Send a plain-text message to the given room.
    async fn send_to_room(&self, room_id_str: &str, message: &str) -> Result<(), String> {
        use matrix_sdk::ruma::RoomId;
        use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;

        let client = self.build_client().await?;

        let room_id = RoomId::parse(room_id_str)
            .map_err(|e| format!("matrix send: invalid room_id '{room_id_str}': {e}"))?;

        let room = client
            .get_room(&room_id)
            .ok_or_else(|| format!("matrix send: room '{room_id_str}' not found"))?;

        room.send(RoomMessageEventContent::text_plain(message))
            .await
            .map_err(|e| format!("matrix send error: {e}"))?;

        Ok(())
    }

    /// Run the Matrix sync loop, forwarding inbound messages to `tx`.
    async fn run_sync_loop(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
        use matrix_sdk::Room;
        use matrix_sdk::config::SyncSettings;
        use matrix_sdk::ruma::events::room::message::{MessageType, OriginalSyncRoomMessageEvent};
        use std::sync::Arc;
        use tokio::sync::Mutex;

        let client = self.build_client().await?;
        let allowed = Arc::new(self.allowed_room_ids.clone());
        let tx = Arc::new(Mutex::new(tx));

        client.add_event_handler({
            let allowed = Arc::clone(&allowed);
            let tx = Arc::clone(&tx);
            move |ev: OriginalSyncRoomMessageEvent, room: Room| {
                let allowed = Arc::clone(&allowed);
                let tx = Arc::clone(&tx);
                async move {
                    let room_id = room.room_id().to_string();

                    // Room allow-list check.
                    if !allowed.is_empty() && !allowed.iter().any(|r| r == &room_id) {
                        return;
                    }

                    if let MessageType::Text(text) = &ev.content.msgtype {
                        let sender = ev.sender.to_string();
                        let content = text.body.clone();
                        let channel_msg =
                            ChannelMessage::new("matrix", content).with_sender(sender);
                        let tx_guard = tx.lock().await;
                        let _ = tx_guard.send(channel_msg).await;
                    }
                }
            }
        });

        log::info!("matrix: starting sync loop");
        client
            .sync(SyncSettings::default())
            .await
            .map_err(|e| format!("matrix: sync error: {e}"))?;

        Ok(())
    }

    /// Call `/whoami` to verify the access token is still valid.
    async fn whoami_check(&self) -> bool {
        match self.build_client().await {
            Ok(client) => client.whoami().await.is_ok(),
            Err(_) => false,
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn channel() -> MatrixChannel {
        MatrixChannel::new(MatrixConfig::new(
            "https://matrix.org",
            "@bot:matrix.org",
            "test-token",
        ))
    }

    fn channel_with_rooms(rooms: Vec<String>) -> MatrixChannel {
        MatrixChannel::new(MatrixConfig::with_allowed_rooms(
            "https://matrix.org",
            "@bot:matrix.org",
            "test-token",
            rooms,
        ))
    }

    // ── Identity ───────────────────────────────────────────────────────────────

    #[test]
    fn channel_name_is_matrix() {
        assert_eq!(channel().name(), "matrix");
    }

    // ── Config construction ───────────────────────────────────────────────────

    #[test]
    fn config_new_stores_credentials() {
        let cfg = MatrixConfig::new("https://example.com", "@user:example.com", "tok123");
        assert_eq!(cfg.homeserver_url, "https://example.com");
        assert_eq!(cfg.username, "@user:example.com");
        assert_eq!(cfg.access_token, "tok123");
        assert!(cfg.allowed_room_ids.is_empty());
    }

    #[test]
    fn config_with_allowed_rooms_stores_rooms() {
        let rooms = vec!["!abc:matrix.org".to_string(), "!def:matrix.org".to_string()];
        let cfg = MatrixConfig::with_allowed_rooms(
            "https://matrix.org",
            "@b:m.org",
            "tok",
            rooms.clone(),
        );
        assert_eq!(cfg.allowed_room_ids, rooms);
    }

    #[test]
    fn config_default_has_empty_fields() {
        let cfg = MatrixConfig::default();
        assert!(cfg.homeserver_url.is_empty());
        assert!(cfg.username.is_empty());
        assert!(cfg.access_token.is_empty());
    }

    // ── Room allow-list ────────────────────────────────────────────────────────

    #[test]
    fn empty_room_allowlist_allows_all() {
        let ch = channel_with_rooms(vec![]);
        assert!(ch.is_room_allowed("!any:matrix.org"));
        assert!(ch.is_room_allowed("!xyz:example.com"));
    }

    #[test]
    fn room_allowlist_allows_only_listed_rooms() {
        let ch = channel_with_rooms(vec!["!abc:matrix.org".to_string()]);
        assert!(ch.is_room_allowed("!abc:matrix.org"));
        assert!(!ch.is_room_allowed("!other:matrix.org"));
    }

    #[test]
    fn room_allowlist_is_exact_match() {
        let ch = channel_with_rooms(vec!["!room1:server.com".to_string()]);
        assert!(!ch.is_room_allowed("!room1:other.com"));
        assert!(!ch.is_room_allowed("!ROOM1:server.com")); // case-sensitive
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
        #[cfg(not(feature = "channels-matrix"))]
        assert!(!ch.health_check().await);
        #[cfg(feature = "channels-matrix")]
        let _ = ch;
    }
}
