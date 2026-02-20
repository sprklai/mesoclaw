//! Multi-agent orchestration for coordinating parallel agent execution.
//!
//! Provides high-level coordination of multiple agents running in parallel,
//! with support for result aggregation, failure handling, and execution
//! strategies (all, first, any N).

use chrono::{DateTime, Utc};
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinSet;

use crate::agent::session_router::SessionRouter;
use crate::ai::LLMProvider;
use crate::event_bus::EventBus;
use crate::security::SecurityPolicy;
use crate::tools::ToolRegistry;

use super::spawner::{SubagentResult, SubagentSpawner, SubagentTask};

// ─── ParallelExecutionConfig ───────────────────────────────────────────────

/// Configuration for parallel execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParallelExecutionConfig {
    /// Maximum number of concurrent subagents.
    pub max_concurrency: usize,
    /// Failure handling strategy.
    pub on_fail: FailureStrategy,
    /// Execution mode (all, first, any).
    pub mode: ExecutionMode,
    /// For "any" mode: number of successful results needed.
    pub required_count: Option<usize>,
    /// Timeout per subagent in seconds (0 = no timeout).
    pub timeout_secs: u64,
    /// Whether to cancel remaining subagents when target is reached.
    pub cancel_on_target: bool,
}

impl Default for ParallelExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 5,
            on_fail: FailureStrategy::Continue,
            mode: ExecutionMode::All,
            required_count: None,
            timeout_secs: 0,
            cancel_on_target: false,
        }
    }
}

// ─── FailureStrategy ───────────────────────────────────────────────────────

/// How to handle subagent failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FailureStrategy {
    /// Continue with other subagents, collect what succeeded.
    Continue,
    /// Fail the entire operation if any subagent fails.
    FailFast,
    /// Ignore failures completely (don't even report them).
    Ignore,
}

// ─── ExecutionMode ─────────────────────────────────────────────────────────

/// Execution mode for parallel tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    /// Run all tasks and collect all results.
    All,
    /// Return as soon as the first task completes successfully.
    First,
    /// Return when N tasks complete successfully (quorum).
    Any,
}

// ─── ParallelResult ────────────────────────────────────────────────────────

/// Result from parallel execution of multiple subagent tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParallelResult {
    /// All subagent results (successful and failed).
    pub results: Vec<SubagentResult>,
    /// Number of successful results.
    pub success_count: usize,
    /// Number of failed results.
    pub failure_count: usize,
    /// Whether the overall operation succeeded.
    pub overall_success: bool,
    /// Execution mode that was used.
    pub mode: ExecutionMode,
    /// Total wall-clock time in milliseconds.
    pub total_duration_ms: i64,
    /// When execution started.
    pub started_at: DateTime<Utc>,
    /// When execution completed.
    pub completed_at: DateTime<Utc>,
}

impl ParallelResult {
    /// Get only successful results.
    pub fn successful(&self) -> Vec<&SubagentResult> {
        self.results.iter().filter(|r| r.success).collect()
    }

    /// Get only failed results.
    pub fn failed(&self) -> Vec<&SubagentResult> {
        self.results.iter().filter(|r| !r.success).collect()
    }

    /// Get all responses from successful subagents.
    pub fn responses(&self) -> Vec<&str> {
        self.results
            .iter()
            .filter(|r| r.success)
            .map(|r| r.response.as_str())
            .collect()
    }

    /// Aggregate all responses into a single string.
    pub fn aggregated_response(&self) -> String {
        self.responses().join("\n\n---\n\n")
    }
}

// ─── AgentOrchestrator ─────────────────────────────────────────────────────

/// Coordinates multi-agent execution with parallel task support.
///
/// # Capabilities
/// - Spawn multiple subagents in parallel
/// - Aggregate results with different strategies
/// - Handle failures gracefully
/// - Support quorum-based execution (any N of M)
/// - Track active subagents and enforce concurrency limits
///
/// # Example
/// ```rust,ignore
/// let orchestrator = AgentOrchestrator::new(
///     sessions,
///     provider,
///     tool_registry,
///     security_policy,
///     Some(bus),
///     AgentConfig::default(),
/// );
///
/// // Run 3 tasks in parallel, continue on failures
/// let config = ParallelExecutionConfig {
///     max_concurrency: 3,
///     on_fail: FailureStrategy::Continue,
///     ..Default::default()
/// };
///
/// let result = orchestrator.run_parallel_tasks(
///     "agent:default",
///     "You are a research assistant.",
///     vec![
///         SubagentTask::new("Research topic A"),
///         SubagentTask::new("Research topic B"),
///         SubagentTask::new("Research topic C"),
///     ],
///     Some(config),
/// ).await?;
///
/// println!("Completed {} tasks in {}ms",
///     result.success_count,
///     result.total_duration_ms
/// );
/// ```
pub struct AgentOrchestrator {
    /// Subagent spawner for lifecycle management.
    spawner: SubagentSpawner,
    /// Default parallel execution config.
    default_config: ParallelExecutionConfig,
}

impl AgentOrchestrator {
    /// Create a new agent orchestrator.
    pub fn new(
        sessions: Arc<SessionRouter>,
        provider: Arc<dyn LLMProvider>,
        tool_registry: Arc<ToolRegistry>,
        security_policy: Arc<SecurityPolicy>,
        bus: Option<Arc<dyn EventBus>>,
        base_config: crate::agent::loop_::AgentConfig,
    ) -> Self {
        Self {
            spawner: SubagentSpawner::new(
                sessions,
                provider,
                tool_registry,
                security_policy,
                bus,
                base_config,
            ),
            default_config: ParallelExecutionConfig::default(),
        }
    }

    /// Set the default parallel execution config.
    pub fn with_default_config(mut self, config: ParallelExecutionConfig) -> Self {
        self.default_config = config;
        self
    }

    /// Run multiple subagent tasks in parallel.
    ///
    /// # Arguments
    /// * `parent_session_key` - Session key of the parent agent
    /// * `system_prompt` - System prompt for all subagents
    /// * `tasks` - Tasks to execute in parallel
    /// * `config` - Optional execution config (uses default if None)
    ///
    /// # Returns
    /// Aggregated results from all subagents.
    #[tracing::instrument(
        name = "orchestrator.run_parallel",
        skip_all,
        fields(
            parent = %parent_session_key,
            task_count = tasks.len(),
        )
    )]
    pub async fn run_parallel_tasks(
        &self,
        parent_session_key: &str,
        system_prompt: &str,
        tasks: Vec<SubagentTask>,
        config: Option<ParallelExecutionConfig>,
    ) -> Result<ParallelResult, String> {
        let config = config.unwrap_or_else(|| self.default_config.clone());
        let started_at = Utc::now();

        info!(
            "[agent:orchestrator] starting orchestration session={} tasks={} mode={:?}",
            parent_session_key,
            tasks.len(),
            config.mode
        );

        if tasks.is_empty() {
            return Ok(ParallelResult {
                results: vec![],
                success_count: 0,
                failure_count: 0,
                overall_success: true,
                mode: config.mode,
                total_duration_ms: 0,
                started_at,
                completed_at: started_at,
            });
        }

        // Execute based on mode
        let results = match config.mode {
            ExecutionMode::All => {
                self.execute_all(parent_session_key, system_prompt, tasks, &config)
                    .await?
            }
            ExecutionMode::First => {
                self.execute_first(parent_session_key, system_prompt, tasks, &config)
                    .await?
            }
            ExecutionMode::Any => {
                self.execute_any(parent_session_key, system_prompt, tasks, &config)
                    .await?
            }
        };

        let completed_at = Utc::now();
        let total_duration_ms = (completed_at - started_at).num_milliseconds();

        // Calculate success/failure counts
        let success_count = results.iter().filter(|r| r.success).count();
        let failure_count = results.len() - success_count;

        // Determine overall success based on mode and strategy
        let overall_success = self.determine_overall_success(success_count, failure_count, &config);

        info!(
            "[agent:orchestrator] orchestration completed session={} success_count={} failure_count={} overall_success={} duration_ms={}",
            parent_session_key,
            success_count,
            failure_count,
            overall_success,
            total_duration_ms
        );

        Ok(ParallelResult {
            results,
            success_count,
            failure_count,
            overall_success,
            mode: config.mode,
            total_duration_ms,
            started_at,
            completed_at,
        })
    }

    /// Execute all tasks and collect all results.
    async fn execute_all(
        &self,
        parent_session_key: &str,
        system_prompt: &str,
        tasks: Vec<SubagentTask>,
        config: &ParallelExecutionConfig,
    ) -> Result<Vec<SubagentResult>, String> {
        let mut join_set = JoinSet::new();
        let mut results = Vec::with_capacity(tasks.len());

        // Spawn tasks with concurrency limit
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrency));

        for task in tasks {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let spawner = self.spawner.create_handle();
            let parent_key = parent_session_key.to_string();
            let prompt = system_prompt.to_string();

            join_set.spawn(async move {
                let result = spawner.spawn(&parent_key, task, &prompt).await;
                drop(permit); // Release semaphore permit
                result
            });
        }

        // Collect results
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(Ok(result)) => {
                    let is_failure = !result.success;
                    results.push(result);

                    // Check fail-fast
                    if is_failure && config.on_fail == FailureStrategy::FailFast {
                        // Cancel remaining tasks
                        join_set.shutdown().await;
                        break;
                    }
                }
                Ok(Err(e)) => {
                    if config.on_fail == FailureStrategy::FailFast {
                        return Err(e);
                    }
                    // Otherwise continue
                }
                Err(e) => {
                    if config.on_fail == FailureStrategy::FailFast {
                        return Err(e.to_string());
                    }
                }
            }
        }

        // Filter based on strategy
        if config.on_fail == FailureStrategy::Ignore {
            results.retain(|r| r.success);
        }

        Ok(results)
    }

    /// Execute tasks and return first successful result.
    async fn execute_first(
        &self,
        parent_session_key: &str,
        system_prompt: &str,
        tasks: Vec<SubagentTask>,
        config: &ParallelExecutionConfig,
    ) -> Result<Vec<SubagentResult>, String> {
        let mut join_set = JoinSet::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrency));

        for task in tasks {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let spawner = self.spawner.create_handle();
            let parent_key = parent_session_key.to_string();
            let prompt = system_prompt.to_string();

            join_set.spawn(async move {
                let result = spawner.spawn(&parent_key, task, &prompt).await;
                drop(permit);
                result
            });
        }

        let mut results = Vec::new();
        let mut first_success = false;

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(Ok(result)) => {
                    let success = result.success;
                    results.push(result);

                    if success && !first_success {
                        first_success = true;
                        if config.cancel_on_target {
                            join_set.shutdown().await;
                            break;
                        }
                    }
                }
                Ok(Err(e)) => {
                    if config.on_fail == FailureStrategy::FailFast {
                        join_set.shutdown().await;
                        return Err(e);
                    }
                }
                Err(e) => {
                    if config.on_fail == FailureStrategy::FailFast {
                        join_set.shutdown().await;
                        return Err(e.to_string());
                    }
                }
            }
        }

        Ok(results)
    }

    /// Execute tasks and return when N succeed (quorum).
    async fn execute_any(
        &self,
        parent_session_key: &str,
        system_prompt: &str,
        tasks: Vec<SubagentTask>,
        config: &ParallelExecutionConfig,
    ) -> Result<Vec<SubagentResult>, String> {
        let required = config.required_count.unwrap_or(1);
        let mut join_set = JoinSet::new();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(config.max_concurrency));

        for task in tasks {
            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let spawner = self.spawner.create_handle();
            let parent_key = parent_session_key.to_string();
            let prompt = system_prompt.to_string();

            join_set.spawn(async move {
                let result = spawner.spawn(&parent_key, task, &prompt).await;
                drop(permit);
                result
            });
        }

        let mut results = Vec::new();
        let mut success_count = 0;

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(Ok(result)) => {
                    if result.success {
                        success_count += 1;
                    }
                    results.push(result);

                    // Check if we have enough
                    if success_count >= required && config.cancel_on_target {
                        join_set.shutdown().await;
                        break;
                    }
                }
                Ok(Err(e)) => {
                    if config.on_fail == FailureStrategy::FailFast {
                        join_set.shutdown().await;
                        return Err(e);
                    }
                }
                Err(e) => {
                    if config.on_fail == FailureStrategy::FailFast {
                        join_set.shutdown().await;
                        return Err(e.to_string());
                    }
                }
            }
        }

        Ok(results)
    }

    /// Determine overall success based on results and config.
    fn determine_overall_success(
        &self,
        success_count: usize,
        failure_count: usize,
        config: &ParallelExecutionConfig,
    ) -> bool {
        match config.mode {
            ExecutionMode::All => match config.on_fail {
                FailureStrategy::FailFast => failure_count == 0,
                FailureStrategy::Continue => success_count > 0,
                FailureStrategy::Ignore => true,
            },
            ExecutionMode::First => success_count > 0,
            ExecutionMode::Any => {
                let required = config.required_count.unwrap_or(1);
                success_count >= required
            }
        }
    }

    /// List all active subagents.
    pub async fn list_active_subagents(&self) -> Vec<super::spawner::ActiveSubagentInfo> {
        self.spawner.list_active().await
    }

    /// Count active subagents for a parent session.
    pub async fn count_active_for_parent(&self, parent_key: &str) -> usize {
        self.spawner.count_active_for_parent(parent_key).await
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_config_defaults() {
        let config = ParallelExecutionConfig::default();
        assert_eq!(config.max_concurrency, 5);
        assert_eq!(config.on_fail, FailureStrategy::Continue);
        assert_eq!(config.mode, ExecutionMode::All);
    }

    #[test]
    fn parallel_result_helpers() {
        let result = ParallelResult {
            results: vec![
                SubagentResult {
                    lane_id: "1".to_string(),
                    session_key: "agent:default:subagent:1".to_string(),
                    task_id: "t1".to_string(),
                    response: "Response 1".to_string(),
                    success: true,
                    error: None,
                    spawn_depth: 1,
                    started_at: Utc::now(),
                    completed_at: Utc::now(),
                    token_usage: None,
                },
                SubagentResult {
                    lane_id: "2".to_string(),
                    session_key: "agent:default:subagent:2".to_string(),
                    task_id: "t2".to_string(),
                    response: "".to_string(),
                    success: false,
                    error: Some("Failed".to_string()),
                    spawn_depth: 1,
                    started_at: Utc::now(),
                    completed_at: Utc::now(),
                    token_usage: None,
                },
            ],
            success_count: 1,
            failure_count: 1,
            overall_success: true,
            mode: ExecutionMode::All,
            total_duration_ms: 1000,
            started_at: Utc::now(),
            completed_at: Utc::now(),
        };

        let successful = result.successful();
        assert_eq!(successful.len(), 1);

        let failed = result.failed();
        assert_eq!(failed.len(), 1);

        let responses = result.responses();
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0], "Response 1");
    }

    #[test]
    fn failure_strategy_serialization() {
        let strategy = FailureStrategy::Continue;
        let json = serde_json::to_string(&strategy).unwrap();
        assert_eq!(json, "\"continue\"");

        let parsed: FailureStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, FailureStrategy::Continue);
    }

    #[test]
    fn execution_mode_serialization() {
        let mode = ExecutionMode::First;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"first\"");

        let parsed: ExecutionMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, ExecutionMode::First);
    }
}
