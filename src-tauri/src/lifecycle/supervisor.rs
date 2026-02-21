//! Lifecycle supervisor - the central controller for resource management.
//!
//! The supervisor orchestrates all lifecycle components: health monitoring,
//! state tracking, recovery, and escalation.

use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use chrono::Utc;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;

use super::escalation_manager::{EscalationConfig, EscalationManager, EscalationError};
use super::event_bus::LifecycleEvent;
use super::handlers::ResourceHandler;
use super::health_monitor::HealthMonitor;
use super::plugin_registry::PluginRegistry;
use super::recovery_engine::{RecoveryAction, RecoveryEngine, RecoveryResult};
use super::state_registry::StateRegistry;
use super::states::{
    HealthStatus, HeartbeatConfig, InterventionResolution, PreservedState, RecoveryActionType,
    ResourceConfig, ResourceError, ResourceId, ResourceInstance, ResourceState, ResourceType,
    SupervisorConfig, UserInterventionRequest,
};

/// The central controller for resource lifecycle management.
pub struct LifecycleSupervisor {
    /// Configuration
    config: SupervisorConfig,
    /// Health monitor for heartbeat tracking
    health_monitor: Arc<HealthMonitor>,
    /// State registry for resource tracking
    state_registry: Arc<StateRegistry>,
    /// Recovery engine for orchestrating recovery
    recovery_engine: Arc<RecoveryEngine>,
    /// Escalation manager for tier management
    escalation_manager: Arc<EscalationManager>,
    /// Plugin registry for resource handlers
    plugin_registry: Arc<PluginRegistry>,
    /// Event bus for lifecycle events
    event_bus: super::event_bus::LifecycleEventBus,
    /// Background monitoring task handle
    monitor_task: RwLock<Option<JoinHandle<()>>>,
    /// Running flag
    running: RwLock<bool>,
    /// Intervention interface mode
    intervention_interface: RwLock<InterventionInterface>,
}

/// Interface for user interventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterventionInterface {
    /// Tauri desktop app - uses Tauri events → React dialogs
    Tauri,
    /// CLI mode - uses console prompts
    Cli,
}

impl Default for InterventionInterface {
    fn default() -> Self {
        Self::Tauri
    }
}

impl LifecycleSupervisor {
    /// Create a new lifecycle supervisor.
    pub fn new(config: SupervisorConfig) -> Self {
        let event_bus = super::event_bus::LifecycleEventBus::new();
        let plugin_registry = Arc::new(PluginRegistry::new());
        let state_registry = Arc::new(StateRegistry::new());
        let health_monitor = Arc::new(HealthMonitor::new());

        let escalation_manager = Arc::new(EscalationManager::new(
            EscalationConfig::default(),
            Arc::clone(&state_registry),
        ));

        let recovery_engine = Arc::new(RecoveryEngine::new(
            Arc::clone(&state_registry),
            Arc::clone(&plugin_registry),
        ));

        Self {
            config,
            health_monitor,
            state_registry,
            recovery_engine,
            escalation_manager,
            plugin_registry,
            event_bus,
            monitor_task: RwLock::new(None),
            running: RwLock::new(false),
            intervention_interface: RwLock::new(InterventionInterface::default()),
        }
    }

    /// Create with a shared event bus.
    pub fn with_event_bus(
        config: SupervisorConfig,
        event_bus: super::event_bus::LifecycleEventBus,
    ) -> Self {
        let plugin_registry = Arc::new(PluginRegistry::new());
        let state_registry = Arc::new(StateRegistry::new());
        let health_monitor = Arc::new(HealthMonitor::new());

        let escalation_manager = Arc::new(EscalationManager::new(
            EscalationConfig::default(),
            Arc::clone(&state_registry),
        ));

        let recovery_engine = Arc::new(RecoveryEngine::new(
            Arc::clone(&state_registry),
            Arc::clone(&plugin_registry),
        ));

        Self {
            config,
            health_monitor,
            state_registry,
            recovery_engine,
            escalation_manager,
            plugin_registry,
            event_bus,
            monitor_task: RwLock::new(None),
            running: RwLock::new(false),
            intervention_interface: RwLock::new(InterventionInterface::default()),
        }
    }

    /// Set the intervention interface mode.
    pub async fn set_intervention_interface(&self, interface: InterventionInterface) {
        let mut current = self.intervention_interface.write().await;
        *current = interface;
    }

    /// Get the current intervention interface mode.
    pub async fn get_intervention_interface(&self) -> InterventionInterface {
        *self.intervention_interface.read().await
    }

    /// Register a resource handler.
    pub fn register_handler(&self, handler: Box<dyn ResourceHandler>) {
        self.plugin_registry.register(handler);
    }

    /// Subscribe to lifecycle events.
    pub fn subscribe_events(&self) -> broadcast::Receiver<LifecycleEvent> {
        self.event_bus.subscribe()
    }

    /// Start tracking a new resource.
    ///
    /// This creates a new resource instance and begins monitoring it.
    pub async fn spawn_resource(
        &self,
        resource_type: ResourceType,
        config: ResourceConfig,
    ) -> Result<ResourceId, ResourceError> {
        // Generate a unique ID
        let instance_id = format!("{}:{}", Utc::now().timestamp_millis(), uuid::Uuid::new_v4());
        let id = ResourceId::new(resource_type.clone(), instance_id);

        // Create instance
        let instance = ResourceInstance::new(id.clone(), config);

        // Register in state registry
        if !self.state_registry.register(instance.clone()).await {
            return Err(ResourceError::AlreadyExists(id.to_string()));
        }

        // Start health monitoring
        self.health_monitor.start_tracking(&instance).await;

        // Update state to running
        self.state_registry
            .update_state(
                &id,
                ResourceState::Running {
                    substate: "initialized".to_string(),
                    started_at: Utc::now(),
                    progress: None,
                },
                "Resource spawned".to_string(),
            )
            .await;

        // Emit event
        let _ = self.event_bus.publish(LifecycleEvent::ResourceStarted {
            resource_id: id.clone(),
            resource_type: resource_type.clone(),
        });

        log::info!("LifecycleSupervisor: spawned {} ({})", id, resource_type);

        Ok(id)
    }

    /// Get the current state of a resource.
    pub async fn get_state(&self, resource_id: &ResourceId) -> Option<ResourceState> {
        self.state_registry.get(resource_id).await.map(|r| r.state)
    }

    /// Get a resource instance by ID.
    pub async fn get_resource(&self, resource_id: &ResourceId) -> Option<ResourceInstance> {
        self.state_registry.get(resource_id).await
    }

    /// Get all resources of a type.
    pub async fn get_resources_by_type(&self, resource_type: ResourceType) -> Vec<ResourceInstance> {
        self.state_registry.get_by_type(&resource_type).await
    }

    /// Get all tracked resources.
    pub async fn get_all_resources(&self) -> Vec<ResourceInstance> {
        self.state_registry.get_all().await
    }

    /// Get all stuck resources.
    pub async fn get_stuck_resources(&self) -> Vec<ResourceInstance> {
        self.state_registry.get_stuck().await
    }

    /// Record a heartbeat for a resource.
    pub async fn record_heartbeat(&self, resource_id: &ResourceId) {
        self.health_monitor.record_heartbeat(resource_id).await;

        let _ = self.event_bus.publish(LifecycleEvent::ResourceHeartbeat {
            resource_id: resource_id.clone(),
            timestamp: Utc::now(),
        });
    }

    /// Update resource progress.
    pub async fn update_progress(
        &self,
        resource_id: &ResourceId,
        progress: f32,
        substate: String,
    ) -> bool {
        let instance = self.state_registry.get(resource_id).await;
        if let Some(instance) = instance {
            self.state_registry
                .update_state(
                    resource_id,
                    ResourceState::Running {
                        substate,
                        started_at: match instance.state {
                            ResourceState::Running { started_at, .. } => started_at,
                            _ => Utc::now(),
                        },
                        progress: Some(progress),
                    },
                    "Progress update".to_string(),
                )
                .await;

            let _ = self.event_bus.publish(LifecycleEvent::ResourceProgress {
                resource_id: resource_id.clone(),
                progress,
                substate: format!("{:?}", instance.state),
            });

            true
        } else {
            false
        }
    }

    /// Manually trigger recovery for a resource.
    pub async fn recover_resource(&self, resource_id: &ResourceId) -> Result<RecoveryResult, ResourceError> {
        let instance = self
            .state_registry
            .get(resource_id)
            .await
            .ok_or_else(|| ResourceError::NotFound(resource_id.to_string()))?;

        // Get recovery action from escalation manager
        let action = self.escalation_manager.determine_action(&instance).await;

        // Record the attempt
        self.escalation_manager.record_attempt(&resource_id.to_string()).await;

        // Update state to recovering
        let action_type = match &action {
            RecoveryAction::Retry { .. } => RecoveryActionType::Retry,
            RecoveryAction::Transfer { .. } => RecoveryActionType::Transfer,
            RecoveryAction::Escalate { .. } => RecoveryActionType::Fallback,
            RecoveryAction::Abort { .. } => RecoveryActionType::UserIntervention,
        };

        self.state_registry
            .update_state(
                resource_id,
                ResourceState::Recovering {
                    action: action_type.clone(),
                    started_at: Utc::now(),
                },
                "Manual recovery triggered".to_string(),
            )
            .await;

        let _ = self.event_bus.publish(LifecycleEvent::ResourceRecovering {
            resource_id: resource_id.clone(),
            action: action_type,
        });

        // Execute recovery
        let result = self.recovery_engine.recover(resource_id, action).await;

        match &result {
            Ok(RecoveryResult::Recovered { resource_id }) => {
                let tier = self.escalation_manager.get_current_tier(&resource_id.to_string()).await;
                let _ = self.event_bus.publish(LifecycleEvent::ResourceRecovered {
                    resource_id: resource_id.clone(),
                    tier,
                });
                self.escalation_manager.reset(&resource_id.to_string()).await;
            }
            Ok(RecoveryResult::Transferred { from_id, to_id }) => {
                let _ = self.event_bus.publish(LifecycleEvent::ResourceTransferring {
                    from_id: from_id.clone(),
                    to_id: to_id.clone(),
                });
            }
            Ok(RecoveryResult::Escalated { tier }) => {
                self.escalation_manager.escalate(&resource_id.to_string()).await.ok();
            }
            Ok(RecoveryResult::Failed { reason }) => {
                let _ = self.event_bus.publish(LifecycleEvent::ResourceFailed {
                    resource_id: resource_id.clone(),
                    error: reason.clone(),
                    terminal: false,
                });
            }
            Err(_) => {}
        }

        result
    }

    /// Gracefully stop a resource.
    pub async fn stop_resource(&self, resource_id: &ResourceId) -> Result<(), ResourceError> {
        let instance = self
            .state_registry
            .get(resource_id)
            .await
            .ok_or_else(|| ResourceError::NotFound(resource_id.to_string()))?;

        // Stop health monitoring
        self.health_monitor.stop_tracking(resource_id).await;

        // Update state
        self.state_registry
            .update_state(
                resource_id,
                ResourceState::Completed {
                    at: Utc::now(),
                    result: None,
                },
                "Stopped gracefully".to_string(),
            )
            .await;

        let _ = self.event_bus.publish(LifecycleEvent::ResourceCompleted {
            resource_id: resource_id.clone(),
            result: None,
        });

        log::info!("LifecycleSupervisor: stopped {}", resource_id);

        Ok(())
    }

    /// Force kill a resource.
    pub async fn kill_resource(&self, resource_id: &ResourceId) -> Result<(), ResourceError> {
        let instance = self
            .state_registry
            .get(resource_id)
            .await
            .ok_or_else(|| ResourceError::NotFound(resource_id.to_string()))?;

        // Stop health monitoring
        self.health_monitor.stop_tracking(resource_id).await;

        // Update state
        self.state_registry
            .update_state(
                resource_id,
                ResourceState::Failed {
                    at: Utc::now(),
                    error: "Force killed".to_string(),
                    terminal: true,
                    escalation_tier_reached: instance.current_escalation_tier,
                },
                "Force killed".to_string(),
            )
            .await;

        let _ = self.event_bus.publish(LifecycleEvent::ResourceFailed {
            resource_id: resource_id.clone(),
            error: "Force killed".to_string(),
            terminal: true,
        });

        log::warn!("LifecycleSupervisor: killed {}", resource_id);

        Ok(())
    }

    /// Get pending user intervention requests.
    pub async fn get_pending_interventions(&self) -> Vec<UserInterventionRequest> {
        self.escalation_manager.get_pending_interventions().await
    }

    /// Resolve a user intervention request.
    pub async fn resolve_intervention(
        &self,
        request_id: &str,
        resolution: InterventionResolution,
    ) -> Result<(), ResourceError> {
        self.escalation_manager
            .resolve_intervention(request_id, resolution.clone())
            .await
            .map_err(|e| ResourceError::Internal(e.to_string()))?;

        let _ = self.event_bus.publish(LifecycleEvent::UserInterventionResolved {
            request_id: request_id.to_string(),
            selected_option: resolution.selected_option,
        });

        Ok(())
    }

    /// Start the background monitoring loop.
    pub async fn start_monitoring(self: Arc<Self>) {
        let mut running = self.running.write().await;
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let _ = self.event_bus.publish(LifecycleEvent::SupervisorStarted {
            timestamp: Utc::now(),
        });

        let supervisor = Arc::clone(&self);
        let handle = tokio::spawn(async move {
            let interval = Duration::from_secs(supervisor.config.health_check_interval_secs);
            let mut ticker = tokio::time::interval(interval);

            loop {
                ticker.tick().await;

                let is_running = *supervisor.running.read().await;
                if !is_running {
                    break;
                }

                // Perform health check
                supervisor.health_monitor.check_health().await;

                // Get stuck resources
                let stuck = supervisor.health_monitor.get_stuck_resources().await;

                // Process stuck resources
                for resource_id in stuck {
                    log::warn!(
                        "LifecycleSupervisor: detected stuck resource {}",
                        resource_id
                    );

                    // Get the instance
                    if let Some(instance) = supervisor.state_registry.get(&resource_id).await {
                        // Update state
                        supervisor
                            .state_registry
                            .update_state(
                                &resource_id,
                                ResourceState::Stuck {
                                    since: Utc::now(),
                                    recovery_attempts: instance.recovery_attempts,
                                    last_known_progress: None,
                                },
                                "Detected as stuck".to_string(),
                            )
                            .await;

                        // Get last heartbeat
                        let last_heartbeat = supervisor
                            .health_monitor
                            .get_last_heartbeat(&resource_id)
                            .await
                            .unwrap_or_else(Utc::now);

                        // Emit event
                        let _ = supervisor.event_bus.publish(LifecycleEvent::ResourceStuck {
                            resource_id: resource_id.clone(),
                            last_heartbeat,
                        });

                        // Check if we can auto-recover
                        if supervisor.escalation_manager.can_escalate(&instance).await {
                            // Attempt automatic recovery
                            if !supervisor.escalation_manager.is_cooldown_active(&resource_id.to_string()).await {
                                let _ = supervisor.recover_resource(&resource_id).await;
                            }
                        } else {
                            // Need user intervention
                            let request = supervisor
                                .escalation_manager
                                .create_intervention_request(&instance)
                                .await;

                            let _ = supervisor.event_bus.publish(
                                LifecycleEvent::UserInterventionNeeded { request },
                            );
                        }
                    }
                }

                // Get stats
                let stats = supervisor.health_monitor.get_stats().await;
                let _ = supervisor.event_bus.publish(LifecycleEvent::HealthCheckCompleted {
                    total_checked: stats.total_tracked,
                    stuck_found: stats.stuck,
                });
            }
        });

        let mut task = self.monitor_task.write().await;
        *task = Some(handle);
    }

    /// Stop the background monitoring loop.
    pub async fn stop_monitoring(&self) {
        let mut running = self.running.write().await;
        *running = false;

        let mut task = self.monitor_task.write().await;
        if let Some(handle) = task.take() {
            handle.abort();
        }

        let _ = self.event_bus.publish(LifecycleEvent::SupervisorStopped {
            timestamp: Utc::now(),
        });

        log::info!("LifecycleSupervisor: monitoring stopped");
    }

    /// Check if monitoring is running.
    pub async fn is_monitoring(&self) -> bool {
        *self.running.read().await
    }

    /// Get statistics about the supervisor.
    pub async fn get_stats(&self) -> SupervisorStats {
        let state_stats = self.state_registry.get_stats().await;
        let health_stats = self.health_monitor.get_stats().await;

        SupervisorStats {
            total_resources: state_stats.total,
            idle: state_stats.idle,
            running: state_stats.running,
            stuck: state_stats.stuck,
            recovering: state_stats.recovering,
            completed: state_stats.completed,
            failed: state_stats.failed,
            healthy: health_stats.healthy,
            degraded: health_stats.degraded,
            is_monitoring: self.is_monitoring().await,
        }
    }

    /// Get transition history for a resource.
    pub async fn get_transition_history(
        &self,
        resource_id: &ResourceId,
    ) -> Vec<super::states::StateTransition> {
        self.state_registry.get_history(resource_id).await
    }
}

/// Statistics about the supervisor.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupervisorStats {
    pub total_resources: usize,
    pub idle: usize,
    pub running: usize,
    pub stuck: usize,
    pub recovering: usize,
    pub completed: usize,
    pub failed: usize,
    pub healthy: usize,
    pub degraded: usize,
    pub is_monitoring: bool,
}

/// Thread-safe shared supervisor.
pub type SharedLifecycleSupervisor = Arc<LifecycleSupervisor>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_and_get_resource() {
        let supervisor = Arc::new(LifecycleSupervisor::new(SupervisorConfig::default()));

        let id = supervisor
            .spawn_resource(ResourceType::Agent, ResourceConfig::default())
            .await
            .unwrap();

        let resource = supervisor.get_resource(&id).await;
        assert!(resource.is_some());

        let state = supervisor.get_state(&id).await;
        assert!(matches!(state, Some(ResourceState::Running { .. })));
    }

    #[tokio::test]
    async fn test_record_heartbeat() {
        let supervisor = Arc::new(LifecycleSupervisor::new(SupervisorConfig::default()));

        let id = supervisor
            .spawn_resource(ResourceType::Agent, ResourceConfig::default())
            .await
            .unwrap();

        supervisor.record_heartbeat(&id).await;

        let health = supervisor.health_monitor.get_health(&id).await;
        assert!(matches!(health, HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_stop_resource() {
        let supervisor = Arc::new(LifecycleSupervisor::new(SupervisorConfig::default()));

        let id = supervisor
            .spawn_resource(ResourceType::Agent, ResourceConfig::default())
            .await
            .unwrap();

        supervisor.stop_resource(&id).await.unwrap();

        let state = supervisor.get_state(&id).await;
        assert!(matches!(state, Some(ResourceState::Completed { .. })));
    }

    #[tokio::test]
    async fn test_get_stats() {
        let supervisor = Arc::new(LifecycleSupervisor::new(SupervisorConfig::default()));

        supervisor
            .spawn_resource(ResourceType::Agent, ResourceConfig::default())
            .await
            .unwrap();

        let stats = supervisor.get_stats().await;
        assert_eq!(stats.total_resources, 1);
        assert_eq!(stats.running, 1);
    }
}
