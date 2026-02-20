//! Agent configuration and workspace management module.
//!
//! This module provides agent lifecycle management, workspace isolation,
//! and configuration persistence for multi-agent orchestration.

pub mod config;
pub mod orchestrator;
pub mod spawner;
pub mod workspace;

pub use config::{
    AgentConfig, AgentConfigManager, AgentConfigUpdate, ConcurrencyConfig, ModelConfig,
};
pub use orchestrator::{
    AgentOrchestrator, ParallelExecutionConfig as OrchestratorParallelConfig, ParallelResult,
};
pub use spawner::{
    ActiveSubagentInfo, SpawnHandle, SubagentResult, SubagentSpawner, SubagentTask, TokenUsage,
};
pub use workspace::{AgentWorkspace, WorkspaceManager};

use serde::{Deserialize, Serialize};

/// Unique identifier for an agent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(pub String);

impl AgentId {
    /// Create a new agent ID.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the agent ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for AgentId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for AgentId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Agent execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// Agent is initialized and ready.
    Initialized,
    /// Agent is actively processing a request.
    Running,
    /// Agent completed its task successfully.
    Completed,
    /// Agent was cancelled by user or system.
    Aborted,
    /// Agent encountered an unrecoverable error.
    Error,
}

impl Default for AgentStatus {
    fn default() -> Self {
        Self::Initialized
    }
}

/// Thinking level for agent execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Low,
    Medium,
    High,
    Xhigh,
}

impl Default for ThinkingLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl std::fmt::Display for ThinkingLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThinkingLevel::Low => write!(f, "low"),
            ThinkingLevel::Medium => write!(f, "medium"),
            ThinkingLevel::High => write!(f, "high"),
            ThinkingLevel::Xhigh => write!(f, "xhigh"),
        }
    }
}

/// Verbose level for agent output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VerboseLevel {
    Off,
    On,
    Full,
}

impl Default for VerboseLevel {
    fn default() -> Self {
        Self::Off
    }
}

impl std::fmt::Display for VerboseLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerboseLevel::Off => write!(f, "off"),
            VerboseLevel::On => write!(f, "on"),
            VerboseLevel::Full => write!(f, "full"),
        }
    }
}

/// Error type for agent operations.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Agent already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid agent configuration: {0}")]
    InvalidConfig(String),

    #[error("Workspace error: {0}")]
    Workspace(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

// Re-export orchestrator types with shorter names for convenience
pub use orchestrator::{ExecutionMode, FailureStrategy};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_id_display() {
        let id = AgentId::new("test-agent");
        assert_eq!(format!("{}", id), "test-agent");
    }

    #[test]
    fn test_agent_status_default() {
        let status = AgentStatus::default();
        assert_eq!(status, AgentStatus::Initialized);
    }

    #[test]
    fn test_thinking_level_display() {
        assert_eq!(format!("{}", ThinkingLevel::Low), "low");
        assert_eq!(format!("{}", ThinkingLevel::Medium), "medium");
        assert_eq!(format!("{}", ThinkingLevel::High), "high");
        assert_eq!(format!("{}", ThinkingLevel::Xhigh), "xhigh");
    }

    #[test]
    fn test_verbose_level_display() {
        assert_eq!(format!("{}", VerboseLevel::Off), "off");
        assert_eq!(format!("{}", VerboseLevel::On), "on");
        assert_eq!(format!("{}", VerboseLevel::Full), "full");
    }
}
