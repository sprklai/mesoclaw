//! Core types and states for the resource lifecycle management system.
//!
//! This module defines the fundamental types used throughout the lifecycle
//! management system: resource identifiers, states, and preserved state structures.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::collections::HashMap;

// ─── ResourceType ────────────────────────────────────────────────────────────

/// Identifies the type of a managed resource.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    /// LLM-based autonomous agent session
    Agent,
    /// External messaging channel (Telegram, Discord, Slack, Matrix)
    Channel,
    /// Tool execution (sandbox or direct)
    Tool,
    /// Scheduled background job
    SchedulerJob,
    /// Subagent within a multi-agent workflow
    Subagent,
    /// HTTP API gateway request handler
    GatewayHandler,
    /// Background memory operation
    MemoryOperation,
    /// Extension point for custom resource types
    Custom(String),
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResourceType::Agent => write!(f, "agent"),
            ResourceType::Channel => write!(f, "channel"),
            ResourceType::Tool => write!(f, "tool"),
            ResourceType::SchedulerJob => write!(f, "scheduler_job"),
            ResourceType::Subagent => write!(f, "subagent"),
            ResourceType::GatewayHandler => write!(f, "gateway_handler"),
            ResourceType::MemoryOperation => write!(f, "memory_operation"),
            ResourceType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

// ─── ResourceId ──────────────────────────────────────────────────────────────

/// Unique identifier for a managed resource.
///
/// Format: `{resource_type}:{namespace}:{instance_id}`
/// Example: `agent:main:dm:tauri:user123`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(pub ResourceType, pub String);

impl ResourceId {
    /// Create a new resource ID.
    pub fn new(resource_type: ResourceType, instance_id: impl Into<String>) -> Self {
        Self(resource_type, instance_id.into())
    }

    /// Parse a resource ID from its string representation.
    pub fn parse(s: &str) -> Result<Self, String> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(format!("Invalid resource ID format: {}", s));
        }

        let resource_type = match parts[0] {
            "agent" => ResourceType::Agent,
            "channel" => ResourceType::Channel,
            "tool" => ResourceType::Tool,
            "scheduler_job" => ResourceType::SchedulerJob,
            "subagent" => ResourceType::Subagent,
            "gateway_handler" => ResourceType::GatewayHandler,
            "memory_operation" => ResourceType::MemoryOperation,
            other if other.starts_with("custom:") => {
                ResourceType::Custom(other.strip_prefix("custom:").unwrap_or(other).to_string())
            }
            other => return Err(format!("Unknown resource type: {}", other)),
        };

        Ok(Self(resource_type, parts[1].to_string()))
    }

    /// Get the resource type.
    pub fn resource_type(&self) -> &ResourceType {
        &self.0
    }

    /// Get the instance ID.
    pub fn instance_id(&self) -> &str {
        &self.1
    }
}

impl std::fmt::Display for ResourceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.0, self.1)
    }
}

// ─── ResourceState ───────────────────────────────────────────────────────────

/// The lifecycle state of a managed resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ResourceState {
    /// Resource available, not in use
    Idle,

    /// Active execution with substate tracking
    Running {
        /// Current activity substate (e.g., "thinking", "executing_tool")
        substate: String,
        /// When execution started
        started_at: DateTime<Utc>,
        /// Progress percentage (0.0 to 1.0)
        #[serde(default)]
        progress: Option<f32>,
    },

    /// Heartbeats stopped, recovery possible
    Stuck {
        /// When the resource was detected as stuck
        since: DateTime<Utc>,
        /// Number of recovery attempts made
        recovery_attempts: u32,
        /// Last known progress before stuck
        #[serde(default)]
        last_known_progress: Option<f32>,
    },

    /// Currently being recovered
    Recovering {
        /// The recovery action being taken
        action: RecoveryActionType,
        /// When recovery started
        started_at: DateTime<Utc>,
    },

    /// Successfully finished
    Completed {
        /// When execution completed
        at: DateTime<Utc>,
        /// Optional result data
        #[serde(default)]
        result: Option<serde_json::Value>,
    },

    /// Terminal failure after escalation exhausted
    Failed {
        /// When failure occurred
        at: DateTime<Utc>,
        /// Error message
        error: String,
        /// Whether this is a terminal (non-recoverable) failure
        terminal: bool,
        /// Highest escalation tier reached
        escalation_tier_reached: u8,
    },
}

impl ResourceState {
    /// Check if the resource is in an active (non-terminal) state.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            ResourceState::Running { .. }
                | ResourceState::Recovering { .. }
                | ResourceState::Stuck { .. }
        )
    }

    /// Check if the resource is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ResourceState::Completed { .. } | ResourceState::Failed { terminal: true, .. }
        )
    }

    /// Get the substate if running.
    pub fn substate(&self) -> Option<&str> {
        match self {
            ResourceState::Running { substate, .. } => Some(substate),
            _ => None,
        }
    }
}

impl Default for ResourceState {
    fn default() -> Self {
        Self::Idle
    }
}

// ─── RecoveryActionType ──────────────────────────────────────────────────────

/// Types of recovery actions that can be taken for stuck resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryActionType {
    /// Retry the same resource with same configuration
    Retry,
    /// Transfer to a new resource instance
    Transfer,
    /// Fall back to an alternative resource/implementation
    Fallback,
    /// Wait for user to manually intervene
    UserIntervention,
}

// ─── HealthStatus ────────────────────────────────────────────────────────────

/// Health status of a monitored resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// Resource is healthy and responding
    Healthy,
    /// Resource is degraded (missed heartbeats but not stuck yet)
    Degraded {
        /// Number of missed heartbeats
        missed: u32,
    },
    /// Resource is stuck (exceeded missed heartbeat threshold)
    Stuck {
        /// When the resource became stuck
        since: DateTime<Utc>,
    },
    /// Health status unknown (not monitored)
    Unknown,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

// ─── PreservedState ──────────────────────────────────────────────────────────

/// Generic preserved state for resource transfer.
///
/// Handlers define their specific state structures. This enum provides
/// type-safe dispatch to the appropriate handler-specific state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum PreservedState {
    /// Agent session state (message history, context, etc.)
    Agent(AgentPreservedState),
    /// Channel connection state (queues, config, etc.)
    Channel(ChannelPreservedState),
    /// Tool execution state (arguments, partial results)
    Tool(ToolPreservedState),
    /// Scheduler job state (config, partial results)
    Scheduler(SchedulerPreservedState),
    /// Generic state for custom resource types
    Generic(serde_json::Value),
}

/// Preserved state for an agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentPreservedState {
    /// Conversation message history
    pub message_history: Vec<PreservedMessage>,
    /// Completed tool results by tool call ID
    #[serde(default)]
    pub completed_tool_results: HashMap<String, ToolResult>,
    /// Session metadata (provider, model, etc.)
    pub session_metadata: SessionMetadata,
    /// Memory context entries
    #[serde(default)]
    pub memory_context: Vec<MemoryEntry>,
    /// Current agent step for resumption
    #[serde(default)]
    pub current_step: Option<String>,
}

/// Simplified message structure for preservation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreservedMessage {
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<serde_json::Value>,
}

/// Session metadata for agent preservation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMetadata {
    /// Provider ID (e.g., "openai", "anthropic")
    pub provider_id: String,
    /// Model ID
    pub model_id: String,
    /// System prompt used
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Temperature setting
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Maximum tokens
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

/// Tool execution result for preservation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub output: String,
    pub success: bool,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// Memory entry for preservation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryEntry {
    pub key: String,
    pub content: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Preserved state for a channel connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelPreservedState {
    /// Outbound message queue
    #[serde(default)]
    pub outbound_queue: VecDeque<QueuedMessage>,
    /// Channel configuration
    pub config: HashMap<String, String>,
    /// Last sequence number
    #[serde(default)]
    pub last_sequence: u64,
    /// Messages pending acknowledgment
    #[serde(default)]
    pub pending_acks: HashMap<u64, QueuedMessage>,
}

/// Queued message for channel preservation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedMessage {
    pub id: u64,
    pub content: String,
    pub recipient: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// When the message was queued
    pub queued_at: DateTime<Utc>,
}

/// Preserved state for a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolPreservedState {
    /// Tool name
    pub tool_name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
    /// Partial result if execution was interrupted
    #[serde(default)]
    pub partial_result: Option<serde_json::Value>,
    /// Attempt number for retry tracking
    #[serde(default)]
    pub attempt_number: u32,
}

/// Preserved state for a scheduler job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerPreservedState {
    /// Job ID
    pub job_id: String,
    /// Job configuration
    pub job_config: serde_json::Value,
    /// Execution context (session state, etc.)
    #[serde(default)]
    pub execution_context: Option<serde_json::Value>,
    /// Partial results from interrupted execution
    #[serde(default)]
    pub partial_results: Vec<serde_json::Value>,
}

// ─── ResourceConfig ──────────────────────────────────────────────────────────

/// Configuration for starting a new resource.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceConfig {
    /// Provider/model to use (for agents)
    #[serde(default)]
    pub provider_id: Option<String>,
    /// Model ID (for agents)
    #[serde(default)]
    pub model_id: Option<String>,
    /// System prompt (for agents)
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Channel type (for channels)
    #[serde(default)]
    pub channel_type: Option<String>,
    /// Channel configuration
    #[serde(default)]
    pub channel_config: HashMap<String, String>,
    /// Tool name (for tools)
    #[serde(default)]
    pub tool_name: Option<String>,
    /// Tool arguments
    #[serde(default)]
    pub tool_args: Option<serde_json::Value>,
    /// Job configuration (for scheduler)
    #[serde(default)]
    pub job_config: Option<serde_json::Value>,
    /// Custom settings for extension types
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

// ─── FallbackOption ──────────────────────────────────────────────────────────

/// Fallback option for resource recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackOption {
    /// Unique identifier for this fallback
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Description of what this fallback does
    pub description: String,
    /// Configuration for the fallback resource
    pub config: ResourceConfig,
}

// ─── StateTransition ─────────────────────────────────────────────────────────

/// Record of a state transition for auditing/debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateTransition {
    /// Resource that transitioned
    pub resource_id: String,
    /// Previous state (serialized for storage)
    pub from_state: String,
    /// New state (serialized for storage)
    pub to_state: String,
    /// When the transition occurred
    pub timestamp: DateTime<Utc>,
    /// Reason for the transition
    pub reason: String,
}

// ─── UserInterventionRequest ─────────────────────────────────────────────────

/// Request for user intervention when automated recovery fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInterventionRequest {
    /// Unique request ID
    pub id: String,
    /// Resource that needs intervention
    pub resource_id: String,
    /// Type of resource
    pub resource_type: ResourceType,
    /// Context about the failure
    pub failure_context: FailureContext,
    /// Escalation tiers already attempted
    pub attempted_tiers: Vec<u8>,
    /// Options available to the user
    pub options: Vec<InterventionOption>,
    /// When the request was created
    pub created_at: DateTime<Utc>,
}

/// Context about a failure for user intervention.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureContext {
    /// Error message
    pub error: String,
    /// Number of recovery attempts
    pub recovery_attempts: u32,
    /// How long the resource was running before failure
    pub running_duration_secs: u64,
    /// Last known state
    pub last_state: String,
    /// Timestamp of failure
    pub failed_at: DateTime<Utc>,
}

/// Option for user to choose during intervention.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterventionOption {
    /// Unique option ID
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Description of the option
    pub description: String,
    /// Whether this option is destructive
    pub destructive: bool,
}

/// Resolution of a user intervention request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterventionResolution {
    /// ID of the intervention request
    pub request_id: String,
    /// ID of the selected option
    pub selected_option: String,
    /// Additional data provided by user
    #[serde(default)]
    pub additional_data: Option<serde_json::Value>,
}

// ─── HeartbeatConfig ─────────────────────────────────────────────────────────

/// Configuration for heartbeat monitoring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartbeatConfig {
    /// Interval between expected heartbeats (seconds)
    pub interval_secs: u64,
    /// Number of missed heartbeats before marking stuck
    pub stuck_threshold: u32,
    /// Maximum recovery attempts
    pub max_retries: u32,
    /// Cooldown between recovery attempts (seconds)
    pub recovery_cooldown_secs: u64,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval_secs: 10,
            stuck_threshold: 3,
            max_retries: 3,
            recovery_cooldown_secs: 5,
        }
    }
}

impl HeartbeatConfig {
    /// Configuration for agent resources.
    pub fn for_agent() -> Self {
        Self {
            interval_secs: 5,
            stuck_threshold: 3,
            max_retries: 2,
            recovery_cooldown_secs: 2,
        }
    }

    /// Configuration for channel resources.
    pub fn for_channel() -> Self {
        Self {
            interval_secs: 30,
            stuck_threshold: 2,
            max_retries: 3,
            recovery_cooldown_secs: 10,
        }
    }

    /// Configuration for tool resources.
    pub fn for_tool() -> Self {
        Self {
            interval_secs: 10,
            stuck_threshold: 2,
            max_retries: 3,
            recovery_cooldown_secs: 5,
        }
    }

    /// Configuration for scheduler job resources.
    pub fn for_scheduler_job() -> Self {
        Self {
            interval_secs: 60,
            stuck_threshold: 2,
            max_retries: 2,
            recovery_cooldown_secs: 30,
        }
    }

    /// Configuration for subagent resources.
    pub fn for_subagent() -> Self {
        Self {
            interval_secs: 5,
            stuck_threshold: 3,
            max_retries: 1,
            recovery_cooldown_secs: 2,
        }
    }

    /// Configuration for gateway handler resources.
    pub fn for_gateway_handler() -> Self {
        Self {
            interval_secs: 30,
            stuck_threshold: 2,
            max_retries: 2,
            recovery_cooldown_secs: 10,
        }
    }

    /// Get configuration for a resource type.
    pub fn for_resource_type(resource_type: &ResourceType) -> Self {
        match resource_type {
            ResourceType::Agent => Self::for_agent(),
            ResourceType::Channel => Self::for_channel(),
            ResourceType::Tool => Self::for_tool(),
            ResourceType::SchedulerJob => Self::for_scheduler_job(),
            ResourceType::Subagent => Self::for_subagent(),
            ResourceType::GatewayHandler => Self::for_gateway_handler(),
            ResourceType::MemoryOperation => Self::default(),
            ResourceType::Custom(_) => Self::default(),
        }
    }
}

// ─── SupervisorConfig ────────────────────────────────────────────────────────

/// Configuration for the lifecycle supervisor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupervisorConfig {
    /// Interval for health check sweeps (seconds)
    pub health_check_interval_secs: u64,
    /// Maximum concurrent resources
    pub max_concurrent_resources: usize,
    /// Default heartbeat config
    pub default_heartbeat: HeartbeatConfig,
    /// Per-resource-type heartbeat config overrides
    #[serde(default)]
    pub heartbeat_overrides: HashMap<String, HeartbeatConfig>,
    /// Custom settings for extensions
    #[serde(default)]
    pub custom_settings: HashMap<String, serde_json::Value>,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            health_check_interval_secs: 5,
            max_concurrent_resources: 100,
            default_heartbeat: HeartbeatConfig::default(),
            heartbeat_overrides: HashMap::new(),
            custom_settings: HashMap::new(),
        }
    }
}

// ─── ResourceError ───────────────────────────────────────────────────────────

/// Error type for resource operations.
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize)]
pub enum ResourceError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid state transition: {0}")]
    InvalidTransition(String),

    #[error("Operation timeout: {0}")]
    Timeout(String),

    #[error("Handler not registered for type: {0}")]
    HandlerNotRegistered(String),

    #[error("Failed to start resource: {0}")]
    StartFailed(String),

    #[error("Failed to stop resource: {0}")]
    StopFailed(String),

    #[error("Failed to kill resource: {0}")]
    KillFailed(String),

    #[error("Failed to extract state: {0}")]
    StateExtractionFailed(String),

    #[error("Failed to apply state: {0}")]
    StateApplyFailed(String),

    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

// ─── ResourceInstance ────────────────────────────────────────────────────────

/// A tracked resource instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceInstance {
    /// Unique identifier
    pub id: ResourceId,
    /// Resource type
    pub resource_type: ResourceType,
    /// Current state
    pub state: ResourceState,
    /// Configuration used to start
    pub config: ResourceConfig,
    /// When the resource was created
    pub created_at: DateTime<Utc>,
    /// Number of recovery attempts
    pub recovery_attempts: u32,
    /// Current escalation tier (1-3)
    pub current_escalation_tier: u8,
    /// Heartbeat configuration for this resource
    pub heartbeat_config: HeartbeatConfig,
}

impl ResourceInstance {
    /// Create a new resource instance.
    pub fn new(id: ResourceId, config: ResourceConfig) -> Self {
        let resource_type = id.0.clone();
        let heartbeat_config = HeartbeatConfig::for_resource_type(&resource_type);

        Self {
            id,
            resource_type,
            state: ResourceState::Idle,
            config,
            created_at: Utc::now(),
            recovery_attempts: 0,
            current_escalation_tier: 0,
            heartbeat_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_id_parse() {
        let id = ResourceId::parse("agent:main:session123").unwrap();
        assert_eq!(id.0, ResourceType::Agent);
        assert_eq!(id.1, "main:session123");
    }

    #[test]
    fn test_resource_id_display() {
        let id = ResourceId::new(ResourceType::Agent, "session123");
        assert_eq!(format!("{}", id), "agent:session123");
    }

    #[test]
    fn test_resource_state_is_active() {
        assert!(ResourceState::Running {
            substate: "thinking".into(),
            started_at: Utc::now(),
            progress: None
        }.is_active());

        assert!(!ResourceState::Idle.is_active());
        assert!(!ResourceState::Completed { at: Utc::now(), result: None }.is_active());
    }

    #[test]
    fn test_heartbeat_config_defaults() {
        let agent_config = HeartbeatConfig::for_agent();
        assert_eq!(agent_config.interval_secs, 5);
        assert_eq!(agent_config.stuck_threshold, 3);

        let channel_config = HeartbeatConfig::for_channel();
        assert_eq!(channel_config.interval_secs, 30);
    }
}
