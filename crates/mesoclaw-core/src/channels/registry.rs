use std::sync::Arc;

use dashmap::DashMap;

use super::message::ChannelMessage;
use super::traits::{Channel, ChannelSender, ChannelStatus};
use crate::Result;
use crate::error::MesoError;

/// Concurrent channel registry (DashMap-backed, follows ToolRegistry pattern).
pub struct ChannelRegistry {
    channels: DashMap<String, Arc<dyn Channel>>,
    senders: DashMap<String, Arc<dyn ChannelSender>>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
            senders: DashMap::new(),
        }
    }

    /// Register a channel. Creates a sender handle automatically.
    pub fn register(&self, channel: Arc<dyn Channel>) -> Result<()> {
        let name = channel.display_name().to_string();
        if self.channels.contains_key(&name) {
            return Err(MesoError::Channel(format!(
                "channel already registered: {name}"
            )));
        }
        let sender = channel.create_sender();
        let sender: Arc<dyn ChannelSender> = Arc::from(sender);
        self.senders.insert(name.clone(), sender);
        self.channels.insert(name, channel);
        Ok(())
    }

    /// Unregister a channel by name.
    pub fn unregister(&self, name: &str) -> bool {
        self.senders.remove(name);
        self.channels.remove(name).is_some()
    }

    /// Get a lightweight send-only handle for a channel.
    pub fn get_sender(&self, name: &str) -> Option<Arc<dyn ChannelSender>> {
        self.senders.get(name).map(|r| Arc::clone(r.value()))
    }

    /// List all registered channel names.
    pub fn list(&self) -> Vec<String> {
        self.channels.iter().map(|e| e.key().clone()).collect()
    }

    /// Get the status of a channel.
    pub fn status(&self, name: &str) -> Option<ChannelStatus> {
        self.channels.get(name).map(|c| c.status())
    }

    /// Connect all registered channels.
    pub async fn connect_all(&self) -> Result<()> {
        let channels: Vec<Arc<dyn Channel>> = self
            .channels
            .iter()
            .map(|e| Arc::clone(e.value()))
            .collect();
        for ch in channels {
            ch.connect().await?;
        }
        Ok(())
    }

    /// Disconnect all registered channels.
    pub async fn disconnect_all(&self) -> Result<()> {
        let channels: Vec<Arc<dyn Channel>> = self
            .channels
            .iter()
            .map(|e| Arc::clone(e.value()))
            .collect();
        for ch in channels {
            ch.disconnect().await?;
        }
        Ok(())
    }

    /// Health check all channels.
    pub async fn health_all(&self) -> std::collections::HashMap<String, bool> {
        let channels: Vec<(String, Arc<dyn Channel>)> = self
            .channels
            .iter()
            .map(|e| (e.key().clone(), Arc::clone(e.value())))
            .collect();
        let mut results = std::collections::HashMap::new();
        for (name, ch) in channels {
            let healthy = ch.health_check().await;
            results.insert(name, healthy);
        }
        results
    }

    /// Send a message through a named channel.
    pub async fn send(&self, name: &str, message: ChannelMessage) -> Result<()> {
        let sender = self
            .get_sender(name)
            .ok_or_else(|| MesoError::Channel(format!("channel not found: {name}")))?;
        sender.send_message(message).await
    }

    /// Number of registered channels.
    pub fn len(&self) -> usize {
        self.channels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
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
}
