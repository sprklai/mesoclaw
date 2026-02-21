//! Escalation management for tiered recovery.
//!
//! The escalation manager implements the tiered escalation policy:
//! - Tier 1: Retry same resource
//! - Tier 2: Fallback to alternative
//! - Tier 3: User intervention

use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};

use super::recovery_engine::RecoveryAction;
use super::state_registry::StateRegistry;
use super::states::{
    FailureContext, InterventionOption, InterventionResolution, ResourceInstance, ResourceState,
};

/// Maximum escalation tier.
pub const MAX_TIER: u8 = 3;

/// Policy for an escalation tier.
#[derive(Debug, Clone)]
pub struct TierPolicy {
    /// Tier number (1-3)
    pub tier: u8,
    /// Human-readable name
    pub name: String,
    /// Maximum attempts at this tier
    pub max_attempts: u32,
    /// Cooldown between attempts
    pub cooldown: Duration,
    /// Action to take at this tier
    pub action: TierAction,
}

impl Default for TierPolicy {
    fn default() -> Self {
        Self {
            tier: 1,
            name: "Retry".to_string(),
            max_attempts: 3,
            cooldown: Duration::from_secs(5),
            action: TierAction::Retry,
        }
    }
}

/// Actions that can be taken at an escalation tier.
#[derive(Debug, Clone)]
pub enum TierAction {
    /// Retry the same resource
    Retry,
    /// Fall back to an alternative provider/implementation
    FallbackProvider,
    /// Fall back to an alternative channel
    FallbackChannel,
    /// Request user intervention
    UserIntervention { options: Vec<InterventionOption> },
}

/// Configuration for the escalation manager.
#[derive(Debug, Clone)]
pub struct EscalationConfig {
    /// Tier policies in order
    pub tiers: Vec<TierPolicy>,
}

impl Default for EscalationConfig {
    fn default() -> Self {
        Self {
            tiers: vec![
                TierPolicy {
                    tier: 1,
                    name: "Retry".to_string(),
                    max_attempts: 3,
                    cooldown: Duration::from_secs(5),
                    action: TierAction::Retry,
                },
                TierPolicy {
                    tier: 2,
                    name: "Fallback".to_string(),
                    max_attempts: 2,
                    cooldown: Duration::from_secs(10),
                    action: TierAction::FallbackProvider,
                },
                TierPolicy {
                    tier: 3,
                    name: "UserIntervention".to_string(),
                    max_attempts: 1,
                    cooldown: Duration::from_secs(0),
                    action: TierAction::UserIntervention {
                        options: vec![
                            InterventionOption {
                                id: "retry".to_string(),
                                label: "Retry".to_string(),
                                description: "Try the operation again".to_string(),
                                destructive: false,
                            },
                            InterventionOption {
                                id: "abort".to_string(),
                                label: "Abort".to_string(),
                                description: "Cancel the operation".to_string(),
                                destructive: true,
                            },
                            InterventionOption {
                                id: "change_config".to_string(),
                                label: "Change Configuration".to_string(),
                                description: "Modify settings and retry".to_string(),
                                destructive: false,
                            },
                        ],
                    },
                },
            ],
        }
    }
}

/// Events emitted by the escalation manager.
#[derive(Debug, Clone)]
pub enum EscalationEvent {
    /// Resource escalated to a new tier
    Escalated {
        resource_id: String,
        from_tier: u8,
        to_tier: u8,
    },
    /// Recovery attempt started
    RecoveryAttempted {
        resource_id: String,
        tier: u8,
        attempt: u32,
    },
    /// User intervention requested
    InterventionRequested {
        request: super::states::UserInterventionRequest,
    },
    /// User intervention resolved
    InterventionResolved {
        request_id: String,
        resolution: InterventionResolution,
    },
}

/// Tracks the state of a resource's escalation.
#[derive(Debug, Clone)]
struct EscalationState {
    /// Current tier
    current_tier: u8,
    /// Attempts at current tier
    tier_attempts: u32,
    /// Last attempt time
    last_attempt: Option<DateTime<Utc>>,
    /// Tiers already attempted
    attempted_tiers: Vec<u8>,
}

impl Default for EscalationState {
    fn default() -> Self {
        Self {
            current_tier: 0,
            tier_attempts: 0,
            last_attempt: None,
            attempted_tiers: Vec::new(),
        }
    }
}

/// Manages escalation of failing resources.
pub struct EscalationManager {
    /// Configuration
    config: EscalationConfig,
    /// State registry for resource lookup (for future use)
    #[allow(dead_code)]
    state_registry: Arc<StateRegistry>,
    /// Escalation state by resource ID
    escalation_states: RwLock<HashMap<String, EscalationState>>,
    /// Pending user intervention requests
    intervention_queue: RwLock<HashMap<String, super::states::UserInterventionRequest>>,
    /// Event sender
    event_sender: broadcast::Sender<EscalationEvent>,
}

impl EscalationManager {
    /// Create a new escalation manager.
    pub fn new(config: EscalationConfig, state_registry: Arc<StateRegistry>) -> Self {
        let (sender, _) = broadcast::channel(256);
        Self {
            config,
            state_registry,
            escalation_states: RwLock::new(HashMap::new()),
            intervention_queue: RwLock::new(HashMap::new()),
            event_sender: sender,
        }
    }

    /// Subscribe to escalation events.
    pub fn subscribe(&self) -> broadcast::Receiver<EscalationEvent> {
        self.event_sender.subscribe()
    }

    /// Determine the next recovery action for a resource.
    pub async fn determine_action(&self, resource: &ResourceInstance) -> RecoveryAction {
        let states = self.escalation_states.read().await;
        let state = states
            .get(&resource.id.to_string())
            .cloned()
            .unwrap_or_default();

        drop(states);

        // Get the policy for the current tier
        let tier = if state.current_tier == 0 {
            1
        } else {
            state.current_tier
        };
        let policy = self.get_tier_policy(tier);

        // Check if we've exceeded attempts at this tier
        if state.tier_attempts >= policy.max_attempts {
            // Need to escalate
            if tier < MAX_TIER {
                return RecoveryAction::Escalate { tier: tier + 1 };
            } else {
                // At max tier, need user intervention
                return RecoveryAction::Abort {
                    reason: "Exhausted all escalation tiers".to_string(),
                };
            }
        }

        // Determine action based on tier policy
        match &policy.action {
            TierAction::Retry => RecoveryAction::Retry {
                preserve_state: true,
            },
            TierAction::FallbackProvider | TierAction::FallbackChannel => {
                RecoveryAction::Transfer {
                    to_type: None, // Handler will determine fallback
                    preserve_state: true,
                }
            }
            TierAction::UserIntervention { .. } => RecoveryAction::Abort {
                reason: "User intervention required".to_string(),
            },
        }
    }

    /// Check if escalation is possible for a resource.
    pub async fn can_escalate(&self, resource: &ResourceInstance) -> bool {
        let states = self.escalation_states.read().await;
        let state = states
            .get(&resource.id.to_string())
            .cloned()
            .unwrap_or_default();

        state.current_tier < MAX_TIER
    }

    /// Escalate a resource to the next tier.
    pub async fn escalate(&self, resource_id: &str) -> Result<u8, EscalationError> {
        let mut states = self.escalation_states.write().await;
        let state = states.entry(resource_id.to_string()).or_default();

        let old_tier = state.current_tier;
        let new_tier = (old_tier + 1).min(MAX_TIER);

        if new_tier == old_tier && old_tier > 0 {
            return Err(EscalationError::AlreadyAtMaxTier);
        }

        // Record the tier transition
        if !state.attempted_tiers.contains(&old_tier) && old_tier > 0 {
            state.attempted_tiers.push(old_tier);
        }

        state.current_tier = new_tier;
        state.tier_attempts = 0;

        let _ = self.event_sender.send(EscalationEvent::Escalated {
            resource_id: resource_id.to_string(),
            from_tier: old_tier,
            to_tier: new_tier,
        });

        log::warn!(
            "EscalationManager: {} escalated from tier {} to {}",
            resource_id,
            old_tier,
            new_tier
        );

        Ok(new_tier)
    }

    /// Record a recovery attempt.
    pub async fn record_attempt(&self, resource_id: &str) {
        let mut states = self.escalation_states.write().await;
        let state = states.entry(resource_id.to_string()).or_default();

        // Ensure we're at tier 1 if not yet escalated
        if state.current_tier == 0 {
            state.current_tier = 1;
        }

        state.tier_attempts += 1;
        state.last_attempt = Some(Utc::now());

        let tier = state.current_tier;
        let attempt = state.tier_attempts;

        let _ = self.event_sender.send(EscalationEvent::RecoveryAttempted {
            resource_id: resource_id.to_string(),
            tier,
            attempt,
        });

        log::debug!(
            "EscalationManager: {} recovery attempt {} at tier {}",
            resource_id,
            attempt,
            tier
        );
    }

    /// Check if cooldown is active for a resource.
    pub async fn is_cooldown_active(&self, resource_id: &str) -> bool {
        let states = self.escalation_states.read().await;

        if let Some(state) = states.get(resource_id) {
            if let Some(last) = state.last_attempt {
                let policy = self.get_tier_policy(state.current_tier);
                let elapsed = (Utc::now() - last).to_std().unwrap_or(Duration::ZERO);
                return elapsed < policy.cooldown;
            }
        }

        false
    }

    /// Create a user intervention request for a resource.
    pub async fn create_intervention_request(
        &self,
        resource: &ResourceInstance,
    ) -> super::states::UserInterventionRequest {
        let states = self.escalation_states.read().await;
        let state = states
            .get(&resource.id.to_string())
            .cloned()
            .unwrap_or_default();

        let policy = self.get_tier_policy(state.current_tier);
        let options = match &policy.action {
            TierAction::UserIntervention { options } => options.clone(),
            _ => vec![
                InterventionOption {
                    id: "retry".to_string(),
                    label: "Retry".to_string(),
                    description: "Try the operation again".to_string(),
                    destructive: false,
                },
                InterventionOption {
                    id: "abort".to_string(),
                    label: "Abort".to_string(),
                    description: "Cancel the operation".to_string(),
                    destructive: true,
                },
            ],
        };

        let failure_context = match &resource.state {
            ResourceState::Stuck {
                since,
                recovery_attempts,
                ..
            } => FailureContext {
                error: "Resource stopped responding".to_string(),
                recovery_attempts: *recovery_attempts,
                running_duration_secs: (Utc::now() - *since).num_seconds() as u64,
                last_state: format!("{:?}", resource.state),
                failed_at: *since,
            },
            ResourceState::Failed { at, error, .. } => FailureContext {
                error: error.clone(),
                recovery_attempts: resource.recovery_attempts,
                running_duration_secs: 0,
                last_state: "failed".to_string(),
                failed_at: *at,
            },
            _ => FailureContext {
                error: "Unknown failure".to_string(),
                recovery_attempts: 0,
                running_duration_secs: 0,
                last_state: format!("{:?}", resource.state),
                failed_at: Utc::now(),
            },
        };

        let request = super::states::UserInterventionRequest {
            id: format!("intervention:{}:{}", resource.id, Utc::now().timestamp()),
            resource_id: resource.id.to_string(),
            resource_type: resource.resource_type.clone(),
            failure_context,
            attempted_tiers: state.attempted_tiers.clone(),
            options,
            created_at: Utc::now(),
        };

        // Store in queue
        {
            let mut queue = self.intervention_queue.write().await;
            queue.insert(request.id.clone(), request.clone());
        }

        let _ = self
            .event_sender
            .send(EscalationEvent::InterventionRequested {
                request: request.clone(),
            });

        log::warn!(
            "EscalationManager: user intervention requested for {} (request_id={})",
            resource.id,
            request.id
        );

        request
    }

    /// Get all pending intervention requests.
    pub async fn get_pending_interventions(&self) -> Vec<super::states::UserInterventionRequest> {
        let queue = self.intervention_queue.read().await;
        queue.values().cloned().collect()
    }

    /// Resolve an intervention request.
    pub async fn resolve_intervention(
        &self,
        request_id: &str,
        resolution: InterventionResolution,
    ) -> Result<(), EscalationError> {
        let mut queue = self.intervention_queue.write().await;

        if let Some(_request) = queue.remove(request_id) {
            let _ = self
                .event_sender
                .send(EscalationEvent::InterventionResolved {
                    request_id: request_id.to_string(),
                    resolution: resolution.clone(),
                });

            log::info!(
                "EscalationManager: intervention {} resolved with option {}",
                request_id,
                resolution.selected_option
            );

            Ok(())
        } else {
            Err(EscalationError::InterventionNotFound(
                request_id.to_string(),
            ))
        }
    }

    /// Reset escalation state for a resource (e.g., after successful recovery).
    pub async fn reset(&self, resource_id: &str) {
        let mut states = self.escalation_states.write().await;
        states.remove(resource_id);

        log::debug!(
            "EscalationManager: reset escalation state for {}",
            resource_id
        );
    }

    /// Get the policy for a tier.
    fn get_tier_policy(&self, tier: u8) -> TierPolicy {
        self.config
            .tiers
            .iter()
            .find(|p| p.tier == tier)
            .cloned()
            .unwrap_or_default()
    }

    /// Get the current tier for a resource.
    pub async fn get_current_tier(&self, resource_id: &str) -> u8 {
        let states = self.escalation_states.read().await;
        states.get(resource_id).map(|s| s.current_tier).unwrap_or(0)
    }
}

/// Errors from the escalation manager.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EscalationError {
    #[error("Already at maximum escalation tier")]
    AlreadyAtMaxTier,

    #[error("Intervention request not found: {0}")]
    InterventionNotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::states::{ResourceConfig, ResourceId, ResourceType};

    #[tokio::test]
    async fn test_escalation_flow() {
        let state_registry = Arc::new(StateRegistry::new());
        let manager = EscalationManager::new(EscalationConfig::default(), state_registry);

        let resource = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );

        // Should start at tier 0
        assert_eq!(manager.get_current_tier(&resource.id.to_string()).await, 0);

        // Escalate to tier 1
        let tier = manager.escalate(&resource.id.to_string()).await.unwrap();
        assert_eq!(tier, 1);

        // Escalate to tier 2
        let tier = manager.escalate(&resource.id.to_string()).await.unwrap();
        assert_eq!(tier, 2);

        // Escalate to tier 3
        let tier = manager.escalate(&resource.id.to_string()).await.unwrap();
        assert_eq!(tier, 3);

        // Should not be able to escalate past max
        let result = manager.escalate(&resource.id.to_string()).await;
        assert!(matches!(result, Err(EscalationError::AlreadyAtMaxTier)));
    }

    #[tokio::test]
    async fn test_determine_action() {
        let state_registry = Arc::new(StateRegistry::new());
        let manager = EscalationManager::new(EscalationConfig::default(), state_registry);

        let resource = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );

        // Tier 1 should give retry
        manager.escalate(&resource.id.to_string()).await.unwrap();
        let action = manager.determine_action(&resource).await;
        assert!(matches!(action, RecoveryAction::Retry { .. }));
    }

    #[tokio::test]
    async fn test_intervention_request() {
        let state_registry = Arc::new(StateRegistry::new());
        let manager = EscalationManager::new(EscalationConfig::default(), state_registry);

        let mut resource = ResourceInstance::new(
            ResourceId::new(ResourceType::Agent, "test:1"),
            ResourceConfig::default(),
        );
        resource.state = ResourceState::Stuck {
            since: Utc::now(),
            recovery_attempts: 3,
            last_known_progress: None,
        };

        let request = manager.create_intervention_request(&resource).await;
        assert!(!request.options.is_empty());

        let pending = manager.get_pending_interventions().await;
        assert_eq!(pending.len(), 1);

        // Resolve the request
        let resolution = InterventionResolution {
            request_id: request.id.clone(),
            selected_option: "retry".to_string(),
            additional_data: None,
        };

        manager
            .resolve_intervention(&request.id, resolution)
            .await
            .unwrap();

        let pending = manager.get_pending_interventions().await;
        assert!(pending.is_empty());
    }
}
