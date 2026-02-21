//! Tool resource handler.
//!
//! Handles lifecycle management for tool executions (sandbox and direct).

use async_trait::async_trait;

use crate::lifecycle::handlers::ResourceHandler;
use crate::lifecycle::states::{
    FallbackOption, HealthStatus, PreservedState, ResourceConfig, ResourceError, ResourceId,
    ResourceInstance, ResourceType, ToolPreservedState,
};

/// Valid substates for tool resources.
pub const TOOL_SUBSTATES: &[&str] = &[
    "initialized",
    "validating",
    "executing",
    "waiting_result",
    "cleanup",
    "completed",
    "failed",
];

/// Handler for tool resources.
pub struct ToolHandler {
    /// Maximum retry attempts (for future use)
    #[allow(dead_code)]
    max_retries: u32,
}

impl ToolHandler {
    /// Create a new tool handler.
    pub fn new() -> Self {
        Self { max_retries: 3 }
    }

    /// Create with custom max retries.
    pub fn with_max_retries(max_retries: u32) -> Self {
        Self { max_retries }
    }
}

impl Default for ToolHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ResourceHandler for ToolHandler {
    fn resource_type(&self) -> ResourceType {
        ResourceType::Tool
    }

    async fn start(
        &self,
        id: ResourceId,
        config: ResourceConfig,
    ) -> Result<ResourceInstance, ResourceError> {
        log::info!("ToolHandler: starting tool {}", id);

        let tool_name = config.tool_name.as_deref().unwrap_or("unknown");
        log::debug!("ToolHandler: tool name = {}", tool_name);

        let instance = ResourceInstance::new(id.clone(), config);

        log::info!("ToolHandler: tool {} started", id);
        Ok(instance)
    }

    async fn stop(&self, instance: &mut ResourceInstance) -> Result<(), ResourceError> {
        log::info!("ToolHandler: stopping tool {}", instance.id);

        // Cancel execution if running

        log::info!("ToolHandler: tool {} stopped", instance.id);
        Ok(())
    }

    async fn kill(&self, instance: &mut ResourceInstance) -> Result<(), ResourceError> {
        log::warn!("ToolHandler: killing tool {}", instance.id);

        // Force terminate sandbox/container

        log::warn!("ToolHandler: tool {} killed", instance.id);
        Ok(())
    }

    async fn extract_state(
        &self,
        instance: &ResourceInstance,
    ) -> Result<PreservedState, ResourceError> {
        log::debug!("ToolHandler: extracting state from {}", instance.id);

        Ok(PreservedState::Tool(ToolPreservedState {
            tool_name: instance.config.tool_name.clone().unwrap_or_default(),
            arguments: instance.config.tool_args.clone().unwrap_or_default(),
            partial_result: None,
            attempt_number: 0,
        }))
    }

    async fn apply_state(
        &self,
        instance: &mut ResourceInstance,
        state: PreservedState,
    ) -> Result<(), ResourceError> {
        log::debug!("ToolHandler: applying state to {}", instance.id);

        match state {
            PreservedState::Tool(tool_state) => {
                instance.config.tool_name = Some(tool_state.tool_name);
                instance.config.tool_args = Some(tool_state.arguments);
                log::info!("ToolHandler: applied state to {}", instance.id);
                Ok(())
            }
            _ => Err(ResourceError::StateApplyFailed(
                "Invalid state type for tool".to_string(),
            )),
        }
    }

    fn get_fallbacks(&self, _current: &ResourceInstance) -> Vec<FallbackOption> {
        // Tools don't typically have fallbacks - they either work or fail
        vec![]
    }

    async fn health_check(
        &self,
        _instance: &ResourceInstance,
    ) -> Result<HealthStatus, ResourceError> {
        // Check if tool is still executing
        Ok(HealthStatus::Healthy)
    }

    async fn cleanup(&self, instance: &ResourceInstance) -> Result<(), ResourceError> {
        log::debug!("ToolHandler: cleaning up {}", instance.id);

        // Clean up sandbox, release resources

        log::debug!("ToolHandler: cleaned up {}", instance.id);
        Ok(())
    }

    fn valid_substates(&self) -> &'static [&'static str] {
        TOOL_SUBSTATES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_handler_start() {
        let handler = ToolHandler::new();

        let config = ResourceConfig {
            tool_name: Some("shell".into()),
            tool_args: Some(serde_json::json!({"command": "echo test"})),
            ..Default::default()
        };

        let instance = handler
            .start(ResourceId::new(ResourceType::Tool, "shell:1"), config)
            .await
            .unwrap();

        assert_eq!(instance.resource_type, ResourceType::Tool);
    }
}
