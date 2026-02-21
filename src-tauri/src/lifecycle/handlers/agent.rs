//! Agent resource handler.
//!
//! Handles lifecycle management for LLM-based autonomous agent sessions.

use async_trait::async_trait;

use crate::lifecycle::handlers::ResourceHandler;
use crate::lifecycle::states::{
    AgentPreservedState, FallbackOption, HealthStatus, PreservedState, ResourceConfig,
    ResourceError, ResourceId, ResourceInstance, ResourceType, SessionMetadata,
};

/// Valid substates for agent resources.
pub const AGENT_SUBSTATES: &[&str] = &[
    "initialized",
    "thinking",         // Waiting for LLM response
    "executing_tool",   // Running a tool
    "waiting_approval", // Waiting for user approval
    "waiting_input",    // Waiting for user input
    "compacting",       // Compacting history
    "streaming",        // Streaming response
    "recovered",        // Just recovered
];

/// Handler for agent resources.
pub struct AgentHandler {
    /// Fallback providers (in order of preference)
    fallback_providers: Vec<String>,
}

impl AgentHandler {
    /// Create a new agent handler.
    pub fn new() -> Self {
        Self {
            fallback_providers: vec![
                "openai".to_string(),
                "anthropic".to_string(),
                "google".to_string(),
                "groq".to_string(),
                "ollama".to_string(),
            ],
        }
    }

    /// Create with custom fallback providers.
    pub fn with_fallbacks(fallback_providers: Vec<String>) -> Self {
        Self { fallback_providers }
    }
}

impl Default for AgentHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ResourceHandler for AgentHandler {
    fn resource_type(&self) -> ResourceType {
        ResourceType::Agent
    }

    async fn start(
        &self,
        id: ResourceId,
        config: ResourceConfig,
    ) -> Result<ResourceInstance, ResourceError> {
        log::info!("AgentHandler: starting agent {}", id);

        // Create the instance - actual agent loop integration happens elsewhere
        let instance = ResourceInstance::new(id.clone(), config);

        log::info!("AgentHandler: agent {} started", id);
        Ok(instance)
    }

    async fn stop(&self, instance: &mut ResourceInstance) -> Result<(), ResourceError> {
        log::info!("AgentHandler: stopping agent {}", instance.id);

        // Actual implementation would:
        // 1. Signal the agent loop to stop
        // 2. Wait for graceful shutdown
        // 3. Release any held resources

        log::info!("AgentHandler: agent {} stopped", instance.id);
        Ok(())
    }

    async fn kill(&self, instance: &mut ResourceInstance) -> Result<(), ResourceError> {
        log::warn!("AgentHandler: killing agent {}", instance.id);

        // Actual implementation would:
        // 1. Force terminate the agent loop
        // 2. Clean up any partial state

        log::warn!("AgentHandler: agent {} killed", instance.id);
        Ok(())
    }

    async fn extract_state(
        &self,
        instance: &ResourceInstance,
    ) -> Result<PreservedState, ResourceError> {
        log::debug!("AgentHandler: extracting state from {}", instance.id);

        // Actual implementation would:
        // 1. Get message history from session router
        // 2. Collect tool results
        // 3. Capture provider/model config

        // Return a minimal preserved state for now
        Ok(PreservedState::Agent(AgentPreservedState {
            message_history: vec![],
            completed_tool_results: Default::default(),
            session_metadata: SessionMetadata {
                provider_id: instance.config.provider_id.clone().unwrap_or_default(),
                model_id: instance.config.model_id.clone().unwrap_or_default(),
                system_prompt: instance.config.system_prompt.clone(),
                temperature: None,
                max_tokens: None,
            },
            memory_context: vec![],
            current_step: None,
        }))
    }

    async fn apply_state(
        &self,
        instance: &mut ResourceInstance,
        state: PreservedState,
    ) -> Result<(), ResourceError> {
        log::debug!("AgentHandler: applying state to {}", instance.id);

        match state {
            PreservedState::Agent(agent_state) => {
                // Actual implementation would:
                // 1. Restore message history
                // 2. Restore tool results
                // 3. Configure provider/model

                log::info!(
                    "AgentHandler: applied {} messages to {}",
                    agent_state.message_history.len(),
                    instance.id
                );
                Ok(())
            }
            _ => Err(ResourceError::StateApplyFailed(
                "Invalid state type for agent".to_string(),
            )),
        }
    }

    fn get_fallbacks(&self, current: &ResourceInstance) -> Vec<FallbackOption> {
        let current_provider = current.config.provider_id.as_deref().unwrap_or("");

        self.fallback_providers
            .iter()
            .filter(|p| *p != current_provider)
            .take(3)
            .enumerate()
            .map(|(i, provider)| FallbackOption {
                id: format!("fallback_{}", provider),
                label: format!("Switch to {}", provider),
                description: format!("Use {} as the LLM provider", provider),
                config: ResourceConfig {
                    provider_id: Some(provider.clone()),
                    model_id: None, // Will use default for provider
                    ..current.config.clone()
                },
            })
            .collect()
    }

    async fn health_check(
        &self,
        instance: &ResourceInstance,
    ) -> Result<HealthStatus, ResourceError> {
        // Actual implementation would:
        // 1. Check if agent loop is still responsive
        // 2. Verify provider connection
        // 3. Check memory usage

        Ok(HealthStatus::Healthy)
    }

    async fn cleanup(&self, instance: &ResourceInstance) -> Result<(), ResourceError> {
        log::debug!("AgentHandler: cleaning up {}", instance.id);

        // Actual implementation would:
        // 1. Release session from session router
        // 2. Clear any cached state
        // 3. Notify listeners

        log::debug!("AgentHandler: cleaned up {}", instance.id);
        Ok(())
    }

    fn valid_substates(&self) -> &'static [&'static str] {
        AGENT_SUBSTATES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_handler_start() {
        let handler = AgentHandler::new();

        let config = ResourceConfig {
            provider_id: Some("openai".into()),
            model_id: Some("gpt-4".into()),
            ..Default::default()
        };

        let instance = handler
            .start(ResourceId::new(ResourceType::Agent, "test:1"), config)
            .await
            .unwrap();

        assert_eq!(instance.resource_type, ResourceType::Agent);
    }

    #[test]
    fn test_agent_fallbacks() {
        let handler = AgentHandler::new();

        let config = ResourceConfig {
            provider_id: Some("openai".into()),
            ..Default::default()
        };

        let instance =
            ResourceInstance::new(ResourceId::new(ResourceType::Agent, "test:1"), config);

        let fallbacks = handler.get_fallbacks(&instance);

        // Should not include current provider (openai)
        assert!(!fallbacks.iter().any(|f| f.id == "fallback_openai"));
        assert!(!fallbacks.is_empty());
    }
}
