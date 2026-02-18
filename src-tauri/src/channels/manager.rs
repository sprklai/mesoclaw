//! `ChannelManager` — lifecycle management for registered channels.
//!
//! The manager:
//! - Keeps a registry of [`Channel`] instances.
//! - Aggregates inbound messages from all channels into a single receiver.
//! - Monitors channel health and logs disconnections.
//! - Exposes a broadcast interface for sending to a named channel.

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{RwLock, mpsc};

use super::traits::{Channel, ChannelMessage};

// ─── ChannelManager ───────────────────────────────────────────────────────────

/// Manages the lifecycle of registered channels and aggregates inbound messages.
pub struct ChannelManager {
    channels: Arc<RwLock<HashMap<String, Arc<dyn Channel>>>>,
}

impl ChannelManager {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a channel.  Returns an error if a channel with the same name
    /// already exists.
    pub async fn register(&self, channel: Arc<dyn Channel>) -> Result<(), String> {
        let name = channel.name().to_string();
        let mut map = self.channels.write().await;
        if map.contains_key(&name) {
            return Err(format!("channel '{name}' is already registered"));
        }
        map.insert(name, channel);
        Ok(())
    }

    /// Unregister a channel by name.
    pub async fn unregister(&self, name: &str) -> bool {
        self.channels.write().await.remove(name).is_some()
    }

    /// Return the names of all registered channels.
    pub async fn channel_names(&self) -> Vec<String> {
        self.channels.read().await.keys().cloned().collect()
    }

    /// Return the number of registered channels.
    pub async fn len(&self) -> usize {
        self.channels.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.channels.read().await.is_empty()
    }

    /// Check the health of all registered channels.  Returns a map of
    /// `channel_name → is_healthy`.
    pub async fn health_all(&self) -> HashMap<String, bool> {
        let channels = self.channels.read().await;
        let mut result = HashMap::new();
        for (name, ch) in channels.iter() {
            result.insert(name.clone(), ch.health_check().await);
        }
        result
    }

    /// Send a message via the named channel.
    pub async fn send(
        &self,
        channel_name: &str,
        message: &str,
        recipient: Option<&str>,
    ) -> Result<(), String> {
        let channels = self.channels.read().await;
        let ch = channels
            .get(channel_name)
            .ok_or_else(|| format!("channel '{channel_name}' not found"))?;
        ch.send(message, recipient).await
    }

    /// Start listening on all registered channels.  All inbound messages are
    /// forwarded to the returned `mpsc::Receiver`.
    ///
    /// Each channel is given a clone of the same sender, so a single receiver
    /// aggregates messages from all channels.  The returned receiver can be
    /// polled from the agent loop.
    pub async fn start_all(
        &self,
        buffer: usize,
    ) -> (
        mpsc::Receiver<ChannelMessage>,
        Vec<tokio::task::JoinHandle<()>>,
    ) {
        let (tx, rx) = mpsc::channel::<ChannelMessage>(buffer);
        let channels = self.channels.read().await;

        let mut handles = Vec::new();
        for (name, ch) in channels.iter() {
            let tx_clone = tx.clone();
            let ch_clone = Arc::clone(ch);
            let name_clone = name.clone();
            let handle = tokio::spawn(async move {
                if let Err(e) = ch_clone.listen(tx_clone).await {
                    log::warn!("Channel '{name_clone}' listen error: {e}");
                }
            });
            handles.push(handle);
        }

        (rx, handles)
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::traits::ChannelMessage;
    use async_trait::async_trait;
    use tokio::sync::mpsc;

    /// Minimal test double that always succeeds.
    struct AlwaysHealthy {
        id: String,
    }

    #[async_trait]
    impl Channel for AlwaysHealthy {
        fn name(&self) -> &str {
            &self.id
        }
        async fn send(&self, _msg: &str, _r: Option<&str>) -> Result<(), String> {
            Ok(())
        }
        async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
            // Immediately return (no messages).
            Ok(())
        }
        async fn health_check(&self) -> bool {
            true
        }
    }

    /// Test double that always reports unhealthy.
    struct AlwaysUnhealthy {
        id: String,
    }

    #[async_trait]
    impl Channel for AlwaysUnhealthy {
        fn name(&self) -> &str {
            &self.id
        }
        async fn send(&self, _msg: &str, _r: Option<&str>) -> Result<(), String> {
            Err("unhealthy".into())
        }
        async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
            Err("unhealthy".into())
        }
        async fn health_check(&self) -> bool {
            false
        }
    }

    fn healthy(id: &str) -> Arc<dyn Channel> {
        Arc::new(AlwaysHealthy { id: id.to_string() })
    }

    fn unhealthy(id: &str) -> Arc<dyn Channel> {
        Arc::new(AlwaysUnhealthy { id: id.to_string() })
    }

    #[tokio::test]
    async fn register_and_len() {
        let mgr = ChannelManager::new();
        mgr.register(healthy("a")).await.unwrap();
        mgr.register(healthy("b")).await.unwrap();
        assert_eq!(mgr.len().await, 2);
        assert!(!mgr.is_empty().await);
    }

    #[tokio::test]
    async fn duplicate_register_returns_error() {
        let mgr = ChannelManager::new();
        mgr.register(healthy("ch")).await.unwrap();
        let result = mgr.register(healthy("ch")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn unregister_removes_channel() {
        let mgr = ChannelManager::new();
        mgr.register(healthy("ch")).await.unwrap();
        assert!(mgr.unregister("ch").await);
        assert!(mgr.is_empty().await);
        // Unregistering a non-existent channel returns false.
        assert!(!mgr.unregister("ch").await);
    }

    #[tokio::test]
    async fn channel_names_returns_all() {
        let mgr = ChannelManager::new();
        mgr.register(healthy("x")).await.unwrap();
        mgr.register(healthy("y")).await.unwrap();
        let mut names = mgr.channel_names().await;
        names.sort();
        assert_eq!(names, vec!["x", "y"]);
    }

    #[tokio::test]
    async fn health_all_reports_individual_status() {
        let mgr = ChannelManager::new();
        mgr.register(healthy("ok")).await.unwrap();
        mgr.register(unhealthy("bad")).await.unwrap();
        let health = mgr.health_all().await;
        assert_eq!(health["ok"], true);
        assert_eq!(health["bad"], false);
    }

    #[tokio::test]
    async fn send_unknown_channel_returns_error() {
        let mgr = ChannelManager::new();
        let result = mgr.send("missing", "hello", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_known_channel_delegates() {
        let mgr = ChannelManager::new();
        mgr.register(healthy("ch")).await.unwrap();
        assert!(mgr.send("ch", "hello", None).await.is_ok());
    }

    #[tokio::test]
    async fn start_all_returns_receiver_and_handles() {
        let mgr = ChannelManager::new();
        mgr.register(healthy("a")).await.unwrap();
        mgr.register(healthy("b")).await.unwrap();
        let (_rx, handles) = mgr.start_all(32).await;
        assert_eq!(handles.len(), 2);
        // Wait for the immediate-return channels to complete.
        for h in handles {
            h.await.unwrap_or_default();
        }
    }

    #[tokio::test]
    async fn default_is_empty() {
        let mgr = ChannelManager::default();
        assert!(mgr.is_empty().await);
    }
}
