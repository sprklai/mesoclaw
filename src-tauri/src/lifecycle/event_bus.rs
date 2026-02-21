//! Lifecycle event bus for resource lifecycle events.
//!
//! This module extends the application event bus with lifecycle-specific events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use super::states::{
    RecoveryActionType, ResourceId, ResourceType, UserInterventionRequest,
};

/// All lifecycle events that flow through the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LifecycleEvent {
    /// Resource started
    ResourceStarted {
        resource_id: ResourceId,
        resource_type: ResourceType,
    },

    /// Heartbeat received from resource
    ResourceHeartbeat {
        resource_id: ResourceId,
        timestamp: DateTime<Utc>,
    },

    /// Resource progress update
    ResourceProgress {
        resource_id: ResourceId,
        progress: f32,
        substate: String,
    },

    /// Resource detected as stuck
    ResourceStuck {
        resource_id: ResourceId,
        last_heartbeat: DateTime<Utc>,
    },

    /// Resource recovery started
    ResourceRecovering {
        resource_id: ResourceId,
        action: RecoveryActionType,
    },

    /// Resource being transferred
    ResourceTransferring {
        from_id: ResourceId,
        to_id: ResourceId,
    },

    /// Resource successfully recovered
    ResourceRecovered {
        resource_id: ResourceId,
        tier: u8,
    },

    /// Resource failed (terminal or non-terminal)
    ResourceFailed {
        resource_id: ResourceId,
        error: String,
        terminal: bool,
    },

    /// Resource completed successfully
    ResourceCompleted {
        resource_id: ResourceId,
        result: Option<serde_json::Value>,
    },

    /// User intervention needed
    UserInterventionNeeded {
        request: UserInterventionRequest,
    },

    /// User intervention resolved
    UserInterventionResolved {
        request_id: String,
        selected_option: String,
    },

    /// Health check sweep completed
    HealthCheckCompleted {
        total_checked: usize,
        stuck_found: usize,
    },

    /// Supervisor started
    SupervisorStarted {
        timestamp: DateTime<Utc>,
    },

    /// Supervisor stopped
    SupervisorStopped {
        timestamp: DateTime<Utc>,
    },
}

impl LifecycleEvent {
    /// Get the event type name.
    pub fn event_type(&self) -> &'static str {
        match self {
            LifecycleEvent::ResourceStarted { .. } => "resource_started",
            LifecycleEvent::ResourceHeartbeat { .. } => "resource_heartbeat",
            LifecycleEvent::ResourceProgress { .. } => "resource_progress",
            LifecycleEvent::ResourceStuck { .. } => "resource_stuck",
            LifecycleEvent::ResourceRecovering { .. } => "resource_recovering",
            LifecycleEvent::ResourceTransferring { .. } => "resource_transferring",
            LifecycleEvent::ResourceRecovered { .. } => "resource_recovered",
            LifecycleEvent::ResourceFailed { .. } => "resource_failed",
            LifecycleEvent::ResourceCompleted { .. } => "resource_completed",
            LifecycleEvent::UserInterventionNeeded { .. } => "user_intervention_needed",
            LifecycleEvent::UserInterventionResolved { .. } => "user_intervention_resolved",
            LifecycleEvent::HealthCheckCompleted { .. } => "health_check_completed",
            LifecycleEvent::SupervisorStarted { .. } => "supervisor_started",
            LifecycleEvent::SupervisorStopped { .. } => "supervisor_stopped",
        }
    }

    /// Get the resource ID if this event is about a specific resource.
    pub fn resource_id(&self) -> Option<&ResourceId> {
        match self {
            LifecycleEvent::ResourceStarted { resource_id, .. } => Some(resource_id),
            LifecycleEvent::ResourceHeartbeat { resource_id, .. } => Some(resource_id),
            LifecycleEvent::ResourceProgress { resource_id, .. } => Some(resource_id),
            LifecycleEvent::ResourceStuck { resource_id, .. } => Some(resource_id),
            LifecycleEvent::ResourceRecovering { resource_id, .. } => Some(resource_id),
            LifecycleEvent::ResourceTransferring { from_id, .. } => Some(from_id),
            LifecycleEvent::ResourceRecovered { resource_id, .. } => Some(resource_id),
            LifecycleEvent::ResourceFailed { resource_id, .. } => Some(resource_id),
            LifecycleEvent::ResourceCompleted { resource_id, .. } => Some(resource_id),
            LifecycleEvent::UserInterventionNeeded { request, .. } => {
                // Parse the resource_id from the request
                None // Request has a string, not ResourceId
            }
            _ => None,
        }
    }
}

/// Event bus specifically for lifecycle events.
pub struct LifecycleEventBus {
    sender: broadcast::Sender<LifecycleEvent>,
}

impl LifecycleEventBus {
    /// Create a new lifecycle event bus.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1024);
        Self { sender }
    }

    /// Create with a specific capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Publish a lifecycle event.
    pub fn publish(&self, event: LifecycleEvent) -> Result<(), String> {
        self.sender
            .send(event)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Subscribe to all lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<LifecycleEvent> {
        self.sender.subscribe()
    }

    /// Get a clone of the sender for publishing from other contexts.
    pub fn sender(&self) -> broadcast::Sender<LifecycleEvent> {
        self.sender.clone()
    }
}

impl Default for LifecycleEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for LifecycleEventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_and_receive() {
        let bus = LifecycleEventBus::new();
        let mut rx = bus.subscribe();

        let event = LifecycleEvent::SupervisorStarted {
            timestamp: Utc::now(),
        };

        bus.publish(event.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        assert!(matches!(received, LifecycleEvent::SupervisorStarted { .. }));
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = LifecycleEventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let event = LifecycleEvent::ResourceStarted {
            resource_id: ResourceId::new(ResourceType::Agent, "test:1"),
            resource_type: ResourceType::Agent,
        };

        bus.publish(event.clone()).unwrap();

        assert!(matches!(rx1.recv().await.unwrap(), LifecycleEvent::ResourceStarted { .. }));
        assert!(matches!(rx2.recv().await.unwrap(), LifecycleEvent::ResourceStarted { .. }));
    }
}
