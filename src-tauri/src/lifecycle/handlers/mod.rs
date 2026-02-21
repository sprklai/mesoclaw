//! Resource handler traits and implementations.
//!
//! This module defines the `ResourceHandler` trait that all resource types
//! must implement to be managed by the lifecycle supervisor.

pub mod agent;
pub mod channel;
pub mod scheduler;
pub mod tool;

use async_trait::async_trait;

use crate::lifecycle::states::{
    FallbackOption, HealthStatus, PreservedState, ResourceConfig, ResourceError, ResourceId,
    ResourceInstance, ResourceType,
};

/// Core trait for all resource handlers.
///
/// Implement this trait to add support for a new resource type to the
/// lifecycle management system. The supervisor uses handlers to perform
/// all resource-specific operations.
#[async_trait]
pub trait ResourceHandler: Send + Sync {
    /// Unique identifier for this resource type.
    fn resource_type(&self) -> ResourceType;

    /// Start a new resource instance.
    ///
    /// Called when spawning a new resource. The handler should initialize
    /// the resource and return an instance ready for tracking.
    async fn start(
        &self,
        id: ResourceId,
        config: ResourceConfig,
    ) -> Result<ResourceInstance, ResourceError>;

    /// Stop a resource gracefully.
    ///
    /// Called when a resource should be stopped normally. The handler
    /// should clean up resources and transition the instance to a
    /// terminal state.
    async fn stop(&self, instance: &mut ResourceInstance) -> Result<(), ResourceError>;

    /// Force kill a stuck resource.
    ///
    /// Called when a resource is unresponsive and must be terminated.
    /// This is more aggressive than stop and may leave resources in an
    /// inconsistent state.
    async fn kill(&self, instance: &mut ResourceInstance) -> Result<(), ResourceError>;

    /// Extract preservable state for transfer.
    ///
    /// Called before transferring a resource to capture its state
    /// so it can be applied to a new instance.
    async fn extract_state(
        &self,
        instance: &ResourceInstance,
    ) -> Result<PreservedState, ResourceError>;

    /// Apply preserved state to a new instance.
    ///
    /// Called after creating a new instance to restore state from
    /// a previous resource.
    async fn apply_state(
        &self,
        instance: &mut ResourceInstance,
        state: PreservedState,
    ) -> Result<(), ResourceError>;

    /// Get fallback options for this resource type.
    ///
    /// Called when escalating to determine what alternatives are
    /// available for this resource.
    fn get_fallbacks(&self, current: &ResourceInstance) -> Vec<FallbackOption>;

    /// Perform a health check on the resource.
    ///
    /// Called periodically by the health monitor to verify the
    /// resource is still functioning correctly beyond just heartbeats.
    async fn health_check(
        &self,
        instance: &ResourceInstance,
    ) -> Result<HealthStatus, ResourceError>;

    /// Clean up resources after completion or failure.
    ///
    /// Called after a resource has reached a terminal state to
    /// release any held resources.
    async fn cleanup(&self, instance: &ResourceInstance) -> Result<(), ResourceError>;

    /// Record a heartbeat from this resource.
    ///
    /// Called by the resource when it wants to signal it's still alive.
    /// The default implementation does nothing; handlers may override
    /// to perform resource-specific heartbeat processing.
    async fn record_heartbeat(&self, _instance: &ResourceInstance) -> Result<(), ResourceError> {
        Ok(())
    }

    /// Get the list of valid substates for this resource type.
    ///
    /// Used for validation and display purposes.
    fn valid_substates(&self) -> &'static [&'static str] {
        &[]
    }

    /// Check if a substate is valid for this resource type.
    fn is_valid_substate(&self, substate: &str) -> bool {
        self.valid_substates().contains(&substate)
    }
}

/// Type alias for a boxed resource handler.
pub type BoxedResourceHandler = Box<dyn ResourceHandler>;

#[cfg(test)]
mod tests {
    // Placeholder for handler tests - actual tests are in handler-specific modules
}
