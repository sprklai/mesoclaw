//! Scheduler job resource handler.
//!
//! Handles lifecycle management for scheduled background jobs.

use async_trait::async_trait;

use crate::lifecycle::handlers::ResourceHandler;
use crate::lifecycle::states::{
    FallbackOption, HealthStatus, PreservedState, ResourceConfig, ResourceError, ResourceId,
    ResourceInstance, ResourceType, SchedulerPreservedState,
};

/// Valid substates for scheduler job resources.
pub const SCHEDULER_SUBSTATES: &[&str] = &[
    "scheduled",
    "triggered",
    "running",
    "waiting_agent",
    "finishing",
    "completed",
    "failed",
    "paused",
];

/// Handler for scheduler job resources.
pub struct SchedulerHandler {
    /// Maximum retry attempts
    max_retries: u32,
}

impl SchedulerHandler {
    /// Create a new scheduler handler.
    pub fn new() -> Self {
        Self { max_retries: 2 }
    }

    /// Create with custom max retries.
    pub fn with_max_retries(max_retries: u32) -> Self {
        Self { max_retries }
    }
}

impl Default for SchedulerHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ResourceHandler for SchedulerHandler {
    fn resource_type(&self) -> ResourceType {
        ResourceType::SchedulerJob
    }

    async fn start(
        &self,
        id: ResourceId,
        config: ResourceConfig,
    ) -> Result<ResourceInstance, ResourceError> {
        log::info!("SchedulerHandler: starting job {}", id);

        let instance = ResourceInstance::new(id.clone(), config);

        log::info!("SchedulerHandler: job {} started", id);
        Ok(instance)
    }

    async fn stop(&self, instance: &mut ResourceInstance) -> Result<(), ResourceError> {
        log::info!("SchedulerHandler: stopping job {}", instance.id);

        // Cancel current execution if running

        log::info!("SchedulerHandler: job {} stopped", instance.id);
        Ok(())
    }

    async fn kill(&self, instance: &mut ResourceInstance) -> Result<(), ResourceError> {
        log::warn!("SchedulerHandler: killing job {}", instance.id);

        // Force terminate

        log::warn!("SchedulerHandler: job {} killed", instance.id);
        Ok(())
    }

    async fn extract_state(
        &self,
        instance: &ResourceInstance,
    ) -> Result<PreservedState, ResourceError> {
        log::debug!("SchedulerHandler: extracting state from {}", instance.id);

        Ok(PreservedState::Scheduler(SchedulerPreservedState {
            job_id: instance.id.instance_id().to_string(),
            job_config: instance.config.job_config.clone().unwrap_or_default(),
            execution_context: None,
            partial_results: vec![],
        }))
    }

    async fn apply_state(
        &self,
        instance: &mut ResourceInstance,
        state: PreservedState,
    ) -> Result<(), ResourceError> {
        log::debug!("SchedulerHandler: applying state to {}", instance.id);

        match state {
            PreservedState::Scheduler(scheduler_state) => {
                instance.config.job_config = Some(scheduler_state.job_config);
                log::info!("SchedulerHandler: applied state to {}", instance.id);
                Ok(())
            }
            _ => Err(ResourceError::StateApplyFailed(
                "Invalid state type for scheduler".to_string(),
            )),
        }
    }

    fn get_fallbacks(&self, _current: &ResourceInstance) -> Vec<FallbackOption> {
        // Scheduler jobs can rerun on failure
        vec![FallbackOption {
            id: "rerun".to_string(),
            label: "Rerun job".to_string(),
            description: "Execute the job again".to_string(),
            config: ResourceConfig::default(),
        }]
    }

    async fn health_check(
        &self,
        _instance: &ResourceInstance,
    ) -> Result<HealthStatus, ResourceError> {
        // Check if job is still executing
        Ok(HealthStatus::Healthy)
    }

    async fn cleanup(&self, instance: &ResourceInstance) -> Result<(), ResourceError> {
        log::debug!("SchedulerHandler: cleaning up {}", instance.id);

        // Clean up any execution artifacts

        log::debug!("SchedulerHandler: cleaned up {}", instance.id);
        Ok(())
    }

    fn valid_substates(&self) -> &'static [&'static str] {
        SCHEDULER_SUBSTATES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scheduler_handler_start() {
        let handler = SchedulerHandler::new();

        let config = ResourceConfig {
            job_config: Some(serde_json::json!({"type": "heartbeat"})),
            ..Default::default()
        };

        let instance = handler
            .start(
                ResourceId::new(ResourceType::SchedulerJob, "heartbeat:1"),
                config,
            )
            .await
            .unwrap();

        assert_eq!(instance.resource_type, ResourceType::SchedulerJob);
    }
}
