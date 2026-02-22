//! Unified lifecycle manager combining SessionRouter and StateRegistry.
//!
//! This module provides a single entry point for managing resource lifecycles,
//! integrating session management with state tracking and persistence.

use std::sync::Arc;
use tauri::AppHandle;

use crate::agent::session_router::{SessionKey, SessionRouter};
use crate::event_bus::EventBus;

use super::events;
use super::states::{
    ResourceConfig, ResourceId, ResourceInstance, ResourceState, ResourceType, StateTransition,
};
use super::state_registry::StateRegistry;
use super::storage::LifecycleStorage;

/// Unified lifecycle manager combining SessionRouter and StateRegistry.
///
/// This is the central point for managing resource lifecycles:
/// - Creating and registering sessions
/// - Tracking state transitions
/// - Persisting state to database
/// - Emitting real-time events to frontend
pub struct LifecycleManager {
    /// Session router for message history
    sessions: Arc<SessionRouter>,
    /// State registry for lifecycle tracking
    registry: Arc<StateRegistry>,
    /// Persistent storage
    storage: Arc<LifecycleStorage>,
    /// Event bus for internal events
    event_bus: Arc<dyn EventBus>,
    /// Tauri app handle for emitting events to frontend
    app_handle: Option<AppHandle>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager.
    pub fn new(
        sessions: Arc<SessionRouter>,
        registry: Arc<StateRegistry>,
        storage: Arc<LifecycleStorage>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            sessions,
            registry,
            storage,
            event_bus,
            app_handle: None,
        }
    }

    /// Set the Tauri app handle for emitting events.
    pub fn set_app_handle(&mut self, handle: AppHandle) {
        self.app_handle = Some(handle);
    }

    /// Create a session and register it with lifecycle tracking.
    ///
    /// This combines:
    /// 1. Creating the session in SessionRouter
    /// 2. Registering with StateRegistry
    /// 3. Persisting to SQLite
    /// 4. Emitting real-time event
    pub async fn create_session(
        &self,
        key: SessionKey,
        config: ResourceConfig,
    ) -> Result<ResourceInstance, String> {
        log::info!("LifecycleManager: creating session {}", key);

        // 1. Create in SessionRouter
        self.sessions.create_session(key.clone())?;

        // 2. Create resource ID and instance
        let resource_id = ResourceId::new(ResourceType::Agent, key.as_str());
        let instance = ResourceInstance::new(resource_id.clone(), config);

        // 3. Register in StateRegistry
        if !self.registry.register(instance.clone()).await {
            return Err(format!("Resource {} already registered", resource_id));
        }

        // 4. Persist to SQLite
        self.storage.save_instance(&instance)?;

        // 5. Emit event
        if let Some(ref app) = self.app_handle {
            let _ = events::emit_session_created(app, &instance);
        }

        log::info!("LifecycleManager: session {} created and registered", key);
        Ok(instance)
    }

    /// Update session substate (thinking, executing_tool, etc.).
    pub async fn update_substate(
        &self,
        key: &SessionKey,
        substate: &str,
        progress: Option<f32>,
    ) -> Result<(), String> {
        let resource_id = ResourceId::new(ResourceType::Agent, key.as_str());

        // Get current instance
        let current = self
            .registry
            .get(&resource_id)
            .await
            .ok_or_else(|| format!("Resource {} not found", resource_id))?;

        // Create new state
        let new_state = ResourceState::Running {
            substate: substate.to_string(),
            started_at: chrono::Utc::now(),
            progress,
        };

        // Update registry
        let from_state = format!("{:?}", current.state);
        self.registry
            .update_state(&resource_id, new_state.clone(), format!("Substate: {}", substate))
            .await;

        // Record transition
        let transition = StateTransition {
            resource_id: resource_id.to_string(),
            from_state: from_state.clone(),
            to_state: format!("{:?}", new_state),
            timestamp: chrono::Utc::now(),
            reason: format!("Substate change: {}", substate),
        };
        self.storage.record_transition(&transition, Some(substate))?;

        // Update persistent storage
        let updated_instance = ResourceInstance {
            state: new_state.clone(),
            ..current
        };
        self.storage.save_instance(&updated_instance)?;

        // Emit event
        if let Some(ref app) = self.app_handle {
            let _ = events::emit_progress_updated(app, &resource_id.to_string(), progress.unwrap_or(0.0), substate);
            let _ = events::emit_state_changed(
                app,
                &resource_id.to_string(),
                "agent",
                &from_state,
                "running",
                Some(substate),
                progress,
            );
        }

        Ok(())
    }

    /// Mark a session as completed.
    pub async fn complete(&self, key: &SessionKey) -> Result<(), String> {
        let resource_id = ResourceId::new(ResourceType::Agent, key.as_str());

        // Get current instance
        let current = self
            .registry
            .get(&resource_id)
            .await
            .ok_or_else(|| format!("Resource {} not found", resource_id))?;

        let from_state = format!("{:?}", current.state);
        let new_state = ResourceState::Completed {
            at: chrono::Utc::now(),
            result: None,
        };

        // Update registry
        self.registry
            .update_state(&resource_id, new_state.clone(), "Session completed".to_string())
            .await;

        // Record transition
        let transition = StateTransition {
            resource_id: resource_id.to_string(),
            from_state: from_state.clone(),
            to_state: "completed".to_string(),
            timestamp: chrono::Utc::now(),
            reason: "Session completed".to_string(),
        };
        self.storage.record_transition(&transition, None)?;

        // Update persistent storage
        let updated_instance = ResourceInstance {
            state: new_state,
            ..current
        };
        self.storage.save_instance(&updated_instance)?;

        // Emit event
        if let Some(ref app) = self.app_handle {
            let _ = events::emit_session_completed(app, &resource_id.to_string());
        }

        log::info!("LifecycleManager: session {} completed", key);
        Ok(())
    }

    /// Mark a session as failed.
    pub async fn fail(&self, key: &SessionKey, error: &str, terminal: bool) -> Result<(), String> {
        let resource_id = ResourceId::new(ResourceType::Agent, key.as_str());

        // Get current instance
        let current = self
            .registry
            .get(&resource_id)
            .await
            .ok_or_else(|| format!("Resource {} not found", resource_id))?;

        let from_state = format!("{:?}", current.state);
        let new_state = ResourceState::Failed {
            at: chrono::Utc::now(),
            error: error.to_string(),
            terminal,
            escalation_tier_reached: current.current_escalation_tier,
        };

        // Update registry
        self.registry
            .update_state(&resource_id, new_state.clone(), format!("Failed: {}", error))
            .await;

        // Record transition
        let transition = StateTransition {
            resource_id: resource_id.to_string(),
            from_state: from_state.clone(),
            to_state: "failed".to_string(),
            timestamp: chrono::Utc::now(),
            reason: error.to_string(),
        };
        self.storage.record_transition(&transition, None)?;

        // Update persistent storage
        let updated_instance = ResourceInstance {
            state: new_state,
            ..current
        };
        self.storage.save_instance(&updated_instance)?;

        // Emit event
        if let Some(ref app) = self.app_handle {
            let _ = events::emit_session_failed(app, &resource_id.to_string(), error);
        }

        log::warn!("LifecycleManager: session {} failed: {}", key, error);
        Ok(())
    }

    /// Stop a session gracefully.
    pub async fn stop(&self, key: &SessionKey) -> Result<(), String> {
        let resource_id = ResourceId::new(ResourceType::Agent, key.as_str());

        // Get current instance
        let current = self
            .registry
            .get(&resource_id)
            .await
            .ok_or_else(|| format!("Resource {} not found", resource_id))?;

        let from_state = format!("{:?}", current.state);

        // Update registry to idle
        self.registry
            .update_state(&resource_id, ResourceState::Idle, "Stopped by user".to_string())
            .await;

        // Record transition
        let transition = StateTransition {
            resource_id: resource_id.to_string(),
            from_state: from_state.clone(),
            to_state: "idle".to_string(),
            timestamp: chrono::Utc::now(),
            reason: "Stopped by user".to_string(),
        };
        self.storage.record_transition(&transition, None)?;

        // Remove from persistent storage (session ended gracefully)
        self.storage.remove_instance(&resource_id)?;

        // Unregister from registry
        self.registry.unregister(&resource_id).await;

        log::info!("LifecycleManager: session {} stopped", key);
        Ok(())
    }

    /// Kill a session forcefully.
    pub async fn kill(&self, key: &SessionKey) -> Result<(), String> {
        let resource_id = ResourceId::new(ResourceType::Agent, key.as_str());

        // Get current instance
        let current = self
            .registry
            .get(&resource_id)
            .await
            .ok_or_else(|| format!("Resource {} not found", resource_id))?;

        // Mark as failed
        self.fail(key, "Killed by user", false).await?;

        log::warn!("LifecycleManager: session {} killed", key);
        Ok(())
    }

    /// Restore instances from database on startup.
    pub async fn restore_from_storage(&self) -> Result<Vec<ResourceInstance>, String> {
        log::info!("LifecycleManager: restoring instances from storage...");

        let instances = self.storage.load_active_instances()?;

        for instance in &instances {
            // Re-register with in-memory registry
            self.registry.register(instance.clone()).await;

            log::info!(
                "LifecycleManager: restored instance {} in state {:?}",
                instance.id,
                instance.state
            );
        }

        log::info!("LifecycleManager: restored {} instances", instances.len());
        Ok(instances)
    }

    /// Get all active resources.
    pub async fn get_all_resources(&self) -> Vec<ResourceInstance> {
        self.registry.get_all().await
    }

    /// Get resources by type.
    pub async fn get_resources_by_type(&self, resource_type: &ResourceType) -> Vec<ResourceInstance> {
        self.registry.get_by_type(resource_type).await
    }

    /// Get a specific resource.
    pub async fn get_resource(&self, resource_id: &ResourceId) -> Option<ResourceInstance> {
        self.registry.get(resource_id).await
    }

    /// Get stuck resources.
    pub async fn get_stuck_resources(&self) -> Vec<ResourceInstance> {
        self.registry.get_stuck().await
    }

    /// Get running resources.
    pub async fn get_running_resources(&self) -> Vec<ResourceInstance> {
        self.registry.get_running().await
    }

    /// Record a heartbeat for a resource.
    pub async fn record_heartbeat(&self, resource_id: &ResourceId) -> Result<(), String> {
        // For now, heartbeats are just logged
        // In the future, this could update a last_seen timestamp
        log::debug!("LifecycleManager: heartbeat for {}", resource_id);
        Ok(())
    }

    /// Get the underlying session router.
    pub fn sessions(&self) -> &Arc<SessionRouter> {
        &self.sessions
    }

    /// Get the underlying state registry.
    pub fn registry(&self) -> &Arc<StateRegistry> {
        &self.registry
    }

    /// Get the underlying storage.
    pub fn storage(&self) -> &Arc<LifecycleStorage> {
        &self.storage
    }
}

/// Thread-safe shared lifecycle manager.
pub type SharedLifecycleManager = Arc<LifecycleManager>;

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full tests would require a mock SessionRouter and database.
    // These tests verify the basic structure and type signatures.

    #[test]
    fn test_resource_id_creation() {
        let id = ResourceId::new(ResourceType::Agent, "test:session");
        assert_eq!(id.to_string(), "agent:test:session");
    }
}
