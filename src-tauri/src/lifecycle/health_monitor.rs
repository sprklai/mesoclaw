//! Health monitoring for managed resources.
//!
//! The health monitor tracks heartbeats from resources and detects
//! when they become stuck (stop sending heartbeats).

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio::task::JoinHandle;
use chrono::{DateTime, Utc};

use super::states::{HealthStatus, HeartbeatConfig, ResourceId, ResourceInstance, ResourceType};

/// Internal state tracked for each monitored resource.
#[derive(Debug, Clone)]
struct HeartbeatState {
    /// Last heartbeat timestamp
    last_heartbeat: DateTime<Utc>,
    /// Number of consecutive missed heartbeats
    missed_count: u32,
    /// Current health status
    status: HealthStatus,
    /// Heartbeat configuration for this resource
    config: HeartbeatConfig,
}

impl HeartbeatState {
    fn new(config: HeartbeatConfig) -> Self {
        Self {
            last_heartbeat: Utc::now(),
            missed_count: 0,
            status: HealthStatus::Healthy,
            config,
        }
    }
}

/// Events emitted by the health monitor.
#[derive(Debug, Clone)]
pub enum HealthMonitorEvent {
    /// A heartbeat was recorded
    HeartbeatRecorded {
        resource_id: ResourceId,
        timestamp: DateTime<Utc>,
    },
    /// A resource health status changed
    HealthChanged {
        resource_id: ResourceId,
        old_status: HealthStatus,
        new_status: HealthStatus,
    },
    /// A resource was detected as stuck
    ResourceStuck {
        resource_id: ResourceId,
        last_heartbeat: DateTime<Utc>,
    },
    /// A stuck resource recovered
    ResourceRecovered {
        resource_id: ResourceId,
    },
}

/// Monitors the health of tracked resources via heartbeats.
pub struct HealthMonitor {
    /// Heartbeat state by resource ID
    heartbeats: Arc<RwLock<HashMap<ResourceId, HeartbeatState>>>,
    /// Background health check task handles
    check_tasks: Arc<RwLock<HashMap<ResourceId, JoinHandle<()>>>>,
    /// Event sender for health monitor events
    event_sender: broadcast::Sender<HealthMonitorEvent>,
    /// Default heartbeat configuration
    default_config: HeartbeatConfig,
}

impl HealthMonitor {
    /// Create a new health monitor.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self {
            heartbeats: Arc::new(RwLock::new(HashMap::new())),
            check_tasks: Arc::new(RwLock::new(HashMap::new())),
            event_sender: sender,
            default_config: HeartbeatConfig::default(),
        }
    }

    /// Create a health monitor with a custom default configuration.
    pub fn with_config(config: HeartbeatConfig) -> Self {
        let (sender, _) = broadcast::channel(256);
        Self {
            heartbeats: Arc::new(RwLock::new(HashMap::new())),
            check_tasks: Arc::new(RwLock::new(HashMap::new())),
            event_sender: sender,
            default_config: config,
        }
    }

    /// Subscribe to health monitor events.
    pub fn subscribe(&self) -> broadcast::Receiver<HealthMonitorEvent> {
        self.event_sender.subscribe()
    }

    /// Start tracking a resource.
    ///
    /// The resource will be monitored for heartbeats. If heartbeats
    /// stop arriving, the resource will be marked as stuck.
    pub async fn start_tracking(&self, resource: &ResourceInstance) {
        let config = HeartbeatConfig::for_resource_type(&resource.resource_type);
        let state = HeartbeatState::new(config);

        let mut heartbeats = self.heartbeats.write().await;
        heartbeats.insert(resource.id.clone(), state);

        log::debug!(
            "HealthMonitor: started tracking {} with config (interval={}s, threshold={})",
            resource.id,
            heartbeats.get(&resource.id).unwrap().config.interval_secs,
            heartbeats.get(&resource.id).unwrap().config.stuck_threshold
        );
    }

    /// Stop tracking a resource.
    pub async fn stop_tracking(&self, resource_id: &ResourceId) {
        // Cancel any running health check task
        {
            let mut tasks = self.check_tasks.write().await;
            if let Some(handle) = tasks.remove(resource_id) {
                handle.abort();
            }
        }

        // Remove heartbeat state
        {
            let mut heartbeats = self.heartbeats.write().await;
            heartbeats.remove(resource_id);
        }

        log::debug!("HealthMonitor: stopped tracking {}", resource_id);
    }

    /// Record a heartbeat from a resource.
    ///
    /// This should be called by resources periodically to indicate
    /// they are still alive and functioning.
    pub async fn record_heartbeat(&self, resource_id: &ResourceId) {
        let mut heartbeats = self.heartbeats.write().await;

        if let Some(state) = heartbeats.get_mut(resource_id) {
            let now = Utc::now();
            let old_status = state.status.clone();

            // Update heartbeat state
            state.last_heartbeat = now;
            state.missed_count = 0;

            // If was stuck/degraded, now healthy
            if !matches!(state.status, HealthStatus::Healthy) {
                state.status = HealthStatus::Healthy;

                let _ = self.event_sender.send(HealthMonitorEvent::ResourceRecovered {
                    resource_id: resource_id.clone(),
                });
            }

            let _ = self.event_sender.send(HealthMonitorEvent::HeartbeatRecorded {
                resource_id: resource_id.clone(),
                timestamp: now,
            });

            if old_status != state.status {
                let _ = self.event_sender.send(HealthMonitorEvent::HealthChanged {
                    resource_id: resource_id.clone(),
                    old_status,
                    new_status: state.status.clone(),
                });
            }

            log::trace!("HealthMonitor: heartbeat from {}", resource_id);
        } else {
            log::warn!(
                "HealthMonitor: received heartbeat for untracked resource {}",
                resource_id
            );
        }
    }

    /// Get the current health status of a resource.
    pub async fn get_health(&self, resource_id: &ResourceId) -> HealthStatus {
        let heartbeats = self.heartbeats.read().await;
        heartbeats
            .get(resource_id)
            .map(|s| s.status.clone())
            .unwrap_or(HealthStatus::Unknown)
    }

    /// Get all resources currently marked as stuck.
    pub async fn get_stuck_resources(&self) -> Vec<ResourceId> {
        let heartbeats = self.heartbeats.read().await;
        heartbeats
            .iter()
            .filter(|(_, state)| matches!(state.status, HealthStatus::Stuck { .. }))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get all tracked resource IDs.
    pub async fn get_tracked_resources(&self) -> Vec<ResourceId> {
        let heartbeats = self.heartbeats.read().await;
        heartbeats.keys().cloned().collect()
    }

    /// Perform a health check sweep.
    ///
    /// This should be called periodically by the supervisor to
    /// detect stuck resources.
    pub async fn check_health(&self) {
        let mut heartbeats = self.heartbeats.write().await;
        let now = Utc::now();

        for (resource_id, state) in heartbeats.iter_mut() {
            let elapsed = (now - state.last_heartbeat)
                .to_std()
                .unwrap_or(Duration::ZERO);
            let interval = Duration::from_secs(state.config.interval_secs);

            // Check if we've missed a heartbeat interval
            if elapsed >= interval {
                state.missed_count += 1;

                // Update status based on missed count
                let old_status = state.status.clone();

                if state.missed_count >= state.config.stuck_threshold {
                    // Mark as stuck
                    state.status = HealthStatus::Stuck { since: now };

                    log::warn!(
                        "HealthMonitor: {} is stuck (missed {} heartbeats, last at {})",
                        resource_id,
                        state.missed_count,
                        state.last_heartbeat
                    );

                    let _ = self.event_sender.send(HealthMonitorEvent::ResourceStuck {
                        resource_id: resource_id.clone(),
                        last_heartbeat: state.last_heartbeat,
                    });
                } else if state.missed_count > 0 {
                    // Mark as degraded
                    state.status = HealthStatus::Degraded {
                        missed: state.missed_count,
                    };

                    log::debug!(
                        "HealthMonitor: {} is degraded (missed {} heartbeats)",
                        resource_id,
                        state.missed_count
                    );
                }

                if old_status != state.status {
                    let _ = self.event_sender.send(HealthMonitorEvent::HealthChanged {
                        resource_id: resource_id.clone(),
                        old_status,
                        new_status: state.status.clone(),
                    });
                }
            }
        }
    }

    /// Update the heartbeat configuration for a resource.
    pub async fn update_config(&self, resource_id: &ResourceId, config: HeartbeatConfig) {
        let mut heartbeats = self.heartbeats.write().await;
        if let Some(state) = heartbeats.get_mut(resource_id) {
            state.config = config;
        }
    }

    /// Get the last heartbeat time for a resource.
    pub async fn get_last_heartbeat(&self, resource_id: &ResourceId) -> Option<DateTime<Utc>> {
        let heartbeats = self.heartbeats.read().await;
        heartbeats.get(resource_id).map(|s| s.last_heartbeat)
    }

    /// Get statistics about monitored resources.
    pub async fn get_stats(&self) -> HealthMonitorStats {
        let heartbeats = self.heartbeats.read().await;

        let mut stats = HealthMonitorStats::default();

        for (_, state) in heartbeats.iter() {
            stats.total_tracked += 1;
            match &state.status {
                HealthStatus::Healthy => stats.healthy += 1,
                HealthStatus::Degraded { .. } => stats.degraded += 1,
                HealthStatus::Stuck { .. } => stats.stuck += 1,
                HealthStatus::Unknown => stats.unknown += 1,
            }
        }

        stats
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about monitored resources.
#[derive(Debug, Clone, Default)]
pub struct HealthMonitorStats {
    /// Total resources being tracked
    pub total_tracked: usize,
    /// Resources with healthy status
    pub healthy: usize,
    /// Resources with degraded status
    pub degraded: usize,
    /// Resources that are stuck
    pub stuck: usize,
    /// Resources with unknown status
    pub unknown: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::states::{ResourceType, ResourceConfig};

    #[tokio::test]
    async fn test_start_stop_tracking() {
        let monitor = HealthMonitor::new();

        let resource = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );

        monitor.start_tracking(&resource).await;

        let tracked = monitor.get_tracked_resources().await;
        assert_eq!(tracked.len(), 1);

        monitor.stop_tracking(&resource.id).await;

        let tracked = monitor.get_tracked_resources().await;
        assert!(tracked.is_empty());
    }

    #[tokio::test]
    async fn test_record_heartbeat() {
        let monitor = HealthMonitor::new();

        let resource = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );

        monitor.start_tracking(&resource).await;
        monitor.record_heartbeat(&resource.id).await;

        let health = monitor.get_health(&resource.id).await;
        assert!(matches!(health, HealthStatus::Healthy));
    }

    #[tokio::test]
    async fn test_detect_stuck() {
        let monitor = HealthMonitor::new();

        // Use a custom config with very short interval
        let config = HeartbeatConfig {
            interval_secs: 0,
            ..Default::default()
        };

        let resource = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );

        monitor.start_tracking(&resource).await;

        // Manually set a very old heartbeat time and config
        {
            let mut heartbeats = monitor.heartbeats.write().await;
            if let Some(state) = heartbeats.get_mut(&resource.id) {
                state.last_heartbeat = Utc::now() - chrono::Duration::seconds(100);
                state.config = config;
            }
        }

        // Check health should detect the stuck resource
        monitor.check_health().await;

        let health = monitor.get_health(&resource.id).await;
        assert!(matches!(health, HealthStatus::Stuck { .. }));

        let stuck = monitor.get_stuck_resources().await;
        assert_eq!(stuck.len(), 1);
    }
}
