//! Resource Lifecycle Management for MesoClaw.
//!
//! This module provides comprehensive lifecycle management for all application
//! resources including agents, channels, tools, scheduler jobs, and gateway handlers.
//!
//! # Architecture
//!
//! The lifecycle system uses a centralized supervisor pattern with plugin-based
//! extensibility:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    LifecycleSupervisor                           │
//! │  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐               │
//! │  │HealthMonitor│ │StateRegistry│ │RecoveryEngine│               │
//! │  └─────────────┘ └─────────────┘ └─────────────┘               │
//! │  ┌─────────────────────────────────────────────────────────┐   │
//! │  │              PluginRegistry (extensible)                 │   │
//! │  │  [AgentHandler] [ChannelHandler] [ToolHandler] [...]    │   │
//! │  └─────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Key Features
//!
//! - **Heartbeat + Timeout Detection**: Resources emit heartbeats; missed heartbeats trigger stuck detection
//! - **Transfer & Preserve Recovery**: State is preserved and transferred to new instances
//! - **Tiered Escalation**: Retry → Fallback → User Intervention
//! - **Plugin-based Extensibility**: Add new resource types by implementing `ResourceHandler`
//!
//! # Usage
//!
//! ```rust,ignore
//! use mesoclaw::lifecycle::{LifecycleSupervisor, SupervisorConfig, ResourceType, ResourceConfig};
//!
//! // Create supervisor
//! let supervisor = LifecycleSupervisor::new(SupervisorConfig::default());
//!
//! // Register handlers
//! supervisor.register_handler(Box::new(AgentHandler::new()));
//! supervisor.register_handler(Box::new(ChannelHandler::new()));
//!
//! // Spawn a resource
//! let resource_id = supervisor.spawn_resource(
//!     ResourceType::Agent,
//!     ResourceConfig {
//!         provider_id: Some("openai".into()),
//!         model_id: Some("gpt-4".into()),
//!         ..Default::default()
//!     },
//! ).await?;
//!
//! // Start monitoring
//! let supervisor = Arc::new(supervisor);
//! supervisor.clone().start_monitoring().await;
//! ```

pub mod escalation_manager;
pub mod event_bus;
pub mod events;
pub mod handlers;
pub mod health_monitor;
pub mod manager;
pub mod plugin_registry;
pub mod recovery_engine;
pub mod state_registry;
pub mod states;
pub mod storage;
pub mod supervisor;

// Re-exports for convenience
pub use escalation_manager::{
    EscalationConfig, EscalationError, EscalationManager, TierAction, TierPolicy,
};
pub use event_bus::{LifecycleEvent, LifecycleEventBus};
pub use events::{
    StateChangePayload, emit_lifecycle_event, emit_session_completed, emit_session_created,
    emit_session_failed, emit_state_changed, events as lifecycle_events,
};
pub use handlers::ResourceHandler;
pub use health_monitor::{HealthMonitor, HealthMonitorEvent, HealthMonitorStats};
pub use manager::{LifecycleManager, SharedLifecycleManager};
pub use plugin_registry::PluginRegistry;
pub use recovery_engine::{RecoveryAction, RecoveryEngine, RecoveryResult};
pub use state_registry::{StateRegistry, StateRegistryStats};
pub use states::{
    FailureContext, FallbackOption, HealthStatus, HeartbeatConfig, InterventionOption,
    InterventionResolution, PreservedState, RecoveryActionType, ResourceConfig, ResourceError,
    ResourceId, ResourceInstance, ResourceState, ResourceType, StateTransition, SupervisorConfig,
    UserInterventionRequest,
};
pub use storage::LifecycleStorage;
pub use supervisor::{InterventionInterface, LifecycleSupervisor, SupervisorStats};

/// Default heartbeat interval in seconds.
pub const DEFAULT_HEARTBEAT_INTERVAL_SECS: u64 = 10;

/// Default stuck threshold (number of missed heartbeats).
pub const DEFAULT_STUCK_THRESHOLD: u32 = 3;
