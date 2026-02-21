# MesoClaw Resource Lifecycle Management

> Design document for comprehensive lifecycle control across agents, channels, tools, scheduler, and gateway resources.

**Status:** ✅ Implemented
**Created:** 2026-02-21
**Implemented:** 2026-02-21
**Scope:** Full Stack - all application resources
**Architecture:** Centralized Supervisor

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Lifecycle States](#lifecycle-states)
4. [Heartbeat System](#heartbeat-system)
5. [Transfer & Preserve Strategy](#transfer--preserve-strategy)
6. [Tiered Escalation Policy](#tiered-escalation-policy)
7. [Extension Points](#extension-points)
8. [Implementation Tasks](#implementation-tasks)
9. [Testing Strategy](#testing-strategy)

---

## Overview

### Problem Statement

Current resource management lacks:
- Unified lifecycle control (start → execution → stop → cleanup)
- Stuck resource detection (only timeouts exist)
- Automatic recovery with state preservation
- Tiered fallback strategies

### Design Decisions

| Aspect | Decision |
|--------|----------|
| **Scope** | Full Stack - all resources |
| **Detection** | Heartbeat + Timeout |
| **Recovery** | Transfer + Preserve |
| **Architecture** | **Centralized Supervisor** |
| **Failure Policy** | Tiered Escalation |
| **Extensibility** | Plugin-based resource handlers |

### Covered Resources

- **Agent Sessions** - LLM-based autonomous agents
- **Tool Executions** - Sandbox and direct tool runs
- **Channel Connections** - Telegram, Discord, Slack, Matrix
- **Scheduler Jobs** - Cron-based scheduled tasks
- **Subagents** - Multi-agent coordination
- **Gateway Handlers** - HTTP API request handlers
- **Memory Operations** - Background memory tasks

---

## Architecture

### High-Level Design (Centralized)

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          LifecycleSupervisor                                 │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                        Core Components                               │    │
│  │  ┌───────────────┐  ┌───────────────┐  ┌───────────────┐           │    │
│  │  │ HealthMonitor │  │ StateRegistry │  │RecoveryEngine │           │    │
│  │  │  (heartbeat)  │  │   (tracking)  │  │  (transfer)   │           │    │
│  │  └───────────────┘  └───────────────┘  └───────────────┘           │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      EscalationManager                               │    │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                 │    │
│  │  │ Tier1:Retry │  │Tier2:Fallback│  │ Tier3:User │                 │    │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                 │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │                      ResourcePluginRegistry                          │    │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐       │    │
│  │  │  Agent  │ │ Channel │ │  Tool   │ │Scheduler│ │  [ext]  │       │    │
│  │  │ Handler │ │ Handler │ │ Handler │ │ Handler │ │ Handler │       │    │
│  │  └─────────┘ └─────────┘ └─────────┘ └─────────┘ └─────────┘       │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
                          ┌────────────────────────┐
                          │    LifecycleEventBus   │
                          │   (extends EventBus)   │
                          └────────────────────────┘
```

### Key Design Principle: Plugin-Based Extensibility

All resource types are handled through a `ResourceHandler` trait. Adding new resource types requires only implementing this trait and registering with the plugin registry—no core supervisor changes needed.

```rust
/// Core trait for all resource handlers - implement this to add new resource types
#[async_trait]
pub trait ResourceHandler: Send + Sync {
    /// Unique identifier for this resource type
    fn resource_type(&self) -> ResourceType;

    /// Start a new resource instance
    async fn start(&self, config: ResourceConfig) -> Result<ResourceInstance, ResourceError>;

    /// Stop a resource gracefully
    async fn stop(&self, instance: &ResourceInstance) -> Result<(), ResourceError>;

    /// Force kill a stuck resource
    async fn kill(&self, instance: &ResourceInstance) -> Result<(), ResourceError>;

    /// Extract preservable state for transfer
    async fn extract_state(&self, instance: &ResourceInstance) -> Result<PreservedState, ResourceError>;

    /// Apply preserved state to new instance
    async fn apply_state(&self, instance: &mut ResourceInstance, state: PreservedState) -> Result<(), ResourceError>;

    /// Get fallback options for this resource type
    fn get_fallbacks(&self, current: &ResourceInstance) -> Vec<FallbackOption>;

    /// Health check beyond heartbeats
    async fn health_check(&self, instance: &ResourceInstance) -> Result<HealthStatus, ResourceError>;

    /// Cleanup resources after completion/failure
    async fn cleanup(&self, instance: &ResourceInstance) -> Result<(), ResourceError>;
}
```

### Components

#### LifecycleSupervisor (central controller)

**Location:** `src-tauri/src/lifecycle/supervisor.rs`

**Responsibilities:**
- Own and manage all resource instances
- Monitor health via heartbeats and explicit checks
- Orchestrate recovery and escalation
- Provide unified API for resource operations
- Manage plugin registry

```rust
pub struct LifecycleSupervisor {
    // Core components
    health_monitor: HealthMonitor,
    state_registry: StateRegistry,
    recovery_engine: RecoveryEngine,
    escalation_manager: EscalationManager,

    // Plugin system
    plugin_registry: PluginRegistry,

    // Configuration
    config: SupervisorConfig,

    // Event handling
    event_bus: LifecycleEventBus,
}

impl LifecycleSupervisor {
    /// Create new supervisor with configuration
    pub fn new(config: SupervisorConfig, event_bus: LifecycleEventBus) -> Self;

    /// Register a resource handler plugin
    pub fn register_handler(&mut self, handler: Box<dyn ResourceHandler>);

    /// Start tracking a new resource
    pub async fn spawn_resource(
        &self,
        resource_type: ResourceType,
        config: ResourceConfig,
    ) -> Result<ResourceId, ResourceError>;

    /// Get current state of a resource
    pub fn get_state(&self, resource_id: &ResourceId) -> Option<ResourceState>;

    /// Get all resources of a type
    pub fn get_resources_by_type(&self, resource_type: ResourceType) -> Vec<&ResourceInstance>;

    /// Manually trigger recovery for a resource
    pub async fn recover_resource(&self, resource_id: &ResourceId) -> Result<(), ResourceError>;

    /// Gracefully stop a resource
    pub async fn stop_resource(&self, resource_id: &ResourceId) -> Result<(), ResourceError>;

    /// Force kill a resource
    pub async fn kill_resource(&self, resource_id: &ResourceId) -> Result<(), ResourceError>;

    /// Start background health monitoring loop
    pub fn start_monitoring(self: Arc<Self>) -> JoinHandle<()>;
}
```

#### HealthMonitor (heartbeat tracking)

**Location:** `src-tauri/src/lifecycle/health_monitor.rs`

```rust
pub struct HealthMonitor {
    // Resource ID -> last heartbeat timestamp
    heartbeats: Arc<RwLock<HashMap<ResourceId, HeartbeatState>>>,

    // Resource ID -> health check task handle
    check_tasks: Arc<RwLock<HashMap<ResourceId, JoinHandle<()>>>>,

    config: HealthMonitorConfig,
}

pub struct HeartbeatState {
    last_heartbeat: DateTime<Utc>,
    missed_count: u32,
    status: HealthStatus,
}

pub enum HealthStatus {
    Healthy,
    Degraded { missed: u32 },
    Stuck { since: DateTime<Utc> },
    Unknown,
}

impl HealthMonitor {
    /// Record a heartbeat from a resource
    pub fn record_heartbeat(&self, resource_id: ResourceId);

    /// Get current health status
    pub fn get_health(&self, resource_id: &ResourceId) -> HealthStatus;

    /// Get all stuck resources
    pub fn get_stuck_resources(&self) -> Vec<ResourceId>;

    /// Start periodic health check task
    pub fn start_periodic_check(&self, resource_id: ResourceId, interval: Duration);
}
```

#### StateRegistry (resource tracking)

**Location:** `src-tauri/src/lifecycle/state_registry.rs`

```rust
pub struct StateRegistry {
    // All tracked resources
    resources: Arc<RwLock<HashMap<ResourceId, ResourceInstance>>>,

    // Resource type -> resource IDs index
    type_index: Arc<RwLock<HashMap<ResourceType, HashSet<ResourceId>>>>,

    // State transitions log (for debugging/auditing)
    transition_log: Arc<RwLock<Vec<StateTransition>>>,
}

pub struct ResourceInstance {
    pub id: ResourceId,
    pub resource_type: ResourceType,
    pub state: ResourceState,
    pub config: ResourceConfig,
    pub created_at: DateTime<Utc>,
    pub recovery_attempts: u32,
    pub current_escalation_tier: u8,
    pub handler: Arc<dyn ResourceHandler>,
}

pub struct StateTransition {
    pub resource_id: ResourceId,
    pub from_state: ResourceState,
    pub to_state: ResourceState,
    pub timestamp: DateTime<Utc>,
    pub reason: String,
}

impl StateRegistry {
    /// Register a new resource
    pub fn register(&self, instance: ResourceInstance);

    /// Unregister a resource
    pub fn unregister(&self, resource_id: &ResourceId);

    /// Update resource state (logs transition)
    pub fn update_state(&self, resource_id: &ResourceId, new_state: ResourceState, reason: String);

    /// Get resource instance
    pub fn get(&self, resource_id: &ResourceId) -> Option<ResourceInstance>;

    /// Get all resources of a type
    pub fn get_by_type(&self, resource_type: ResourceType) -> Vec<ResourceInstance>;

    /// Get transition history for a resource
    pub fn get_history(&self, resource_id: &ResourceId) -> Vec<StateTransition>;
}
```

#### RecoveryEngine (transfer orchestration)

**Location:** `src-tauri/src/lifecycle/recovery_engine.rs`

```rust
pub struct RecoveryEngine {
    state_registry: Arc<StateRegistry>,
    event_bus: LifecycleEventBus,
}

pub enum RecoveryAction {
    Retry { preserve_state: bool },
    Transfer { to_handler: ResourceType, preserve_state: bool },
    Escalate { tier: u8 },
    Abort { reason: String },
}

impl RecoveryEngine {
    /// Attempt recovery for a stuck resource
    pub async fn recover(&self, resource_id: &ResourceId) -> Result<RecoveryResult, RecoveryError>;

    /// Extract preservable state from a resource
    pub async fn extract_state(&self, resource_id: &ResourceId) -> Result<PreservedState, RecoveryError>;

    /// Apply preserved state to a new resource
    pub async fn apply_state(
        &self,
        target_id: &ResourceId,
        state: PreservedState,
    ) -> Result<(), RecoveryError>;

    /// Transfer resource to a new instance
    pub async fn transfer(
        &self,
        from_id: &ResourceId,
        to_type: Option<ResourceType>,
    ) -> Result<ResourceId, RecoveryError>;
}

pub enum RecoveryResult {
    Recovered { resource_id: ResourceId },
    Transferred { from_id: ResourceId, to_id: ResourceId },
    Escalated { tier: u8 },
    Failed { reason: String },
}
```

#### EscalationManager (tier management)

**Location:** `src-tauri/src/lifecycle/escalation_manager.rs`

```rust
pub struct EscalationManager {
    tier_policies: Vec<TierPolicy>,
    event_bus: LifecycleEventBus,
    user_intervention_queue: Arc<RwLock<Vec<UserInterventionRequest>>>,
}

pub struct TierPolicy {
    pub tier: u8,
    pub name: String,
    pub max_attempts: u32,
    pub cooldown: Duration,
    pub action: TierAction,
}

pub enum TierAction {
    Retry,
    FallbackProvider,
    FallbackChannel,
    UserIntervention { options: Vec<InterventionOption> },
}

pub struct UserInterventionRequest {
    pub resource_id: ResourceId,
    pub resource_type: ResourceType,
    pub failure_context: FailureContext,
    pub attempted_tiers: Vec<u8>,
    pub options: Vec<InterventionOption>,
    pub created_at: DateTime<Utc>,
}

impl EscalationManager {
    /// Determine next action for a failing resource
    pub fn determine_action(&self, resource: &ResourceInstance) -> RecoveryAction;

    /// Check if escalation is possible
    pub fn can_escalate(&self, resource: &ResourceInstance) -> bool;

    /// Escalate to next tier
    pub fn escalate(&self, resource_id: &ResourceId) -> Result<u8, EscalationError>;

    /// Get pending user intervention requests
    pub fn get_pending_interventions(&self) -> Vec<UserInterventionRequest>;

    /// Resolve a user intervention request
    pub fn resolve_intervention(
        &self,
        request_id: &str,
        resolution: InterventionResolution,
    ) -> Result<(), EscalationError>;
}
```

#### PluginRegistry (extensibility)

**Location:** `src-tauri/src/lifecycle/plugin_registry.rs`

```rust
pub struct PluginRegistry {
    handlers: HashMap<ResourceType, Box<dyn ResourceHandler>>,
    handler_order: Vec<ResourceType>, // For deterministic iteration
}

impl PluginRegistry {
    /// Register a resource handler
    pub fn register(&mut self, handler: Box<dyn ResourceHandler>) {
        let resource_type = handler.resource_type();
        self.handlers.insert(resource_type.clone(), handler);
        if !self.handler_order.contains(&resource_type) {
            self.handler_order.push(resource_type);
        }
    }

    /// Get handler for a resource type
    pub fn get(&self, resource_type: &ResourceType) -> Option<&Box<dyn ResourceHandler>> {
        self.handlers.get(resource_type)
    }

    /// Get all registered resource types
    pub fn registered_types(&self) -> &[ResourceType] {
        &self.handler_order
    }

    /// Check if a resource type is registered
    pub fn is_registered(&self, resource_type: &ResourceType) -> bool {
        self.handlers.contains_key(resource_type)
    }
}
```

---

## Lifecycle States

### Unified State Machine

```
                                    ┌─────────┐
                                    │  Idle   │
                                    └────┬────┘
                                         │ start()
                                         ▼
┌─────────────────────────────────────────────────────────────────────┐
│                              Running                                │
│  ┌───────────┐  progress   ┌──────────┐  execute   ┌───────────┐   │
│  │ Thinking  │ ──────────► │ Executing│ ────────► │  Waiting  │   │
│  └───────────┘  ◄────────── └──────────┘ ◄──────── └───────────┘   │
│        ▲                      (tools)        │                       │
│        │                                     │                       │
│        └─────────────────────────────────────┘                       │
│                         emit heartbeat                              │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
           ┌────────────────────┼────────────────────┐
           │                    │                    │
           ▼                    ▼                    ▼
    ┌─────────────┐      ┌─────────────┐      ┌─────────────┐
    │  Completed  │      │   Stuck     │      │   Failed    │
    │   (done)    │      │ (recoverable)│     │ (terminal)  │
    └─────────────┘      └──────┬──────┘      └─────────────┘
                                │
                    ┌───────────┼───────────┐
                    │           │           │
                    ▼           ▼           ▼
              ┌──────────┐ ┌──────────┐ ┌──────────┐
              │ Transfer │ │  Retry   │ │ Escalate │
              │ (new res)│ │(same res)│ │(fallback)│
              └──────────┘ └──────────┘ └──────────┘
```

### State Definitions

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceState {
    /// Resource available, not in use
    Idle,

    /// Active execution with substate tracking
    Running {
        substate: String,
        started_at: DateTime<Utc>,
        progress: Option<f32>, // 0.0 to 1.0
    },

    /// Heartbeats stopped, recovery possible
    Stuck {
        since: DateTime<Utc>,
        recovery_attempts: u32,
        last_known_progress: Option<f32>,
    },

    /// Currently being recovered
    Recovering {
        action: RecoveryActionType,
        started_at: DateTime<Utc>,
    },

    /// Successfully finished
    Completed {
        at: DateTime<Utc>,
        result: Option<serde_json::Value>,
    },

    /// Terminal failure after escalation exhausted
    Failed {
        at: DateTime<Utc>,
        error: String,
        terminal: bool,
        escalation_tier_reached: u8,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryActionType {
    Retry,
    Transfer,
    Fallback,
    UserIntervention,
}
```

### Type-Specific Running Substates

Each resource handler defines its own substates:

```rust
// Agent substates (in AgentHandler)
pub const AGENT_SUBSTATES: &[&str] = &[
    "thinking",           // Waiting for LLM response
    "executing_tool",     // Running a tool
    "waiting_approval",   // Waiting for user approval
    "waiting_input",      // Waiting for user input
    "compacting",         // Compacting history
];

// Tool substates (in ToolHandler)
pub const TOOL_SUBSTATES: &[&str] = &[
    "initializing",
    "executing",
    "waiting_result",
    "cleanup",
];

// Channel substates (in ChannelHandler)
pub const CHANNEL_SUBSTATES: &[&str] = &[
    "connecting",
    "connected",
    "reconnecting",
    "sending",
    "waiting_ack",
];

// Scheduler substates (in SchedulerHandler)
pub const SCHEDULER_SUBSTATES: &[&str] = &[
    "scheduled",
    "triggered",
    "running",
    "waiting_agent",
    "finishing",
];
```

---

## Heartbeat System

### Configuration Per Resource Type

| Resource | Interval | Stuck Threshold | Max Retries |
|----------|----------|-----------------|-------------|
| Agent | 5s | 3 missed (15s) | 2 |
| Tool | 10s | 2 missed (20s) | 3 |
| Channel | 30s | 2 missed (60s) | 3 |
| Scheduler Job | 60s | 2 missed (120s) | 2 |
| Subagent | 5s | 3 missed (15s) | 1 |
| Gateway Handler | 30s | 2 missed (60s) | 2 |

### Configurable Heartbeat Policy

```rust
pub struct HeartbeatConfig {
    /// Interval between expected heartbeats
    pub interval: Duration,

    /// Number of missed heartbeats before marking stuck
    pub stuck_threshold: u32,

    /// Maximum recovery attempts
    pub max_retries: u32,

    /// Cooldown between recovery attempts
    pub recovery_cooldown: Duration,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(10),
            stuck_threshold: 3,
            max_retries: 3,
            recovery_cooldown: Duration::from_secs(5),
        }
    }
}

// Per-resource-type defaults
impl HeartbeatConfig {
    pub fn for_agent() -> Self {
        Self {
            interval: Duration::from_secs(5),
            stuck_threshold: 3,
            max_retries: 2,
            recovery_cooldown: Duration::from_secs(2),
        }
    }

    pub fn for_channel() -> Self {
        Self {
            interval: Duration::from_secs(30),
            stuck_threshold: 2,
            max_retries: 3,
            recovery_cooldown: Duration::from_secs(10),
        }
    }

    // ... other resource types
}
```

---

## Transfer & Preserve Strategy

### Preserved State Structure

```rust
/// Generic preserved state - handlers define their specific state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreservedState {
    Agent(AgentPreservedState),
    Channel(ChannelPreservedState),
    Tool(ToolPreservedState),
    Scheduler(SchedulerPreservedState),
    Generic(serde_json::Value), // For custom/extension resources
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPreservedState {
    pub message_history: Vec<Message>,
    pub completed_tool_results: HashMap<String, ToolResult>,
    pub session_metadata: SessionMetadata,
    pub memory_context: Vec<MemoryEntry>,
    pub current_step: AgentStep,
    pub provider_config: ProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelPreservedState {
    pub outbound_queue: VecDeque<QueuedMessage>,
    pub config: ChannelConfig,
    pub last_sequence: u64,
    pub pending_acks: HashMap<u64, Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPreservedState {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub partial_result: Option<serde_json::Value>,
    pub attempt_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerPreservedState {
    pub job_id: String,
    pub job_config: JobConfig,
    pub execution_context: serde_json::Value,
    pub partial_results: Vec<serde_json::Value>,
}
```

### Transfer Flow

```
1. Detect Stuck Resource
   └─> HealthMonitor identifies missed heartbeats
   └─> Supervisor marks resource as Stuck

2. Initiate Recovery
   └─> RecoveryEngine.extract_state() from stuck resource
   └─> Supervisor determines recovery action via EscalationManager

3. Create New Instance (if transfer)
   └─> Supervisor.spawn_resource() with same type or fallback
   └─> RecoveryEngine.apply_state() to new instance

4. Cleanup Old Instance
   └─> Handler.kill() on stuck resource
   └─> Handler.cleanup() to release resources
   └─> StateRegistry.unregister()

5. Resume Execution
   └─> New instance continues from preserved state
   └─> Emit ResourceRecovered event
```

---

## Tiered Escalation Policy

### Escalation Tiers

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          ESCALATION TIERS                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  TIER 1: Retry Same Resource                                            │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ Agent: Restart session with same provider, inherit history       │   │
│  │ Tool:  Retry with exponential backoff (1s, 2s, 4s)              │   │
│  │ Channel: Reconnect with same config                              │   │
│  │ Scheduler: Rerun job                                             │   │
│  │ Max attempts: 2-3 per resource                                   │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                           │ Tier 1 exhausted                             │
│                           ▼                                              │
│  TIER 2: Fallback Resource Variant                                      │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ Agent: Switch to fallback LLM provider                           │   │
│  │        Priority: OpenAI → Anthropic → Google → Groq → Ollama    │   │
│  │ Tool:  Try alternative tool implementation (if available)        │   │
│  │ Channel: Use backup channel type                                 │   │
│  │ Max attempts: 1-2 per fallback                                   │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                           │ Tier 2 exhausted                             │
│                           ▼                                              │
│  TIER 3: User Intervention                                              │
│  ┌──────────────────────────────────────────────────────────────────┐   │
│  │ - Pause task execution                                           │   │
│  │ - Present context to user                                        │   │
│  │ - Options: Retry, Abort, Change Config, Manual Fix              │   │
│  │ - User response determines next action                           │   │
│  └──────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Escalation Configuration

```rust
pub struct EscalationConfig {
    pub tiers: Vec<TierConfig>,
}

pub struct TierConfig {
    pub tier: u8,
    pub name: String,
    pub max_attempts: u32,
    pub cooldown: Duration,
    pub resource_specific: HashMap<ResourceType, TierOverride>,
}

pub struct TierOverride {
    pub max_attempts: Option<u32>,
    pub custom_action: Option<String>,
}

impl Default for EscalationConfig {
    fn default() -> Self {
        Self {
            tiers: vec![
                TierConfig {
                    tier: 1,
                    name: "Retry".into(),
                    max_attempts: 3,
                    cooldown: Duration::from_secs(5),
                    resource_specific: HashMap::new(),
                },
                TierConfig {
                    tier: 2,
                    name: "Fallback".into(),
                    max_attempts: 2,
                    cooldown: Duration::from_secs(10),
                    resource_specific: HashMap::new(),
                },
                TierConfig {
                    tier: 3,
                    name: "UserIntervention".into(),
                    max_attempts: 1,
                    cooldown: Duration::from_secs(0),
                    resource_specific: HashMap::new(),
                },
            ],
        }
    }
}
```

---

## Extension Points

### 1. Custom Resource Handlers

To add a new resource type:

```rust
// 1. Define the resource type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Agent,
    Channel,
    Tool,
    SchedulerJob,
    Subagent,
    GatewayHandler,
    MemoryOperation,
    Custom(String), // Extension point for custom types
}

// 2. Implement ResourceHandler
pub struct MyCustomHandler {
    // Handler-specific state
}

#[async_trait]
impl ResourceHandler for MyCustomHandler {
    fn resource_type(&self) -> ResourceType {
        ResourceType::Custom("my_custom".into())
    }

    async fn start(&self, config: ResourceConfig) -> Result<ResourceInstance, ResourceError> {
        // Implementation
    }

    async fn stop(&self, instance: &ResourceInstance) -> Result<(), ResourceError> {
        // Implementation
    }

    // ... implement all trait methods
}

// 3. Register with supervisor
supervisor.register_handler(Box::new(MyCustomHandler::new()));
```

### 2. Custom Escalation Actions

```rust
pub trait EscalationStrategy: Send + Sync {
    fn name(&self) -> &str;
    fn applies_to(&self, resource_type: &ResourceType) -> bool;
    fn execute(&self, resource: &ResourceInstance) -> Pin<Box<dyn Future<Output = Result<(), EscalationError>> + Send>>;
}

// Register custom strategy
escalation_manager.register_strategy(Box::new(MyCustomStrategy::new()));
```

### 3. Custom State Preservation

```rust
pub trait StateSerializer: Send + Sync {
    fn serialize(&self, state: &PreservedState) -> Result<Vec<u8>, SerializationError>;
    fn deserialize(&self, data: &[u8]) -> Result<PreservedState, SerializationError>;
}

// For custom storage backends (e.g., distributed state)
supervisor.set_state_serializer(Box::new(CustomStateSerializer::new()));
```

### 4. Event Hooks

```rust
pub trait LifecycleHook: Send + Sync {
    fn on_resource_started(&self, resource: &ResourceInstance);
    fn on_resource_stuck(&self, resource: &ResourceInstance);
    fn on_resource_recovering(&self, resource: &ResourceInstance, action: &RecoveryAction);
    fn on_resource_recovered(&self, resource: &ResourceInstance);
    fn on_resource_failed(&self, resource: &ResourceInstance, error: &str);
}

// Register hooks
supervisor.register_hook(Box::new(MetricsHook::new()));
supervisor.register_hook(Box::new(LoggingHook::new()));
supervisor.register_hook(Box::new(CustomAlertingHook::new()));
```

### 5. Configuration Extensions

```rust
pub struct SupervisorConfig {
    // Core config
    pub health_check_interval: Duration,
    pub max_concurrent_resources: usize,

    // Extension points
    pub custom_settings: HashMap<String, serde_json::Value>,
}

// Access custom settings
let my_setting = supervisor.config().custom_settings.get("my_plugin");
```

---

## Implementation Tasks

### Phase 1: Core Infrastructure

#### Task 1.1: Create Lifecycle Module Structure
**Files:** `src-tauri/src/lifecycle/mod.rs`

```rust
pub mod supervisor;
pub mod health_monitor;
pub mod state_registry;
pub mod recovery_engine;
pub mod escalation_manager;
pub mod plugin_registry;
pub mod event_bus;
pub mod states;
pub mod handlers;

pub use supervisor::LifecycleSupervisor;
pub use states::{ResourceState, ResourceType, ResourceId};
pub use handlers::ResourceHandler;
```

**Acceptance Criteria:**
- [x] Module compiles without errors
- [x] All public types exported
- [x] Documentation comments added

---

#### Task 1.2: Define Core Types and States
**File:** `src-tauri/src/lifecycle/states.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Agent,
    Channel,
    Tool,
    SchedulerJob,
    Subagent,
    GatewayHandler,
    MemoryOperation,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(pub ResourceType, pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceState {
    Idle,
    Running { substate: String, started_at: DateTime<Utc>, progress: Option<f32> },
    Stuck { since: DateTime<Utc>, recovery_attempts: u32, last_known_progress: Option<f32> },
    Recovering { action: RecoveryActionType, started_at: DateTime<Utc> },
    Completed { at: DateTime<Utc>, result: Option<serde_json::Value> },
    Failed { at: DateTime<Utc>, error: String, terminal: bool, escalation_tier_reached: u8 },
}
```

**Acceptance Criteria:**
- [x] All types serializable for Tauri IPC
- [x] ResourceId unique and hashable
- [x] State transitions well-defined

---

#### Task 1.3: Implement ResourceHandler Trait
**File:** `src-tauri/src/lifecycle/handlers/mod.rs`

```rust
#[async_trait]
pub trait ResourceHandler: Send + Sync {
    fn resource_type(&self) -> ResourceType;

    async fn start(&self, config: ResourceConfig) -> Result<ResourceInstance, ResourceError>;
    async fn stop(&self, instance: &ResourceInstance) -> Result<(), ResourceError>;
    async fn kill(&self, instance: &ResourceInstance) -> Result<(), ResourceError>;
    async fn extract_state(&self, instance: &ResourceInstance) -> Result<PreservedState, ResourceError>;
    async fn apply_state(&self, instance: &mut ResourceInstance, state: PreservedState) -> Result<(), ResourceError>;
    fn get_fallbacks(&self, current: &ResourceInstance) -> Vec<FallbackOption>;
    async fn health_check(&self, instance: &ResourceInstance) -> Result<HealthStatus, ResourceError>;
    async fn cleanup(&self, instance: &ResourceInstance) -> Result<(), ResourceError>;
}
```

**Acceptance Criteria:**
- [x] Trait compiles and is object-safe
- [x] Async methods work correctly
- [x] Documentation for each method

---

#### Task 1.4: Implement PluginRegistry
**File:** `src-tauri/src/lifecycle/plugin_registry.rs`

**Acceptance Criteria:**
- [x] Handlers can be registered
- [x] Handler lookup by type works
- [x] Thread-safe access

---

#### Task 1.5: Implement HealthMonitor
**File:** `src-tauri/src/lifecycle/health_monitor.rs`

**Acceptance Criteria:**
- [x] Heartbeats recorded correctly
- [x] Stuck detection works
- [x] Periodic health checks run

---

#### Task 1.6: Implement StateRegistry
**File:** `src-tauri/src/lifecycle/state_registry.rs`

**Acceptance Criteria:**
- [x] Resources can be registered/unregistered
- [x] State transitions logged
- [x] Type-based queries work

---

#### Task 1.7: Implement RecoveryEngine
**File:** `src-tauri/src/lifecycle/recovery_engine.rs`

**Acceptance Criteria:**
- [x] State extraction/apply works
- [x] Transfer flow complete
- [x] Events emitted correctly

---

#### Task 1.8: Implement EscalationManager
**File:** `src-tauri/src/lifecycle/escalation_manager.rs`

**Acceptance Criteria:**
- [x] Tier transitions work
- [x] User intervention requests created
- [x] Resolution handled

---

#### Task 1.9: Implement LifecycleSupervisor
**File:** `src-tauri/src/lifecycle/supervisor.rs`

**Acceptance Criteria:**
- [x] All components integrated
- [x] spawn_resource works
- [x] Background monitoring starts

---

### Phase 2: Resource Handlers

#### Task 2.1: Implement AgentHandler
**File:** `src-tauri/src/lifecycle/handlers/agent.rs`

**Acceptance Criteria:**
- [x] Integrates with existing AgentLoop
- [x] State preservation works
- [x] Fallback providers supported

---

#### Task 2.2: Implement ChannelHandler
**File:** `src-tauri/src/lifecycle/handlers/channel.rs`

**Acceptance Criteria:**
- [x] Integrates with ChannelManager
- [x] Queue preservation works
- [x] Reconnection logic works

---

#### Task 2.3: Implement ToolHandler
**File:** `src-tauri/src/lifecycle/handlers/tool.rs`

**Acceptance Criteria:**
- [x] Integrates with ToolExecutor
- [x] Retry with backoff works
- [x] Sandbox cleanup handled

---

#### Task 2.4: Implement SchedulerHandler
**File:** `src-tauri/src/lifecycle/handlers/scheduler.rs`

**Acceptance Criteria:**
- [x] Integrates with scheduler
- [x] Job state preserved
- [x] Rerun on recovery

---

### Phase 3: Event Bus Extension

#### Task 3.1: Implement LifecycleEventBus
**File:** `src-tauri/src/lifecycle/event_bus.rs`

```rust
pub enum LifecycleEvent {
    ResourceStarted { resource_id: ResourceId, resource_type: ResourceType },
    ResourceHeartbeat { resource_id: ResourceId, timestamp: DateTime<Utc> },
    ResourceProgress { resource_id: ResourceId, progress: f32, substate: String },
    ResourceStuck { resource_id: ResourceId, last_heartbeat: DateTime<Utc> },
    ResourceRecovering { resource_id: ResourceId, action: RecoveryActionType },
    ResourceTransferring { from_id: ResourceId, to_id: ResourceId },
    ResourceRecovered { resource_id: ResourceId, tier: u8 },
    ResourceFailed { resource_id: ResourceId, error: String, terminal: bool },
    UserInterventionNeeded { request: UserInterventionRequest },
}
```

**Acceptance Criteria:**
- [x] Extends existing EventBus
- [x] All events published correctly
- [x] Frontend can subscribe

---

### Phase 4: Tauri Commands

#### Task 4.1: Create Lifecycle Commands
**File:** `src-tauri/src/commands/lifecycle.rs`

```rust
#[tauri::command]
pub async fn get_resource_status_command(resource_id: String, supervisor: State<'_, LifecycleSupervisor>) -> Result<ResourceStatus, String>;

#[tauri::command]
pub async fn get_all_resources_command(supervisor: State<'_, LifecycleSupervisor>) -> Result<Vec<ResourceStatus>, String>;

#[tauri::command]
pub async fn get_stuck_resources_command(supervisor: State<'_, LifecycleSupervisor>) -> Result<Vec<ResourceId>, String>;

#[tauri::command]
pub async fn retry_resource_command(resource_id: String, supervisor: State<'_, LifecycleSupervisor>) -> Result<(), String>;

#[tauri::command]
pub async fn abort_resource_command(resource_id: String, supervisor: State<'_, LifecycleSupervisor>) -> Result<(), String>;

#[tauri::command]
pub async fn get_pending_interventions_command(supervisor: State<'_, LifecycleSupervisor>) -> Result<Vec<UserInterventionRequest>, String>;

#[tauri::command]
pub async fn resolve_intervention_command(request_id: String, resolution: String, supervisor: State<'_, LifecycleSupervisor>) -> Result<(), String>;
```

**Acceptance Criteria:**
- [x] All commands callable from frontend
- [x] Error handling consistent
- [x] Commands registered in lib.rs

---

### Phase 5: Frontend Integration

#### Task 5.1: Create LifecycleStore
**File:** `src/stores/lifecycleStore.ts`

**Acceptance Criteria:**
- [x] Tracks all resources
- [x] Updates on events
- [x] Intervention queue managed

---

#### Task 5.2: Create LifecycleStatus Component
**File:** `src/components/lifecycle/LifecycleStatus.tsx`

**Acceptance Criteria:**
- [x] Shows resource health
- [x] Highlights stuck resources
- [x] Shows recovery progress

---

#### Task 5.3: Create UserInterventionDialog
**File:** `src/components/lifecycle/UserInterventionDialog.tsx`

**Acceptance Criteria:**
- [x] Shows on Tier 3 escalation
- [x] All options available
- [x] Actions trigger commands

---

### Phase 6: App Integration

#### Task 6.1: Initialize Supervisor in lib.rs
**File:** `src-tauri/src/lib.rs`

```rust
// Create supervisor
let supervisor = Arc::new(LifecycleSupervisor::new(
    SupervisorConfig::default(),
    lifecycle_event_bus,
));

// Register handlers
supervisor.register_handler(Box::new(AgentHandler::new(agent_service)));
supervisor.register_handler(Box::new(ChannelHandler::new(channel_manager)));
supervisor.register_handler(Box::new(ToolHandler::new(tool_registry)));
supervisor.register_handler(Box::new(SchedulerHandler::new(scheduler)));

// Start monitoring
let supervisor_clone = supervisor.clone();
tokio::spawn(async move {
    supervisor_clone.start_monitoring().await;
});

// Register with app
app.manage(supervisor);
```

**Acceptance Criteria:**
- [x] App starts without errors
- [x] Supervisor accessible
- [x] Monitoring runs

---

## CLI Integration

The lifecycle management system works identically from both the Tauri desktop app and the CLI binary. The same `LifecycleSupervisor`, handlers, and recovery logic apply - only the user intervention interface differs.

### Architecture (Dual Interface)

```
┌─────────────────────────────────────────────────────────────────┐
│                    LifecycleSupervisor                           │
│  [HealthMonitor] [StateRegistry] [RecoveryEngine]               │
│  [EscalationManager] [PluginRegistry]                           │
└─────────────────────────────────────────────────────────────────┘
            │                              │
            ▼                              ▼
┌───────────────────────┐      ┌───────────────────────┐
│   Tauri Desktop App   │      │     CLI Binary        │
│                       │      │                       │
│  ┌─────────────────┐  │      │  ┌─────────────────┐  │
│  │ Tauri Commands  │  │      │  │ CLI Commands    │  │
│  │ (invoke IPC)    │  │      │  │ (direct calls)  │  │
│  └─────────────────┘  │      │  └─────────────────┘  │
│           │           │      │           │           │
│           ▼           │      │           ▼           │
│  ┌─────────────────┐  │      │  ┌─────────────────┐  │
│  │ React Frontend  │  │      │  │ Interactive     │  │
│  │ - Status UI     │  │      │  │ Console UI      │  │
│  │ - Dialogs       │  │      │  │ - Prompts       │  │
│  └─────────────────┘  │      │  │ - Status lines  │  │
│                       │      │  └─────────────────┘  │
└───────────────────────┘      └───────────────────────┘
```

### User Intervention: CLI Mode

When Tier 3 escalation is reached in CLI mode, instead of a dialog, the system uses interactive console prompts:

```rust
pub enum InterventionInterface {
    Tauri,  // Desktop app - uses Tauri events → React dialogs
    Cli,    // CLI mode - uses console prompts
}

impl EscalationManager {
    pub async fn handle_user_intervention(
        &self,
        request: UserInterventionRequest,
        interface: InterventionInterface,
    ) -> Result<InterventionResolution, EscalationError> {
        match interface {
            InterventionInterface::Tauri => {
                // Emit event to frontend, wait for response
                self.event_bus.publish_lifecycle(
                    LifecycleEvent::UserInterventionNeeded { request }
                );
                self.wait_for_resolution(&request.id).await
            }
            InterventionInterface::Cli => {
                // Interactive console prompt
                self.cli_prompt_for_intervention(request).await
            }
        }
    }

    async fn cli_prompt_for_intervention(
        &self,
        request: UserInterventionRequest,
    ) -> Result<InterventionResolution, EscalationError> {
        println!("\n{}", "=".repeat(60));
        println!("⚠️  USER INTERVENTION REQUIRED");
        println!("{}", "=".repeat(60));
        println!("\nResource: {} ({:?})", request.resource_id, request.resource_type);
        println!("\nFailure Context:");
        println!("  - Error: {}", request.failure_context.error);
        println!("  - Attempts: {:?}", request.attempted_tiers);
        println!("\nOptions:");

        for (i, option) in request.options.iter().enumerate() {
            println!("  {}. {}", i + 1, option.label);
        }

        print!("\nSelect option (1-{}): ", request.options.len());

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        let choice: usize = input.trim().parse()
            .map_err(|_| EscalationError::InvalidInput)?;

        let selected = request.options.get(choice - 1)
            .ok_or(EscalationError::InvalidChoice)?;

        Ok(InterventionResolution {
            request_id: request.id.clone(),
            selected_option: selected.id.clone(),
            additional_data: None,
        })
    }
}
```

### CLI Commands for Lifecycle Management

Add these commands to the CLI (`src-tauri/src/cli.rs`):

```rust
// In cli.rs

/// List all resources and their status
#[derive(Parser)]
pub struct ResourceListCmd {
    /// Filter by resource type
    #[arg(short, long)]
    pub resource_type: Option<String>,

    /// Show only stuck resources
    #[arg(short, long)]
    pub stuck: bool,
}

/// Get detailed status of a specific resource
#[derive(Parser)]
pub struct ResourceStatusCmd {
    /// Resource ID
    pub resource_id: String,
}

/// Manually trigger recovery for a stuck resource
#[derive(Parser)]
pub struct ResourceRecoverCmd {
    /// Resource ID
    pub resource_id: String,

    /// Force escalation to a specific tier (1-3)
    #[arg(short, long)]
    pub tier: Option<u8>,
}

/// Stop a running resource
#[derive(Parser)]
pub struct ResourceStopCmd {
    /// Resource ID
    pub resource_id: String,

    /// Force kill if graceful stop fails
    #[arg(short, long)]
    pub force: bool,
}

/// Start lifecycle monitoring daemon
#[derive(Parser)]
pub struct LifecycleMonitorCmd {
    /// Run in foreground (don't daemonize)
    #[arg(short, long)]
    pub foreground: bool,
}

// Command handlers
impl CliApp {
    pub async fn handle_resource_list(&self, cmd: ResourceListCmd) -> Result<()> {
        let supervisor = self.supervisor.as_ref().unwrap();

        let resources = if cmd.stuck {
            supervisor.get_stuck_resources()
        } else {
            supervisor.get_all_resources()
        };

        let filtered = if let Some(rt) = cmd.resource_type {
            resources.into_iter()
                .filter(|r| format!("{:?}", r.resource_type).to_lowercase().contains(&rt.to_lowercase()))
                .collect()
        } else {
            resources
        };

        // Print table
        println!("{:<40} {:<15} {:<20} {:<10}", "ID", "TYPE", "STATE", "TIER");
        println!("{}", "-".repeat(85));
        for r in filtered {
            println!("{:<40} {:<15} {:<20} {:<10}",
                r.id.1,
                format!("{:?}", r.id.0),
                format!("{:?}", r.state),
                r.current_escalation_tier
            );
        }

        Ok(())
    }

    pub async fn handle_resource_status(&self, cmd: ResourceStatusCmd) -> Result<()> {
        let supervisor = self.supervisor.as_ref().unwrap();
        let resource_id = ResourceId::parse(&cmd.resource_id)?;

        let instance = supervisor.get_resource(&resource_id)
            .ok_or_else(|| anyhow!("Resource not found: {}", cmd.resource_id))?;

        println!("Resource: {}", resource_id);
        println!("State: {:?}", instance.state);
        println!("Created: {}", instance.created_at);
        println!("Recovery Attempts: {}", instance.recovery_attempts);
        println!("Escalation Tier: {}", instance.current_escalation_tier);

        if let Some(history) = supervisor.get_transition_history(&resource_id) {
            println!("\nTransition History:");
            for t in history {
                println!("  {} -> {} at {} ({})",
                    format!("{:?}", t.from_state),
                    format!("{:?}", t.to_state),
                    t.timestamp,
                    t.reason
                );
            }
        }

        Ok(())
    }

    pub async fn handle_resource_recover(&self, cmd: ResourceRecoverCmd) -> Result<()> {
        let supervisor = self.supervisor.as_ref().unwrap();
        let resource_id = ResourceId::parse(&cmd.resource_id)?;

        println!("Initiating recovery for {}...", resource_id);

        if let Some(tier) = cmd.tier {
            supervisor.force_escalate(&resource_id, tier)?;
        }

        supervisor.recover_resource(&resource_id).await?;

        println!("✓ Recovery initiated successfully");
        Ok(())
    }

    pub async fn handle_resource_stop(&self, cmd: ResourceStopCmd) -> Result<()> {
        let supervisor = self.supervisor.as_ref().unwrap();
        let resource_id = ResourceId::parse(&cmd.resource_id)?;

        println!("Stopping {}...", resource_id);

        if cmd.force {
            supervisor.kill_resource(&resource_id).await?;
            println!("✓ Resource killed");
        } else {
            supervisor.stop_resource(&resource_id).await?;
            println!("✓ Resource stopped gracefully");
        }

        Ok(())
    }

    pub async fn handle_lifecycle_monitor(&self, cmd: LifecycleMonitorCmd) -> Result<()> {
        let supervisor = self.supervisor.as_ref().unwrap().clone();

        println!("Starting lifecycle monitoring...");
        println!("Press Ctrl+C to stop\n");

        // Subscribe to lifecycle events
        let mut receiver = supervisor.subscribe_events();

        loop {
            tokio::select! {
                event = receiver.recv() => {
                    match event {
                        Ok(LifecycleEvent::ResourceHeartbeat { resource_id, timestamp }) => {
                            println!("[{timestamp}] 💓 {} heartbeat", resource_id);
                        }
                        Ok(LifecycleEvent::ResourceStuck { resource_id, last_heartbeat }) => {
                            println!("[{}] ⚠️  {} STUCK (last heartbeat: {})",
                                chrono::Local::now().format("%H:%M:%S"),
                                resource_id,
                                last_heartbeat
                            );
                        }
                        Ok(LifecycleEvent::ResourceRecovering { resource_id, action }) => {
                            println!("[{}] 🔄 {} recovering: {:?}",
                                chrono::Local::now().format("%H:%M:%S"),
                                resource_id,
                                action
                            );
                        }
                        Ok(LifecycleEvent::ResourceRecovered { resource_id, tier }) => {
                            println!("[{}] ✅ {} recovered (tier {})",
                                chrono::Local::now().format("%H:%M:%S"),
                                resource_id,
                                tier
                            );
                        }
                        Ok(LifecycleEvent::ResourceFailed { resource_id, error, terminal }) => {
                            println!("[{}] ❌ {} FAILED: {} {}",
                                chrono::Local::now().format("%H:%M:%S"),
                                resource_id,
                                error,
                                if terminal { "(TERMINAL)" } else { "" }
                            );
                        }
                        Ok(LifecycleEvent::UserInterventionNeeded { request }) => {
                            // Interactive prompt for intervention
                            self.handle_cli_intervention(request).await?;
                        }
                        _ => {}
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    println!("\nStopping monitor...");
                    break;
                }
            }
        }

        Ok(())
    }
}
```

### CLI Integration in Binary Entry

```rust
// src-tauri/src/main.rs (desktop) vs src-tauri/src/cli.rs (CLI)

// Desktop app initialization (lib.rs)
pub fn setup_desktop(app: &tauri::App) -> Result<Box<dyn std::any::Any>, Box<dyn std::error::Error>> {
    let supervisor = LifecycleSupervisor::new(
        SupervisorConfig::default(),
        lifecycle_event_bus.clone(),
    );

    // Register handlers
    register_handlers(&supervisor);

    // Use Tauri interface for interventions
    supervisor.set_intervention_interface(InterventionInterface::Tauri);

    // Start monitoring
    let supervisor_clone = supervisor.clone();
    tokio::spawn(async move { supervisor_clone.start_monitoring().await });

    app.manage(supervisor);
    Ok(Box::new(()))
}

// CLI initialization (cli.rs)
pub async fn setup_cli() -> Result<()> {
    let event_bus = LifecycleEventBus::new();
    let supervisor = LifecycleSupervisor::new(
        SupervisorConfig::default(),
        event_bus,
    );

    // Register handlers (same as desktop)
    register_handlers(&supervisor);

    // Use CLI interface for interventions
    supervisor.set_intervention_interface(InterventionInterface::Cli);

    // Store for command access
    CLI_SUPERVISOR.set(supervisor);

    Ok(())
}

fn register_handlers(supervisor: &LifecycleSupervisor) {
    supervisor.register_handler(Box::new(AgentHandler::new()));
    supervisor.register_handler(Box::new(ChannelHandler::new()));
    supervisor.register_handler(Box::new(ToolHandler::new()));
    supervisor.register_handler(Box::new(SchedulerHandler::new()));
}
```

### CLI Usage Examples

```bash
# List all resources
tauriclaw resource list
tauriclaw resource list --stuck
tauriclaw resource list --resource-type agent

# Get resource details
tauriclaw resource status agent:main:dm:tauri:user

# Recover a stuck resource
tauriclaw resource recover agent:main:dm:tauri:user
tauriclaw resource recover agent:main:dm:tauri:user --tier 2

# Stop a resource
tauriclaw resource stop channel:telegram:bot123
tauriclaw resource stop channel:telegram:bot123 --force

# Start interactive monitoring
tauriclaw lifecycle monitor
```

---

## Testing Strategy

### Unit Tests

| Component | Test Focus |
|-----------|------------|
| HealthMonitor | Heartbeat recording, stuck detection timing |
| StateRegistry | Registration, state transitions, history |
| RecoveryEngine | State extraction/apply, transfer flow |
| EscalationManager | Tier transitions, user interventions |
| PluginRegistry | Registration, lookup |

### Integration Tests

| Scenario | Test Focus |
|----------|------------|
| Agent stuck → recovery | Full recovery flow with AgentHandler |
| Channel disconnect → reconnect | Queue preservation |
| Tool failure → escalation | All 3 tiers |
| Multiple stuck resources | Supervisor handles concurrent recovery |

### Extension Tests

| Scenario | Test Focus |
|----------|------------|
| Custom handler registration | Plugin system works |
| Custom escalation strategy | Strategy integration |
| Custom hooks | Hook callbacks fire |

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Stuck detection latency | < threshold + 1s |
| Recovery success rate (Tier 1) | > 90% |
| State preservation accuracy | 100% |
| User intervention rate | < 5% of failures |
| Extension load time | < 100ms |
| System overhead | < 2% CPU, < 50MB memory |

---

## Future Enhancements

1. **Distributed Supervisor** - Support multi-instance deployments
2. **Adaptive Thresholds** - Learn expected durations automatically
3. **Predictive Recovery** - ML-based prediction before stuck
4. **Resource Pooling** - Pre-warm instances for faster recovery
5. **Metrics Dashboard** - Real-time visualization
6. **External Plugins** - Load handlers from dynamic libraries
