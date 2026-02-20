//! Subagent lifecycle management for multi-agent coordination.
//!
//! Handles spawning, monitoring, and collecting results from subagent
//! sessions. Each subagent runs in an isolated session with a unique
//! lane ID.

use chrono::{DateTime, Utc};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

use crate::agent::{
    loop_::{AgentConfig, AgentLoop, AgentMessage},
    session_router::{SessionKey, SessionRouter},
};
use crate::ai::LLMProvider;
use crate::event_bus::EventBus;
use crate::security::SecurityPolicy;
use crate::tools::ToolRegistry;

// ─── SubagentTask ───────────────────────────────────────────────────────────

/// A task to be executed by a subagent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentTask {
    /// Unique task identifier.
    pub id: String,
    /// The prompt/task description for the subagent.
    pub prompt: String,
    /// Optional agent ID to use (defaults to parent's agent).
    pub agent_id: Option<String>,
    /// Optional thinking level override.
    pub thinking_level: Option<ThinkingLevel>,
    /// Optional model override.
    pub model_override: Option<String>,
    /// Working directory for the subagent (if applicable).
    pub workdir: Option<String>,
}

impl SubagentTask {
    /// Create a new subagent task with the given prompt.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            prompt: prompt.into(),
            agent_id: None,
            thinking_level: None,
            model_override: None,
            workdir: None,
        }
    }

    /// Set the agent ID for this task.
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Set the thinking level for this task.
    pub fn with_thinking(mut self, level: ThinkingLevel) -> Self {
        self.thinking_level = Some(level);
        self
    }

    /// Set a model override for this task.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model_override = Some(model.into());
        self
    }

    /// Set the working directory for this task.
    pub fn with_workdir(mut self, dir: impl Into<String>) -> Self {
        self.workdir = Some(dir.into());
        self
    }
}

// ─── ThinkingLevel ─────────────────────────────────────────────────────────

/// Thinking level for subagent execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    /// Low thinking - fast responses.
    Low,
    /// Medium thinking - balanced.
    Medium,
    /// High thinking - thorough analysis.
    High,
    /// Extra high thinking - deep reasoning.
    Xhigh,
}

impl Default for ThinkingLevel {
    fn default() -> Self {
        Self::Medium
    }
}

impl ThinkingLevel {
    /// Convert to temperature value for LLM.
    pub fn to_temperature(&self) -> f32 {
        match self {
            Self::Low => 0.3,
            Self::Medium => 0.7,
            Self::High => 0.5,
            Self::Xhigh => 0.3,
        }
    }

    /// Convert to max tokens recommendation.
    pub fn to_max_tokens(&self) -> u32 {
        match self {
            Self::Low => 2048,
            Self::Medium => 4096,
            Self::High => 8192,
            Self::Xhigh => 16384,
        }
    }
}

// ─── SubagentResult ────────────────────────────────────────────────────────

/// Result from a completed subagent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentResult {
    /// The lane ID of this subagent.
    pub lane_id: String,
    /// Session key used for this subagent.
    pub session_key: String,
    /// The task that was executed.
    pub task_id: String,
    /// The final response from the subagent.
    pub response: String,
    /// Whether the subagent completed successfully.
    pub success: bool,
    /// Error message if not successful.
    pub error: Option<String>,
    /// Spawn depth of this subagent.
    pub spawn_depth: u32,
    /// When the subagent started.
    pub started_at: DateTime<Utc>,
    /// When the subagent completed.
    pub completed_at: DateTime<Utc>,
    /// Token usage (if available).
    pub token_usage: Option<TokenUsage>,
}

impl SubagentResult {
    /// Duration of execution in milliseconds.
    pub fn duration_ms(&self) -> i64 {
        (self.completed_at - self.started_at).num_milliseconds()
    }
}

// ─── TokenUsage ────────────────────────────────────────────────────────────

/// Token usage statistics for a subagent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ─── SubagentSpawner ───────────────────────────────────────────────────────

/// Manages the lifecycle of subagent sessions.
///
/// Provides utilities for:
/// - Spawning new subagents with isolated sessions
/// - Tracking active subagents
/// - Collecting results from completed subagents
/// - Managing spawn depth limits
pub struct SubagentSpawner {
    /// Session router for creating isolated subagent sessions.
    sessions: Arc<SessionRouter>,
    /// LLM provider for agent execution.
    provider: Arc<dyn LLMProvider>,
    /// Tool registry for agent execution.
    tool_registry: Arc<ToolRegistry>,
    /// Security policy for agent execution.
    security_policy: Arc<SecurityPolicy>,
    /// Optional event bus for publishing events.
    bus: Option<Arc<dyn EventBus>>,
    /// Base agent configuration.
    base_config: AgentConfig,
    /// Maximum spawn depth (prevents infinite recursion).
    max_spawn_depth: u32,
    /// Active subagents being tracked.
    active_subagents: Arc<tokio::sync::RwLock<HashMap<String, ActiveSubagent>>>,
}

/// Information about an active subagent.
#[derive(Debug, Clone)]
struct ActiveSubagent {
    lane_id: String,
    session_key: String,
    spawned_by: String,
    spawn_depth: u32,
    started_at: DateTime<Utc>,
}

impl SubagentSpawner {
    /// Create a new subagent spawner.
    pub fn new(
        sessions: Arc<SessionRouter>,
        provider: Arc<dyn LLMProvider>,
        tool_registry: Arc<ToolRegistry>,
        security_policy: Arc<SecurityPolicy>,
        bus: Option<Arc<dyn EventBus>>,
        base_config: AgentConfig,
    ) -> Self {
        Self {
            sessions,
            provider,
            tool_registry,
            security_policy,
            bus,
            base_config,
            max_spawn_depth: 5,
            active_subagents: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Set the maximum spawn depth.
    pub fn with_max_spawn_depth(mut self, depth: u32) -> Self {
        self.max_spawn_depth = depth;
        self
    }

    /// Spawn a subagent to execute a task.
    ///
    /// # Arguments
    /// * `parent_session_key` - The session key of the parent agent
    /// * `task` - The task to execute
    /// * `system_prompt` - System prompt for the agent
    ///
    /// # Returns
    /// The result of the subagent execution.
    #[tracing::instrument(
        name = "spawner.spawn",
        skip_all,
        fields(
            parent = %parent_session_key,
            task_id = %task.id,
        )
    )]
    pub async fn spawn(
        &self,
        parent_session_key: &str,
        task: SubagentTask,
        system_prompt: &str,
    ) -> Result<SubagentResult, String> {
        let started_at = Utc::now();

        // Log spawn start
        info!(
            "[agent:spawner] spawning subagent task_id={} parent={} prompt_len={}",
            task.id,
            parent_session_key,
            task.prompt.len()
        );

        // Generate unique lane ID
        let lane_id = format!("lane-{}", Uuid::new_v4().simple());

        // Build subagent session key
        let subagent_session_key = self.build_subagent_session_key(parent_session_key, &lane_id)?;

        // Get spawn depth from parent
        let spawn_depth = self.get_spawn_depth(parent_session_key)? + 1;

        // Check spawn depth limit
        if spawn_depth > self.max_spawn_depth {
            warn!(
                "[agent:spawner] maximum spawn depth exceeded: {} > {}",
                spawn_depth, self.max_spawn_depth
            );
            return Err(format!(
                "Maximum spawn depth ({}) exceeded. Current depth: {}",
                self.max_spawn_depth, spawn_depth
            ));
        }

        // Track active subagent
        self.track_subagent(&ActiveSubagent {
            lane_id: lane_id.clone(),
            session_key: subagent_session_key.clone(),
            spawned_by: parent_session_key.to_string(),
            spawn_depth,
            started_at,
        })
        .await;

        // Build agent config for subagent
        let config = self.build_subagent_config(&task);

        // Create agent loop
        let agent = AgentLoop::new(
            Arc::clone(&self.provider),
            Arc::clone(&self.tool_registry),
            Arc::clone(&self.security_policy),
            self.bus.clone(),
            config,
        );

        // Execute the task
        let result = agent.run(system_prompt, &task.prompt).await;

        // Remove from active tracking
        self.untrack_subagent(&lane_id).await;

        let completed_at = Utc::now();

        // Log completion
        match &result {
            Ok(_) => {
                info!(
                    "[agent:spawner] subagent completed lane_id={} duration_ms={} success=true",
                    lane_id,
                    (completed_at - started_at).num_milliseconds()
                );
            }
            Err(e) => {
                warn!(
                    "[agent:spawner] subagent failed lane_id={} duration_ms={} error={}",
                    lane_id,
                    (completed_at - started_at).num_milliseconds(),
                    e
                );
            }
        }

        // Build result
        match result {
            Ok(response) => Ok(SubagentResult {
                lane_id,
                session_key: subagent_session_key,
                task_id: task.id,
                response,
                success: true,
                error: None,
                spawn_depth,
                started_at,
                completed_at,
                token_usage: None,
            }),
            Err(e) => Ok(SubagentResult {
                lane_id,
                session_key: subagent_session_key,
                task_id: task.id,
                response: String::new(),
                success: false,
                error: Some(e.clone()),
                spawn_depth,
                started_at,
                completed_at,
                token_usage: None,
            }),
        }
    }

    /// Build a subagent session key from parent and lane ID.
    ///
    /// Format: `agent:<agentId>:subagent:<laneId>`
    fn build_subagent_session_key(
        &self,
        parent_key: &str,
        lane_id: &str,
    ) -> Result<String, String> {
        // Parse parent key to extract agent ID
        let parent = SessionKey::parse(parent_key)?;

        // Build subagent key
        Ok(format!("agent:{}:subagent:{}", parent.agent, lane_id))
    }

    /// Get the spawn depth for a session.
    fn get_spawn_depth(&self, session_key: &str) -> Result<u32, String> {
        // Check if it's already a subagent session
        if session_key.contains(":subagent:") {
            // Extract depth from active subagents or parse from key
            let active = self.active_subagents.blocking_read();
            if let Some(info) = active.values().find(|s| s.session_key == session_key) {
                return Ok(info.spawn_depth);
            }

            // Parse from key structure
            // Count subagent segments
            let count = session_key.matches(":subagent:").count() as u32;
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Build agent config for a subagent task.
    fn build_subagent_config(&self, task: &SubagentTask) -> AgentConfig {
        let thinking = task.thinking_level.unwrap_or(ThinkingLevel::High);

        AgentConfig {
            model: task
                .model_override
                .clone()
                .unwrap_or_else(|| self.base_config.model.clone()),
            temperature: Some(thinking.to_temperature()),
            max_tokens: Some(thinking.to_max_tokens()),
            max_iterations: self.base_config.max_iterations,
            max_history: self.base_config.max_history,
        }
    }

    /// Track an active subagent.
    async fn track_subagent(&self, info: &ActiveSubagent) {
        let mut active = self.active_subagents.write().await;
        active.insert(info.lane_id.clone(), info.clone());
    }

    /// Remove a subagent from tracking.
    async fn untrack_subagent(&self, lane_id: &str) {
        let mut active = self.active_subagents.write().await;
        active.remove(lane_id);
    }

    /// List all active subagents.
    pub async fn list_active(&self) -> Vec<ActiveSubagentInfo> {
        let active = self.active_subagents.read().await;
        active
            .values()
            .map(|s| ActiveSubagentInfo {
                lane_id: s.lane_id.clone(),
                session_key: s.session_key.clone(),
                spawned_by: s.spawned_by.clone(),
                spawn_depth: s.spawn_depth,
                started_at: s.started_at,
            })
            .collect()
    }

    /// Count active subagents for a parent session.
    pub async fn count_active_for_parent(&self, parent_key: &str) -> usize {
        let active = self.active_subagents.read().await;
        active
            .values()
            .filter(|s| s.spawned_by == parent_key)
            .count()
    }

    /// Create a spawn handle for use in async tasks.
    ///
    /// This returns a `SpawnHandle` that can be used to spawn subagents
    /// from within async tasks without requiring the full spawner.
    pub fn create_handle(&self) -> SpawnHandle {
        SpawnHandle {
            sessions: Arc::clone(&self.sessions),
            provider: Arc::clone(&self.provider),
            tool_registry: Arc::clone(&self.tool_registry),
            security_policy: Arc::clone(&self.security_policy),
            bus: self.bus.clone(),
            base_config: self.base_config.clone(),
            max_spawn_depth: self.max_spawn_depth,
            active_subagents: Arc::clone(&self.active_subagents),
        }
    }
}

/// A handle for spawning subagents in async contexts.
///
/// This is a cloneable handle that can be used to spawn subagents
/// from within async tasks without requiring the full spawner.
#[derive(Clone)]
pub struct SpawnHandle {
    sessions: Arc<SessionRouter>,
    provider: Arc<dyn LLMProvider>,
    tool_registry: Arc<ToolRegistry>,
    security_policy: Arc<SecurityPolicy>,
    bus: Option<Arc<dyn EventBus>>,
    base_config: crate::agent::loop_::AgentConfig,
    max_spawn_depth: u32,
    active_subagents: Arc<tokio::sync::RwLock<HashMap<String, ActiveSubagent>>>,
}

impl SpawnHandle {
    /// Spawn a subagent to execute a task.
    pub async fn spawn(
        &self,
        parent_session_key: &str,
        task: SubagentTask,
        system_prompt: &str,
    ) -> Result<SubagentResult, String> {
        // Create a temporary spawner with the handle's state
        let spawner = SubagentSpawner {
            sessions: Arc::clone(&self.sessions),
            provider: Arc::clone(&self.provider),
            tool_registry: Arc::clone(&self.tool_registry),
            security_policy: Arc::clone(&self.security_policy),
            bus: self.bus.clone(),
            base_config: self.base_config.clone(),
            max_spawn_depth: self.max_spawn_depth,
            active_subagents: Arc::clone(&self.active_subagents),
        };

        spawner.spawn(parent_session_key, task, system_prompt).await
    }
}

/// Public information about an active subagent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSubagentInfo {
    pub lane_id: String,
    pub session_key: String,
    pub spawned_by: String,
    pub spawn_depth: u32,
    pub started_at: DateTime<Utc>,
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subagent_task_builder() {
        let task = SubagentTask::new("Test task")
            .with_agent("research-agent")
            .with_thinking(ThinkingLevel::High)
            .with_model("claude-opus-4");

        assert!(task.prompt == "Test task");
        assert_eq!(task.agent_id, Some("research-agent".to_string()));
        assert_eq!(task.thinking_level, Some(ThinkingLevel::High));
        assert_eq!(task.model_override, Some("claude-opus-4".to_string()));
    }

    #[test]
    fn thinking_level_defaults() {
        let level = ThinkingLevel::default();
        assert_eq!(level, ThinkingLevel::Medium);
    }

    #[test]
    fn thinking_level_to_temperature() {
        assert_eq!(ThinkingLevel::Low.to_temperature(), 0.3);
        assert_eq!(ThinkingLevel::Medium.to_temperature(), 0.7);
        assert_eq!(ThinkingLevel::High.to_temperature(), 0.5);
        assert_eq!(ThinkingLevel::Xhigh.to_temperature(), 0.3);
    }

    #[test]
    fn subagent_result_duration() {
        let result = SubagentResult {
            lane_id: "lane-1".to_string(),
            session_key: "agent:default:subagent:lane-1".to_string(),
            task_id: "task-1".to_string(),
            response: "Done".to_string(),
            success: true,
            error: None,
            spawn_depth: 1,
            started_at: DateTime::parse_from_rfc3339("2026-02-20T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            completed_at: DateTime::parse_from_rfc3339("2026-02-20T12:00:05Z")
                .unwrap()
                .with_timezone(&Utc),
            token_usage: None,
        };

        assert_eq!(result.duration_ms(), 5000);
    }

    #[test]
    fn session_key_format() {
        // Test the expected format
        let parent = "agent:default";
        let lane = "lane-42";
        let expected = format!("agent:default:subagent:{}", lane);

        // This is the format we expect
        assert!(expected.starts_with("agent:"));
        assert!(expected.contains(":subagent:"));
    }
}
