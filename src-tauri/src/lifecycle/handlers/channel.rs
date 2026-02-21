//! Channel resource handler.
//!
//! Handles lifecycle management for external messaging channels
//! (Telegram, Discord, Slack, Matrix).

use async_trait::async_trait;

use crate::lifecycle::handlers::ResourceHandler;
use crate::lifecycle::states::{
    ChannelPreservedState, FallbackOption, HealthStatus, PreservedState, ResourceConfig,
    ResourceError, ResourceId, ResourceInstance, ResourceType,
};

/// Valid substates for channel resources.
pub const CHANNEL_SUBSTATES: &[&str] = &[
    "initialized",
    "connecting",
    "connected",
    "reconnecting",
    "sending",
    "waiting_ack",
    "polling",
    "disconnected",
    "error",
];

/// Handler for channel resources.
pub struct ChannelHandler {
    /// Fallback channel types
    fallback_channels: Vec<String>,
}

impl ChannelHandler {
    /// Create a new channel handler.
    pub fn new() -> Self {
        Self {
            fallback_channels: vec![
                "telegram".to_string(),
                "discord".to_string(),
                "slack".to_string(),
                "matrix".to_string(),
            ],
        }
    }
}

impl Default for ChannelHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ResourceHandler for ChannelHandler {
    fn resource_type(&self) -> ResourceType {
        ResourceType::Channel
    }

    async fn start(
        &self,
        id: ResourceId,
        config: ResourceConfig,
    ) -> Result<ResourceInstance, ResourceError> {
        log::info!("ChannelHandler: starting channel {}", id);

        let instance = ResourceInstance::new(id.clone(), config);

        log::info!("ChannelHandler: channel {} started", id);
        Ok(instance)
    }

    async fn stop(&self, instance: &mut ResourceInstance) -> Result<(), ResourceError> {
        log::info!("ChannelHandler: stopping channel {}", instance.id);

        // Actual implementation would disconnect from the channel

        log::info!("ChannelHandler: channel {} stopped", instance.id);
        Ok(())
    }

    async fn kill(&self, instance: &mut ResourceInstance) -> Result<(), ResourceError> {
        log::warn!("ChannelHandler: killing channel {}", instance.id);

        // Force disconnect

        log::warn!("ChannelHandler: channel {} killed", instance.id);
        Ok(())
    }

    async fn extract_state(
        &self,
        instance: &ResourceInstance,
    ) -> Result<PreservedState, ResourceError> {
        log::debug!("ChannelHandler: extracting state from {}", instance.id);

        Ok(PreservedState::Channel(ChannelPreservedState {
            outbound_queue: Default::default(),
            config: instance.config.channel_config.clone(),
            last_sequence: 0,
            pending_acks: Default::default(),
        }))
    }

    async fn apply_state(
        &self,
        instance: &mut ResourceInstance,
        state: PreservedState,
    ) -> Result<(), ResourceError> {
        log::debug!("ChannelHandler: applying state to {}", instance.id);

        match state {
            PreservedState::Channel(channel_state) => {
                // Restore queue and config
                instance.config.channel_config = channel_state.config;
                log::info!("ChannelHandler: applied state to {}", instance.id);
                Ok(())
            }
            _ => Err(ResourceError::StateApplyFailed(
                "Invalid state type for channel".to_string(),
            )),
        }
    }

    fn get_fallbacks(&self, current: &ResourceInstance) -> Vec<FallbackOption> {
        let current_type = current.config.channel_type.as_deref().unwrap_or("");

        self.fallback_channels
            .iter()
            .filter(|c| *c != current_type)
            .take(2)
            .map(|channel| FallbackOption {
                id: format!("fallback_{}", channel),
                label: format!("Use {} instead", channel),
                description: format!("Switch to {} channel", channel),
                config: ResourceConfig {
                    channel_type: Some(channel.clone()),
                    ..Default::default()
                },
            })
            .collect()
    }

    async fn health_check(
        &self,
        _instance: &ResourceInstance,
    ) -> Result<HealthStatus, ResourceError> {
        // Check connection status
        Ok(HealthStatus::Healthy)
    }

    async fn cleanup(&self, instance: &ResourceInstance) -> Result<(), ResourceError> {
        log::debug!("ChannelHandler: cleaning up {}", instance.id);

        // Clear queues, release connection

        log::debug!("ChannelHandler: cleaned up {}", instance.id);
        Ok(())
    }

    fn valid_substates(&self) -> &'static [&'static str] {
        CHANNEL_SUBSTATES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_handler_start() {
        let handler = ChannelHandler::new();

        let config = ResourceConfig {
            channel_type: Some("telegram".into()),
            ..Default::default()
        };

        let instance = handler
            .start(
                ResourceId::new(ResourceType::Channel, "telegram:bot123"),
                config,
            )
            .await
            .unwrap();

        assert_eq!(instance.resource_type, ResourceType::Channel);
    }
}
