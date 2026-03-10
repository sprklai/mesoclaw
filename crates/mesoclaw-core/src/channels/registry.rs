use std::sync::Arc;

use dashmap::DashMap;

use super::message::ChannelMessage;
use super::traits::{Channel, ChannelSender, ChannelStatus};
use crate::Result;
use crate::error::MesoError;

/// Combined entry holding both the full channel and its send-only handle.
struct ChannelEntry {
    channel: Arc<dyn Channel>,
    sender: Arc<dyn ChannelSender>,
}

/// Concurrent channel registry (single DashMap-backed, atomic register/unregister).
pub struct ChannelRegistry {
    entries: DashMap<String, ChannelEntry>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    /// Register a channel. Creates a sender handle automatically.
    /// Uses `entry()` API for atomic insert (no TOCTOU).
    pub fn register(&self, channel: Arc<dyn Channel>) -> Result<()> {
        let name = channel.display_name().to_string();
        use dashmap::mapref::entry::Entry;
        match self.entries.entry(name.clone()) {
            Entry::Occupied(_) => {
                return Err(MesoError::Channel(format!(
                    "channel already registered: {name}"
                )));
            }
            Entry::Vacant(v) => {
                let sender: Arc<dyn ChannelSender> = Arc::from(channel.create_sender());
                v.insert(ChannelEntry { channel, sender });
            }
        }
        Ok(())
    }

    /// Register a channel, replacing any existing registration with the same name.
    pub fn register_or_replace(&self, channel: Arc<dyn Channel>) -> Result<()> {
        let name = channel.display_name().to_string();
        let sender: Arc<dyn ChannelSender> = Arc::from(channel.create_sender());
        self.entries
            .insert(name, ChannelEntry { channel, sender });
        Ok(())
    }

    /// Unregister a channel by name. Atomically removes both channel and sender.
    pub fn unregister(&self, name: &str) -> bool {
        self.entries.remove(name).is_some()
    }

    /// Get the full channel object by name.
    pub fn get_channel(&self, name: &str) -> Option<Arc<dyn Channel>> {
        self.entries.get(name).map(|r| Arc::clone(&r.value().channel))
    }

    /// Get a lightweight send-only handle for a channel.
    pub fn get_sender(&self, name: &str) -> Option<Arc<dyn ChannelSender>> {
        self.entries.get(name).map(|r| Arc::clone(&r.value().sender))
    }

    /// List all registered channel names.
    pub fn list(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.key().clone()).collect()
    }

    /// Get the status of a channel.
    pub fn status(&self, name: &str) -> Option<ChannelStatus> {
        self.entries.get(name).map(|e| e.value().channel.status())
    }

    /// Connect all registered channels.
    pub async fn connect_all(&self) -> Result<()> {
        let channels: Vec<Arc<dyn Channel>> = self
            .entries
            .iter()
            .map(|e| Arc::clone(&e.value().channel))
            .collect();
        for ch in channels {
            ch.connect().await?;
        }
        Ok(())
    }

    /// Disconnect all registered channels.
    pub async fn disconnect_all(&self) -> Result<()> {
        let channels: Vec<Arc<dyn Channel>> = self
            .entries
            .iter()
            .map(|e| Arc::clone(&e.value().channel))
            .collect();
        for ch in channels {
            ch.disconnect().await?;
        }
        Ok(())
    }

    /// Health check all channels.
    pub async fn health_all(&self) -> std::collections::HashMap<String, bool> {
        let channels: Vec<(String, Arc<dyn Channel>)> = self
            .entries
            .iter()
            .map(|e| (e.key().clone(), Arc::clone(&e.value().channel)))
            .collect();
        let mut results = std::collections::HashMap::new();
        for (name, ch) in channels {
            let healthy = ch.health_check().await;
            results.insert(name, healthy);
        }
        results
    }

    /// Send a message through a named channel.
    /// Routes through the full Channel object (not the lightweight Sender)
    /// so that connected channels can use their active bot handles.
    pub async fn send(&self, name: &str, message: ChannelMessage) -> Result<()> {
        let channel = self
            .get_channel(name)
            .ok_or_else(|| MesoError::Channel(format!("channel not found: {name}")))?;
        channel.send_message(message).await
    }

    /// Number of registered channels.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::mpsc;

    struct MockChannel {
        name: String,
        sent: Arc<AtomicBool>,
    }

    impl MockChannel {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                sent: Arc::new(AtomicBool::new(false)),
            }
        }
    }

    struct MockSender {
        channel_name: String,
        sent: Arc<AtomicBool>,
    }

    #[async_trait]
    impl ChannelSender for MockSender {
        fn channel_type(&self) -> &str {
            &self.channel_name
        }
        async fn send_message(&self, _message: ChannelMessage) -> Result<()> {
            self.sent.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[async_trait]
    impl ChannelSender for MockChannel {
        fn channel_type(&self) -> &str {
            &self.name
        }
        async fn send_message(&self, _message: ChannelMessage) -> Result<()> {
            self.sent.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[async_trait]
    impl super::super::traits::ChannelLifecycle for MockChannel {
        fn display_name(&self) -> &str {
            &self.name
        }
        async fn connect(&self) -> Result<()> {
            Ok(())
        }
        async fn disconnect(&self) -> Result<()> {
            Ok(())
        }
        fn status(&self) -> ChannelStatus {
            ChannelStatus::Connected
        }
        fn create_sender(&self) -> Box<dyn ChannelSender> {
            Box::new(MockSender {
                channel_name: self.name.clone(),
                sent: self.sent.clone(),
            })
        }
    }

    #[async_trait]
    impl Channel for MockChannel {
        async fn listen(&self, _tx: mpsc::Sender<ChannelMessage>) -> Result<()> {
            Ok(())
        }
        async fn health_check(&self) -> bool {
            true
        }
    }

    #[test]
    fn register_channel() {
        let registry = ChannelRegistry::new();
        let channel = Arc::new(MockChannel::new("test"));
        let result = registry.register(channel);
        assert!(result.is_ok());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn unregister_channel() {
        let registry = ChannelRegistry::new();
        registry
            .register(Arc::new(MockChannel::new("test")))
            .unwrap();
        assert!(registry.unregister("test"));
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn unregister_nonexistent() {
        let registry = ChannelRegistry::new();
        assert!(!registry.unregister("nonexistent"));
    }

    #[test]
    fn list_channels() {
        let registry = ChannelRegistry::new();
        registry
            .register(Arc::new(MockChannel::new("telegram")))
            .unwrap();
        registry
            .register(Arc::new(MockChannel::new("slack")))
            .unwrap();
        let names = registry.list();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"telegram".into()));
        assert!(names.contains(&"slack".into()));
    }

    #[test]
    fn register_or_replace_channel() {
        let registry = ChannelRegistry::new();
        let channel1 = Arc::new(MockChannel::new("test"));
        let channel2 = Arc::new(MockChannel::new("test"));
        registry.register(channel1).unwrap();
        assert_eq!(registry.len(), 1);
        // register_or_replace should succeed even though "test" is already registered
        let result = registry.register_or_replace(channel2);
        assert!(result.is_ok());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn register_or_replace_new_channel() {
        let registry = ChannelRegistry::new();
        let channel = Arc::new(MockChannel::new("test"));
        let result = registry.register_or_replace(channel);
        assert!(result.is_ok());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn get_sender() {
        let registry = ChannelRegistry::new();
        registry
            .register(Arc::new(MockChannel::new("test")))
            .unwrap();
        let sender = registry.get_sender("test");
        assert!(sender.is_some());
        assert_eq!(sender.unwrap().channel_type(), "test");
    }

    #[test]
    fn get_sender_unknown() {
        let registry = ChannelRegistry::new();
        assert!(registry.get_sender("unknown").is_none());
    }

    // WS2.8 — Atomic register and unregister removes both channel and sender
    #[test]
    fn channel_registry_atomic_register_and_unregister() {
        let registry = ChannelRegistry::new();
        registry
            .register(Arc::new(MockChannel::new("test")))
            .unwrap();
        assert!(registry.get_channel("test").is_some());
        assert!(registry.get_sender("test").is_some());
        registry.unregister("test");
        assert!(registry.get_channel("test").is_none());
        assert!(registry.get_sender("test").is_none());
    }
}
