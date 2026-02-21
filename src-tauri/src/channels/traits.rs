//! Core channel abstractions for MesoClaw inter-agent messaging.
//!
//! A **channel** is any transport layer over which [`ChannelMessage`]s flow
//! between the agent runtime and an external peer (user, Tauri IPC, webhook,
//! Telegram, etc.).
//!
//! # Implementing a Channel
//!
//! ```rust,ignore
//! use async_trait::async_trait;
//! use crate::channels::traits::{Channel, ChannelMessage, ChannelEvent};
//!
//! struct MyChannel;
//!
//! #[async_trait]
//! impl Channel for MyChannel {
//!     fn name(&self) -> &str { "my-channel" }
//!     async fn send(&self, msg: &str, recipient: Option<&str>) -> Result<(), String> { Ok(()) }
//!     async fn listen(&self, tx: tokio::sync::mpsc::Sender<ChannelMessage>) -> Result<(), String> { Ok(()) }
//!     async fn health_check(&self) -> bool { true }
//! }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// ─── ChannelMessage ───────────────────────────────────────────────────────────

/// A message received from (or sent to) a channel peer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelMessage {
    /// Which channel this message came from.
    pub channel: String,
    /// Optional peer identifier (user ID, chat ID, webhook source, etc.).
    pub sender: Option<String>,
    /// The message payload.
    pub content: String,
    /// ISO-8601 timestamp.
    pub timestamp: String,
    /// Optional metadata (e.g. message ID for reply threading).
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl ChannelMessage {
    /// Convenience constructor with a UTC timestamp.
    pub fn new(channel: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            sender: None,
            content: content.into(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_sender(mut self, sender: impl Into<String>) -> Self {
        self.sender = Some(sender.into());
        self
    }

    pub fn with_metadata(mut self, metadata: std::collections::HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }
}

// ─── ChannelEvent ─────────────────────────────────────────────────────────────

/// Lifecycle events emitted by a channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChannelEvent {
    /// Channel is online and accepting messages.
    Connected { channel: String },
    /// Channel has disconnected; the manager will attempt reconnection.
    Disconnected { channel: String, reason: String },
    /// A message was received.
    MessageReceived(ChannelMessage),
    /// A message was sent successfully.
    MessageSent {
        channel: String,
        recipient: Option<String>,
    },
    /// An error occurred (non-fatal).
    Error { channel: String, error: String },
}

// ─── Channel trait ────────────────────────────────────────────────────────────

/// Transport abstraction for sending and receiving messages.
///
/// Implementations must be `Send + Sync` so they can be stored in a shared
/// [`ChannelManager`] behind an `Arc`.
#[async_trait]
pub trait Channel: Send + Sync {
    /// A unique identifier for this channel type (e.g. `"tauri-ipc"`, `"telegram"`).
    fn name(&self) -> &str;

    /// Send a message to the specified recipient (or the default recipient if `None`).
    async fn send(&self, message: &str, recipient: Option<&str>) -> Result<(), String>;

    /// Begin listening for inbound messages, forwarding them on `tx`.
    ///
    /// This method should run until the channel disconnects or the `tx` is dropped.
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> Result<(), String>;

    /// Perform a lightweight health check.  Returns `true` when the channel is
    /// operational.
    async fn health_check(&self) -> bool;
}
