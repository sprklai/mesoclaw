//! Recovery engine for orchestrating resource recovery.
//!
//! The recovery engine handles the transfer & preserve strategy for
//! recovering stuck resources.

use chrono::Utc;
use std::sync::Arc;

use super::handlers::ResourceHandler;
use super::plugin_registry::PluginRegistry;
use super::state_registry::StateRegistry;
use super::states::{
    InterventionResolution, PreservedState, RecoveryActionType, ResourceError, ResourceId,
    ResourceInstance, ResourceType,
};

/// Result of a recovery attempt.
#[derive(Debug, Clone)]
pub enum RecoveryResult {
    /// Resource was recovered in place
    Recovered { resource_id: ResourceId },
    /// Resource was transferred to a new instance
    Transferred {
        from_id: ResourceId,
        to_id: ResourceId,
    },
    /// Recovery escalated to higher tier
    Escalated { tier: u8 },
    /// Recovery failed (may need user intervention)
    Failed { reason: String },
}

/// Recovery actions that can be taken for stuck resources.
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Retry the same resource
    Retry { preserve_state: bool },
    /// Transfer to a new resource instance
    Transfer {
        to_type: Option<ResourceType>,
        preserve_state: bool,
    },
    /// Escalate to a higher tier
    Escalate { tier: u8 },
    /// Abort the resource
    Abort { reason: String },
}

/// Orchestrates recovery of stuck resources.
pub struct RecoveryEngine {
    /// State registry for resource tracking
    state_registry: Arc<StateRegistry>,
    /// Plugin registry for handler lookup (for future use)
    #[allow(dead_code)]
    plugin_registry: Arc<PluginRegistry>,
}

impl RecoveryEngine {
    /// Create a new recovery engine.
    pub fn new(state_registry: Arc<StateRegistry>, plugin_registry: Arc<PluginRegistry>) -> Self {
        Self {
            state_registry,
            plugin_registry,
        }
    }

    /// Attempt recovery for a stuck resource.
    ///
    /// This implements the transfer & preserve strategy:
    /// 1. Extract preservable state from stuck resource
    /// 2. Create new instance (or reuse existing)
    /// 3. Apply preserved state
    /// 4. Clean up old instance
    pub async fn recover(
        &self,
        resource_id: &ResourceId,
        action: RecoveryAction,
    ) -> Result<RecoveryResult, ResourceError> {
        // Get the current instance
        let instance = self
            .state_registry
            .get(resource_id)
            .await
            .ok_or_else(|| ResourceError::NotFound(resource_id.to_string()))?;

        // Get the appropriate handler
        let handler = self.get_handler(&instance.resource_type)?;

        match action {
            RecoveryAction::Retry { preserve_state } => {
                self.retry_resource(&instance, preserve_state, handler.as_ref())
                    .await
            }
            RecoveryAction::Transfer {
                to_type,
                preserve_state,
            } => {
                self.transfer_resource(&instance, to_type, preserve_state)
                    .await
            }
            RecoveryAction::Escalate { tier } => self.escalate_resource(&instance, tier).await,
            RecoveryAction::Abort { reason } => self.abort_resource(&instance, reason).await,
        }
    }

    /// Retry the same resource.
    async fn retry_resource(
        &self,
        instance: &ResourceInstance,
        preserve_state: bool,
        handler: &dyn ResourceHandler,
    ) -> Result<RecoveryResult, ResourceError> {
        log::info!(
            "RecoveryEngine: retrying {} (preserve_state={})",
            instance.id,
            preserve_state
        );

        // Update state to recovering
        self.state_registry
            .update_state(
                &instance.id,
                super::states::ResourceState::Recovering {
                    action: RecoveryActionType::Retry,
                    started_at: Utc::now(),
                },
                "Starting retry recovery".to_string(),
            )
            .await;

        // Extract state if requested
        let preserved = if preserve_state {
            Some(handler.extract_state(instance).await?)
        } else {
            None
        };

        // Stop the stuck resource
        let mut instance_mut = instance.clone();
        handler.stop(&mut instance_mut).await?;

        // Start a new instance with the same config
        let new_instance = handler
            .start(instance.id.clone(), instance.config.clone())
            .await?;

        // Apply preserved state if available
        if let Some(state) = preserved {
            let mut new_instance_mut = new_instance.clone();
            handler.apply_state(&mut new_instance_mut, state).await?;
        }

        // Update state to running
        self.state_registry
            .update_state(
                &instance.id,
                super::states::ResourceState::Running {
                    substate: "recovered".to_string(),
                    started_at: Utc::now(),
                    progress: None,
                },
                "Recovery retry completed".to_string(),
            )
            .await;

        // Increment recovery attempts
        self.state_registry
            .increment_recovery_attempts(&instance.id)
            .await;

        Ok(RecoveryResult::Recovered {
            resource_id: instance.id.clone(),
        })
    }

    /// Transfer to a new resource instance.
    async fn transfer_resource(
        &self,
        instance: &ResourceInstance,
        to_type: Option<ResourceType>,
        preserve_state: bool,
    ) -> Result<RecoveryResult, ResourceError> {
        let target_type = to_type.unwrap_or_else(|| instance.resource_type.clone());

        log::info!(
            "RecoveryEngine: transferring {} to {} (preserve_state={})",
            instance.id,
            target_type,
            preserve_state
        );

        // Update state to recovering
        self.state_registry
            .update_state(
                &instance.id,
                super::states::ResourceState::Recovering {
                    action: RecoveryActionType::Transfer,
                    started_at: Utc::now(),
                },
                format!("Starting transfer to {}", target_type),
            )
            .await;

        // Get the source handler
        let source_handler = self.get_handler(&instance.resource_type)?;

        // Extract state if requested
        let preserved = if preserve_state {
            Some(source_handler.extract_state(instance).await?)
        } else {
            None
        };

        // Create new resource ID for target
        let new_id = ResourceId::new(
            target_type.clone(),
            format!(
                "{}:transferred:{}",
                instance.id.instance_id(),
                Utc::now().timestamp()
            ),
        );

        // Get target handler
        let target_handler = self.get_handler(&target_type)?;

        // Start new instance
        let mut new_instance = target_handler
            .start(new_id.clone(), instance.config.clone())
            .await?;

        // Register the new instance
        self.state_registry.register(new_instance.clone()).await;

        // Apply preserved state if available
        if let Some(state) = preserved {
            target_handler.apply_state(&mut new_instance, state).await?;
        }

        // Update new instance state
        self.state_registry
            .update_state(
                &new_id,
                super::states::ResourceState::Running {
                    substate: "transferred".to_string(),
                    started_at: Utc::now(),
                    progress: None,
                },
                format!("Transferred from {}", instance.id),
            )
            .await;

        // Clean up the old instance
        source_handler.cleanup(instance).await?;

        // Update old instance state
        self.state_registry
            .update_state(
                &instance.id,
                super::states::ResourceState::Completed {
                    at: Utc::now(),
                    result: Some(serde_json::json!({ "transferred_to": new_id.to_string() })),
                },
                format!("Transferred to {}", new_id),
            )
            .await;

        Ok(RecoveryResult::Transferred {
            from_id: instance.id.clone(),
            to_id: new_id,
        })
    }

    /// Escalate the resource to a higher tier.
    async fn escalate_resource(
        &self,
        instance: &ResourceInstance,
        tier: u8,
    ) -> Result<RecoveryResult, ResourceError> {
        log::info!(
            "RecoveryEngine: escalating {} to tier {}",
            instance.id,
            tier
        );

        // Update escalation tier
        self.state_registry
            .set_escalation_tier(&instance.id, tier)
            .await;

        // Update state
        self.state_registry
            .update_state(
                &instance.id,
                super::states::ResourceState::Recovering {
                    action: RecoveryActionType::Fallback,
                    started_at: Utc::now(),
                },
                format!("Escalated to tier {}", tier),
            )
            .await;

        Ok(RecoveryResult::Escalated { tier })
    }

    /// Abort the resource.
    async fn abort_resource(
        &self,
        instance: &ResourceInstance,
        reason: String,
    ) -> Result<RecoveryResult, ResourceError> {
        log::warn!("RecoveryEngine: aborting {} - {}", instance.id, reason);

        // Get handler
        let handler = self.get_handler(&instance.resource_type)?;

        // Kill the resource
        let mut instance_mut = instance.clone();
        handler.kill(&mut instance_mut).await?;

        // Cleanup
        handler.cleanup(&instance).await?;

        // Update state to failed
        self.state_registry
            .update_state(
                &instance.id,
                super::states::ResourceState::Failed {
                    at: Utc::now(),
                    error: reason.clone(),
                    terminal: true,
                    escalation_tier_reached: instance.current_escalation_tier,
                },
                "Aborted".to_string(),
            )
            .await;

        Ok(RecoveryResult::Failed { reason })
    }

    /// Extract preservable state from a resource.
    pub async fn extract_state(
        &self,
        resource_id: &ResourceId,
    ) -> Result<PreservedState, ResourceError> {
        let instance = self
            .state_registry
            .get(resource_id)
            .await
            .ok_or_else(|| ResourceError::NotFound(resource_id.to_string()))?;

        let handler = self.get_handler(&instance.resource_type)?;
        handler.extract_state(&instance).await
    }

    /// Apply preserved state to a resource.
    pub async fn apply_state(
        &self,
        target_id: &ResourceId,
        state: PreservedState,
    ) -> Result<(), ResourceError> {
        let instance = self
            .state_registry
            .get(target_id)
            .await
            .ok_or_else(|| ResourceError::NotFound(target_id.to_string()))?;

        let handler = self.get_handler(&instance.resource_type)?;
        let mut instance_mut = instance.clone();
        handler.apply_state(&mut instance_mut, state).await
    }

    /// Get fallback options for a resource.
    pub async fn get_fallbacks(
        &self,
        resource_id: &ResourceId,
    ) -> Result<Vec<super::states::FallbackOption>, ResourceError> {
        let instance = self
            .state_registry
            .get(resource_id)
            .await
            .ok_or_else(|| ResourceError::NotFound(resource_id.to_string()))?;

        let handler = self.get_handler(&instance.resource_type)?;
        Ok(handler.get_fallbacks(&instance))
    }

    /// Handle user intervention resolution.
    pub async fn handle_intervention(
        &self,
        _resolution: InterventionResolution,
    ) -> Result<RecoveryResult, ResourceError> {
        // This would be implemented based on the selected option
        // For now, return a placeholder
        Err(ResourceError::Internal(
            "User intervention handling not implemented".to_string(),
        ))
    }

    /// Get the handler for a resource type.
    fn get_handler(
        &self,
        resource_type: &ResourceType,
    ) -> Result<Box<dyn ResourceHandler>, ResourceError> {
        // For now, return an error since we can't clone handlers
        // In practice, handlers would be Arc<dyn ResourceHandler>
        Err(ResourceError::HandlerNotRegistered(
            resource_type.to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_action_debug() {
        let action = RecoveryAction::Retry {
            preserve_state: true,
        };
        assert!(format!("{:?}", action).contains("Retry"));
    }

    #[test]
    fn test_recovery_result_debug() {
        let result = RecoveryResult::Failed {
            reason: "test".to_string(),
        };
        assert!(format!("{:?}", result).contains("Failed"));
    }
}
