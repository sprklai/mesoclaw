use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// All events that flow through the application event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppEvent {
    AgentToolStart {
        tool_name: String,
        args: serde_json::Value,
    },
    AgentToolResult {
        tool_name: String,
        result: String,
        success: bool,
    },
    /// Emitted immediately after a session is registered, before the agent
    /// runs.  Clients should capture `session_id` here to enable cancellation.
    AgentStarted {
        session_id: String,
    },
    AgentComplete {
        session_id: String,
        message: String,
    },
    ApprovalNeeded {
        action_id: String,
        tool_name: String,
        description: String,
        risk_level: String,
    },
    ApprovalResponse {
        action_id: String,
        approved: bool,
    },
    HeartbeatTick {
        timestamp: String,
    },
    /// Emitted when a heartbeat agent run returns meaningful content (not `HEARTBEAT_OK`).
    /// Consumers (notification service, channel bridge) should surface this to the user.
    HeartbeatAlert {
        content: String,
    },
    CronFired {
        job_id: String,
        schedule: String,
    },
    ChannelMessage {
        channel: String,
        from: String,
        content: String,
        #[serde(default)]
        metadata: std::collections::HashMap<String, String>,
    },
    MemoryStored {
        key: String,
        summary: String,
    },
    MemoryRecalled {
        query: String,
        count: usize,
    },
    SystemReady,
    SystemError {
        message: String,
    },
    ProviderHealthChange {
        provider_id: String,
        healthy: bool,
    },
}

/// Selects which event variants a subscriber is interested in.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    AgentToolStart,
    AgentToolResult,
    AgentStarted,
    AgentComplete,
    ApprovalNeeded,
    ApprovalResponse,
    HeartbeatTick,
    HeartbeatAlert,
    CronFired,
    ChannelMessage,
    MemoryStored,
    MemoryRecalled,
    SystemReady,
    SystemError,
    ProviderHealthChange,
    /// Matches every variant.
    All,
}

impl EventType {
    /// Returns true if this filter matches `event`.
    pub fn matches(&self, event: &AppEvent) -> bool {
        match self {
            Self::All => true,
            Self::AgentToolStart => matches!(event, AppEvent::AgentToolStart { .. }),
            Self::AgentToolResult => matches!(event, AppEvent::AgentToolResult { .. }),
            Self::AgentStarted => matches!(event, AppEvent::AgentStarted { .. }),
            Self::AgentComplete => matches!(event, AppEvent::AgentComplete { .. }),
            Self::ApprovalNeeded => matches!(event, AppEvent::ApprovalNeeded { .. }),
            Self::ApprovalResponse => matches!(event, AppEvent::ApprovalResponse { .. }),
            Self::HeartbeatTick => matches!(event, AppEvent::HeartbeatTick { .. }),
            Self::HeartbeatAlert => matches!(event, AppEvent::HeartbeatAlert { .. }),
            Self::CronFired => matches!(event, AppEvent::CronFired { .. }),
            Self::ChannelMessage => matches!(event, AppEvent::ChannelMessage { .. }),
            Self::MemoryStored => matches!(event, AppEvent::MemoryStored { .. }),
            Self::MemoryRecalled => matches!(event, AppEvent::MemoryRecalled { .. }),
            Self::SystemReady => matches!(event, AppEvent::SystemReady),
            Self::SystemError => matches!(event, AppEvent::SystemError { .. }),
            Self::ProviderHealthChange => matches!(event, AppEvent::ProviderHealthChange { .. }),
        }
    }
}

/// A set of event types used for filtering subscriptions.
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub event_types: Vec<EventType>,
}

impl EventFilter {
    pub fn new(event_types: Vec<EventType>) -> Self {
        Self { event_types }
    }

    pub fn matches(&self, event: &AppEvent) -> bool {
        self.event_types.iter().any(|t| t.matches(event))
    }
}

/// Central pub/sub bus for application events.
///
/// All returned `Receiver`s receive every published event; callers are
/// responsible for filtering if they subscribed via [`subscribe_filtered`].
pub trait EventBus: Send + Sync {
    /// Publish an event to all active subscribers.
    fn publish(&self, event: AppEvent) -> Result<(), String>;

    /// Subscribe to all events.
    fn subscribe(&self) -> broadcast::Receiver<AppEvent>;

    /// Subscribe to events, pre-scoped to the given filter.
    ///
    /// The returned receiver still carries all events; the filter is provided
    /// as documentation / future optimisation surface.  Consumers should call
    /// [`EventFilter::matches`] to discard unwanted events.
    fn subscribe_filtered(&self, filter: EventFilter) -> broadcast::Receiver<AppEvent>;
}
