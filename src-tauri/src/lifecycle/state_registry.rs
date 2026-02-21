//! State registry for tracking resource instances.
//!
//! The state registry maintains the authoritative state of all tracked
//! resources and logs state transitions for auditing.

use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::states::{ResourceId, ResourceInstance, ResourceState, ResourceType, StateTransition};

/// Maximum number of transitions to keep per resource.
const MAX_HISTORY_PER_RESOURCE: usize = 100;

/// Maximum total transitions in the global log.
const MAX_GLOBAL_HISTORY: usize = 10000;

/// Registry tracking all resource instances and their states.
pub struct StateRegistry {
    /// All tracked resources by ID
    resources: RwLock<HashMap<ResourceId, ResourceInstance>>,
    /// Resource type to IDs index for fast type-based queries
    type_index: RwLock<HashMap<ResourceType, HashSet<ResourceId>>>,
    /// Per-resource state transition history
    transition_history: RwLock<HashMap<ResourceId, Vec<StateTransition>>>,
    /// Global transition log (most recent first)
    global_history: RwLock<Vec<StateTransition>>,
}

impl StateRegistry {
    /// Create a new empty state registry.
    pub fn new() -> Self {
        Self {
            resources: RwLock::new(HashMap::new()),
            type_index: RwLock::new(HashMap::new()),
            transition_history: RwLock::new(HashMap::new()),
            global_history: RwLock::new(Vec::new()),
        }
    }

    /// Register a new resource instance.
    ///
    /// Returns `false` if a resource with the same ID already exists.
    pub async fn register(&self, instance: ResourceInstance) -> bool {
        let id = instance.id.clone();
        let resource_type = instance.resource_type.clone();

        {
            let mut resources = self.resources.write().await;
            if resources.contains_key(&id) {
                return false;
            }
            resources.insert(id.clone(), instance);
        }

        {
            let mut type_index = self.type_index.write().await;
            type_index
                .entry(resource_type)
                .or_insert_with(HashSet::new)
                .insert(id);
        }

        true
    }

    /// Unregister a resource by ID.
    ///
    /// Returns the removed instance if it existed.
    pub async fn unregister(&self, resource_id: &ResourceId) -> Option<ResourceInstance> {
        let instance = {
            let mut resources = self.resources.write().await;
            resources.remove(resource_id)
        };

        if let Some(ref inst) = instance {
            let mut type_index = self.type_index.write().await;
            if let Some(ids) = type_index.get_mut(&inst.resource_type) {
                ids.remove(resource_id);
                if ids.is_empty() {
                    type_index.remove(&inst.resource_type);
                }
            }
        }

        instance
    }

    /// Update the state of a resource.
    ///
    /// This logs the transition to the resource's history.
    pub async fn update_state(
        &self,
        resource_id: &ResourceId,
        new_state: ResourceState,
        reason: String,
    ) -> bool {
        let transition = {
            let mut resources = self.resources.write().await;

            if let Some(instance) = resources.get_mut(resource_id) {
                let from_state = format!("{:?}", instance.state);
                let to_state = format!("{:?}", new_state);

                let transition = StateTransition {
                    resource_id: resource_id.to_string(),
                    from_state: from_state.clone(),
                    to_state: to_state.clone(),
                    timestamp: Utc::now(),
                    reason: reason.clone(),
                };

                instance.state = new_state;

                Some(transition)
            } else {
                None
            }
        };

        if let Some(transition) = transition {
            // Add to per-resource history
            {
                let mut history = self.transition_history.write().await;
                let resource_history = history.entry(resource_id.clone()).or_insert_with(Vec::new);
                resource_history.push(transition.clone());

                // Trim if too long
                if resource_history.len() > MAX_HISTORY_PER_RESOURCE {
                    let excess = resource_history.len() - MAX_HISTORY_PER_RESOURCE;
                    resource_history.drain(0..excess);
                }
            }

            // Add to global history
            {
                let mut global = self.global_history.write().await;
                global.push(transition);

                // Trim if too long
                if global.len() > MAX_GLOBAL_HISTORY {
                    let excess = global.len() - MAX_GLOBAL_HISTORY;
                    global.drain(0..excess);
                }
            }

            true
        } else {
            false
        }
    }

    /// Get a resource by ID.
    pub async fn get(&self, resource_id: &ResourceId) -> Option<ResourceInstance> {
        let resources = self.resources.read().await;
        resources.get(resource_id).cloned()
    }

    /// Check if a resource exists.
    pub async fn contains(&self, resource_id: &ResourceId) -> bool {
        let resources = self.resources.read().await;
        resources.contains_key(resource_id)
    }

    /// Get all resources of a specific type.
    pub async fn get_by_type(&self, resource_type: &ResourceType) -> Vec<ResourceInstance> {
        let resources = self.resources.read().await;
        let type_index = self.type_index.read().await;

        type_index
            .get(resource_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| resources.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all tracked resources.
    pub async fn get_all(&self) -> Vec<ResourceInstance> {
        let resources = self.resources.read().await;
        resources.values().cloned().collect()
    }

    /// Get the number of tracked resources.
    pub async fn count(&self) -> usize {
        let resources = self.resources.read().await;
        resources.len()
    }

    /// Get resources by state type.
    pub async fn get_by_state(
        &self,
        predicate: impl Fn(&ResourceState) -> bool,
    ) -> Vec<ResourceInstance> {
        let resources = self.resources.read().await;
        resources
            .values()
            .filter(|r| predicate(&r.state))
            .cloned()
            .collect()
    }

    /// Get stuck resources.
    pub async fn get_stuck(&self) -> Vec<ResourceInstance> {
        self.get_by_state(|s| matches!(s, ResourceState::Stuck { .. }))
            .await
    }

    /// Get running resources.
    pub async fn get_running(&self) -> Vec<ResourceInstance> {
        self.get_by_state(|s| matches!(s, ResourceState::Running { .. }))
            .await
    }

    /// Get the transition history for a resource.
    pub async fn get_history(&self, resource_id: &ResourceId) -> Vec<StateTransition> {
        let history = self.transition_history.read().await;
        history
            .get(resource_id)
            .map(|h| h.clone())
            .unwrap_or_default()
    }

    /// Get the global transition log.
    pub async fn get_global_history(&self) -> Vec<StateTransition> {
        let global = self.global_history.read().await;
        global.clone()
    }

    /// Get recent transitions (globally).
    pub async fn get_recent_transitions(&self, limit: usize) -> Vec<StateTransition> {
        let global = self.global_history.read().await;
        global.iter().take(limit).cloned().collect()
    }

    /// Increment the recovery attempt counter for a resource.
    pub async fn increment_recovery_attempts(&self, resource_id: &ResourceId) -> u32 {
        let mut resources = self.resources.write().await;
        if let Some(instance) = resources.get_mut(resource_id) {
            instance.recovery_attempts += 1;
            instance.recovery_attempts
        } else {
            0
        }
    }

    /// Set the escalation tier for a resource.
    pub async fn set_escalation_tier(&self, resource_id: &ResourceId, tier: u8) -> bool {
        let mut resources = self.resources.write().await;
        if let Some(instance) = resources.get_mut(resource_id) {
            instance.current_escalation_tier = tier;
            true
        } else {
            false
        }
    }

    /// Get statistics about the registry.
    pub async fn get_stats(&self) -> StateRegistryStats {
        let resources = self.resources.read().await;

        let mut stats = StateRegistryStats::default();
        stats.total = resources.len();

        for (_, instance) in resources.iter() {
            match &instance.state {
                ResourceState::Idle => stats.idle += 1,
                ResourceState::Running { .. } => stats.running += 1,
                ResourceState::Stuck { .. } => stats.stuck += 1,
                ResourceState::Recovering { .. } => stats.recovering += 1,
                ResourceState::Completed { .. } => stats.completed += 1,
                ResourceState::Failed { .. } => stats.failed += 1,
            }
        }

        stats
    }

    /// Clear all resources (for testing/reset).
    pub async fn clear(&self) {
        let mut resources = self.resources.write().await;
        let mut type_index = self.type_index.write().await;
        let mut history = self.transition_history.write().await;
        let mut global = self.global_history.write().await;

        resources.clear();
        type_index.clear();
        history.clear();
        global.clear();
    }
}

impl Default for StateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the state registry.
#[derive(Debug, Clone, Default)]
pub struct StateRegistryStats {
    /// Total number of resources
    pub total: usize,
    /// Resources in idle state
    pub idle: usize,
    /// Resources in running state
    pub running: usize,
    /// Resources in stuck state
    pub stuck: usize,
    /// Resources in recovering state
    pub recovering: usize,
    /// Resources in completed state
    pub completed: usize,
    /// Resources in failed state
    pub failed: usize,
}

/// Thread-safe shared state registry.
pub type SharedStateRegistry = Arc<StateRegistry>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::states::ResourceConfig;

    #[tokio::test]
    async fn test_register_and_get() {
        let registry = StateRegistry::new();

        let instance = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );

        assert!(registry.register(instance.clone()).await);
        assert!(!registry.register(instance.clone()).await); // Duplicate

        let retrieved = registry.get(&instance.id).await;
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_unregister() {
        let registry = StateRegistry::new();

        let instance = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );

        registry.register(instance.clone()).await;

        let removed = registry.unregister(&instance.id).await;
        assert!(removed.is_some());

        let retrieved = registry.get(&instance.id).await;
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_update_state() {
        let registry = StateRegistry::new();

        let instance = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );

        registry.register(instance.clone()).await;

        let new_state = ResourceState::Running {
            substate: "thinking".to_string(),
            started_at: Utc::now(),
            progress: None,
        };

        let updated = registry
            .update_state(
                &instance.id,
                new_state.clone(),
                "Starting execution".to_string(),
            )
            .await;
        assert!(updated);

        let retrieved = registry.get(&instance.id).await.unwrap();
        assert!(matches!(retrieved.state, ResourceState::Running { .. }));

        // Check history
        let history = registry.get_history(&instance.id).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].reason, "Starting execution");
    }

    #[tokio::test]
    async fn test_get_by_type() {
        let registry = StateRegistry::new();

        let agent1 = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );
        let agent2 = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:2"),
            ResourceConfig::default(),
        );
        let channel = ResourceInstance::new(
            ResourceId::new(ResourceType::Channel, "test:telegram"),
            ResourceConfig::default(),
        );

        registry.register(agent1).await;
        registry.register(agent2).await;
        registry.register(channel).await;

        let agents = registry.get_by_type(&ResourceType::Agent).await;
        assert_eq!(agents.len(), 2);

        let channels = registry.get_by_type(&ResourceType::Channel).await;
        assert_eq!(channels.len(), 1);

        let tools = registry.get_by_type(&ResourceType::Tool).await;
        assert!(tools.is_empty());
    }

    #[tokio::test]
    async fn test_get_stuck() {
        let registry = StateRegistry::new();

        let mut running = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );
        running.state = ResourceState::Running {
            substate: "thinking".to_string(),
            started_at: Utc::now(),
            progress: None,
        };

        let mut stuck = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:2"),
            ResourceConfig::default(),
        );
        stuck.state = ResourceState::Stuck {
            since: Utc::now(),
            recovery_attempts: 0,
            last_known_progress: None,
        };

        registry.register(running).await;
        registry.register(stuck).await;

        let stuck_resources = registry.get_stuck().await;
        assert_eq!(stuck_resources.len(), 1);
        assert_eq!(stuck_resources[0].id.instance_id(), "test:2");
    }
}
