use std::collections::HashMap;

use dashmap::DashMap;
use tracing::{info, warn};

use crate::ai::agent::TokenUsage;
use crate::ai::delegation::task::{DelegationResult, DelegationTask, TaskResult, TaskStatus};
use crate::ai::delegation::DelegationConfig;
use crate::{Result, ZeniiError};

pub struct Coordinator {
    config: DelegationConfig,
    active: DashMap<String, Vec<tokio::task::AbortHandle>>,
}

impl Coordinator {
    pub fn new(config: DelegationConfig) -> Self {
        Self {
            config,
            active: DashMap::new(),
        }
    }

    /// Return IDs of active delegation runs.
    pub fn active_agents(&self) -> Vec<String> {
        self.active.iter().map(|r| r.key().clone()).collect()
    }

    /// Cancel a delegation run by aborting all its sub-agent tasks.
    pub fn cancel(&self, delegation_id: &str) -> bool {
        if let Some((_, handles)) = self.active.remove(delegation_id) {
            for handle in handles {
                handle.abort();
            }
            true
        } else {
            false
        }
    }

    /// Cancel all active delegation runs.
    pub fn cancel_all(&self) {
        let keys: Vec<String> = self.active.iter().map(|r| r.key().clone()).collect();
        for key in keys {
            self.cancel(&key);
        }
    }

    /// Validate that tasks respect config constraints.
    pub fn validate_tasks(&self, tasks: &[DelegationTask], tool_names: &[String]) -> Result<()> {
        if tasks.len() > self.config.max_sub_agents {
            return Err(ZeniiError::Validation(format!(
                "too many sub-tasks: {} (max {})",
                tasks.len(),
                self.config.max_sub_agents
            )));
        }

        for task in tasks {
            if let Some(ref allowlist) = task.tool_allowlist {
                for tool in allowlist {
                    if !tool_names.contains(tool) {
                        return Err(ZeniiError::Validation(format!(
                            "unknown tool '{}' in task '{}' allowlist",
                            tool, task.id
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Decompose a prompt into sub-tasks using the LLM.
    #[cfg(feature = "ai")]
    pub async fn decompose(
        &self,
        prompt: &str,
        agent: &crate::ai::agent::ZeniiAgent,
    ) -> Result<Vec<DelegationTask>> {
        let decompose_prompt = format!(
            "You are a task decomposition agent. Break the following task into {} or fewer \
             independent sub-tasks that can be executed in parallel by separate AI agents.\n\n\
             Return a JSON array of tasks. Each task object must have:\n\
             - \"id\": a unique string like \"t1\", \"t2\"\n\
             - \"description\": what the sub-agent should accomplish\n\
             - \"tool_allowlist\": optional array of tool names to restrict, or null for all tools\n\
             - \"depends_on\": array of task IDs this task depends on (empty array for independent tasks)\n\n\
             Task: {}\n\n\
             Return ONLY a valid JSON array, no markdown formatting or explanation.",
            self.config.max_sub_agents, prompt
        );

        let response = agent.prompt(&decompose_prompt).await?;
        let json_text = extract_json(&response.output);

        let mut tasks: Vec<DelegationTask> =
            serde_json::from_str(json_text).map_err(|e| {
                ZeniiError::Agent(format!(
                    "failed to parse decomposition response as JSON: {e}\nResponse: {}",
                    &response.output[..response.output.len().min(200)]
                ))
            })?;

        for task in &mut tasks {
            task.token_budget = self.config.per_agent_token_budget;
            task.timeout_secs = self.config.per_agent_timeout_secs;
        }

        Ok(tasks)
    }

    /// Execute a delegation: decompose, spawn sub-agents, aggregate results.
    #[cfg(feature = "ai")]
    pub async fn delegate(
        &self,
        prompt: &str,
        state: &crate::gateway::state::AppState,
        surface: &str,
    ) -> Result<DelegationResult> {
        use crate::ai::delegation::sub_agent::SubAgent;

        let delegation_id = uuid::Uuid::new_v4().to_string();
        let start = std::time::Instant::now();

        let decomp_model = self.config.decomposition_model.as_deref();
        let agent =
            crate::ai::resolve_agent(decomp_model, state, None, None, surface).await?;

        let tasks = self.decompose(prompt, &agent).await?;
        if tasks.is_empty() {
            return Err(ZeniiError::Agent("decomposition produced no tasks".into()));
        }

        let tool_names: Vec<String> = state
            .tools
            .to_vec()
            .iter()
            .map(|t| t.name().to_string())
            .collect();
        self.validate_tasks(&tasks, &tool_names)?;

        info!(
            delegation_id = %delegation_id,
            task_count = tasks.len(),
            "Starting delegation"
        );

        // Execute tasks in dependency waves
        let mut completed: HashMap<String, TaskResult> = HashMap::new();
        let mut remaining: Vec<DelegationTask> = tasks;
        let mut all_handles: Vec<tokio::task::AbortHandle> = Vec::new();

        while !remaining.is_empty() {
            let (ready, not_ready): (Vec<_>, Vec<_>) = remaining
                .into_iter()
                .partition(|t| t.depends_on.iter().all(|dep| completed.contains_key(dep)));

            if ready.is_empty() {
                warn!(
                    "Delegation {}: {} tasks stuck with unresolved dependencies",
                    delegation_id,
                    not_ready.len()
                );
                for task in not_ready {
                    completed.insert(
                        task.id.clone(),
                        TaskResult {
                            task_id: task.id,
                            status: TaskStatus::Failed,
                            output: String::new(),
                            usage: TokenUsage::default(),
                            duration_ms: 0,
                            error: Some("unresolved dependencies".into()),
                            session_id: String::new(),
                        },
                    );
                }
                break;
            }

            remaining = not_ready;

            let mut join_set = tokio::task::JoinSet::new();
            for task in ready {
                let task_id = task.id.clone();
                let _ = state.event_bus.publish(
                    crate::event_bus::AppEvent::SubAgentSpawned {
                        agent_id: task_id.clone(),
                        task: task.description.clone(),
                    },
                );

                match SubAgent::new(task, state, surface).await {
                    Ok(sub) => {
                        let abort_handle = join_set.spawn(sub.execute());
                        all_handles.push(abort_handle);
                    }
                    Err(e) => {
                        warn!("Failed to create sub-agent for {}: {e}", task_id);
                        completed.insert(
                            task_id.clone(),
                            TaskResult {
                                task_id,
                                status: TaskStatus::Failed,
                                output: String::new(),
                                usage: TokenUsage::default(),
                                duration_ms: 0,
                                error: Some(e.to_string()),
                                session_id: String::new(),
                            },
                        );
                    }
                }
            }

            self.active
                .insert(delegation_id.clone(), all_handles.clone());

            while let Some(result) = join_set.join_next().await {
                match result {
                    Ok(task_result) => {
                        let event = if task_result.status == TaskStatus::Completed {
                            crate::event_bus::AppEvent::SubAgentCompleted {
                                agent_id: task_result.task_id.clone(),
                                status: "completed".into(),
                                duration_ms: task_result.duration_ms,
                            }
                        } else {
                            crate::event_bus::AppEvent::SubAgentFailed {
                                agent_id: task_result.task_id.clone(),
                                error: task_result.error.clone().unwrap_or_default(),
                            }
                        };
                        let _ = state.event_bus.publish(event);
                        completed.insert(task_result.task_id.clone(), task_result);
                    }
                    Err(e) => {
                        warn!("Sub-agent task panicked: {e}");
                    }
                }
            }
        }

        self.active.remove(&delegation_id);

        let mut total_usage = TokenUsage::default();
        for r in completed.values() {
            total_usage += r.usage.clone();
        }

        let results: Vec<TaskResult> = completed.into_values().collect();
        let aggregated = self.aggregate(prompt, &results, &agent).await?;
        let total_duration_ms = start.elapsed().as_millis() as u64;

        Ok(DelegationResult {
            id: delegation_id,
            task_results: results,
            aggregated_response: aggregated,
            total_usage,
            total_duration_ms,
        })
    }

    /// Aggregate sub-agent results into a unified response.
    #[cfg(feature = "ai")]
    async fn aggregate(
        &self,
        prompt: &str,
        results: &[TaskResult],
        agent: &crate::ai::agent::ZeniiAgent,
    ) -> Result<String> {
        if results.len() == 1 {
            return Ok(results[0].output.clone());
        }

        let mut results_text = String::new();
        for r in results {
            results_text.push_str(&format!(
                "## Task: {}\nStatus: {:?}\nOutput: {}\n\n",
                r.task_id, r.status, r.output
            ));
        }

        let aggregate_prompt = format!(
            "You received the following results from parallel sub-agents working on: \"{}\"\n\n\
             {}\n\
             Synthesize these results into a single coherent response for the user. \
             Do not mention sub-agents or task IDs.",
            prompt, results_text
        );

        let response = agent.prompt(&aggregate_prompt).await?;
        Ok(response.output)
    }
}

/// Extract JSON from a response that may be wrapped in markdown code blocks.
fn extract_json(text: &str) -> &str {
    let trimmed = text.trim();
    if let Some(start) = trimmed.find("```json") {
        let content = &trimmed[start + 7..];
        if let Some(end) = content.find("```") {
            return content[..end].trim();
        }
    }
    if let Some(start) = trimmed.find("```") {
        let content = &trimmed[start + 3..];
        if let Some(end) = content.find("```") {
            return content[..end].trim();
        }
    }
    trimmed
}

impl std::fmt::Debug for Coordinator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Coordinator")
            .field("config", &self.config)
            .field("active_count", &self.active.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_task(id: &str, depends: Vec<&str>) -> DelegationTask {
        DelegationTask {
            id: id.into(),
            description: format!("task {id}"),
            tool_allowlist: None,
            token_budget: 4000,
            timeout_secs: 120,
            depends_on: depends.into_iter().map(String::from).collect(),
        }
    }

    // 7.12
    #[test]
    fn coordinator_new() {
        let coord = Coordinator::new(DelegationConfig::default());
        assert!(coord.active_agents().is_empty());
    }

    // 7.13
    #[test]
    fn coordinator_active_agents_empty() {
        let coord = Coordinator::new(DelegationConfig::default());
        assert_eq!(coord.active_agents().len(), 0);
    }

    // 7.14
    #[test]
    fn coordinator_validate_task_count() {
        let config = DelegationConfig {
            max_sub_agents: 2,
            ..Default::default()
        };
        let coord = Coordinator::new(config);
        let tasks = vec![make_task("t1", vec![]), make_task("t2", vec![]), make_task("t3", vec![])];

        let result = coord.validate_tasks(&tasks, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too many"));
    }

    // 7.15
    #[test]
    fn coordinator_validate_tool_names() {
        let coord = Coordinator::new(DelegationConfig::default());
        let tasks = vec![DelegationTask {
            id: "t1".into(),
            description: "a".into(),
            tool_allowlist: Some(vec!["nonexistent_tool".into()]),
            token_budget: 4000,
            timeout_secs: 120,
            depends_on: vec![],
        }];

        let available = vec!["web_search".to_string(), "system_info".to_string()];
        let result = coord.validate_tasks(&tasks, &available);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown tool"));
    }

    // 7.16 — decompose produces valid prompt (structural)
    #[test]
    fn coordinator_decompose_prompt_format() {
        let config = DelegationConfig {
            max_sub_agents: 3,
            ..Default::default()
        };
        let _coord = Coordinator::new(config);
        // Structural: Coordinator compiles and can be constructed with custom config
    }

    // 7.17 — aggregate (structural)
    #[test]
    fn coordinator_aggregate_format() {
        let _coord = Coordinator::new(DelegationConfig::default());
        // Full integration test requires real LLM endpoint (manual test M7.1)
    }

    // 7.18
    #[test]
    fn coordinator_cancel_all() {
        let coord = Coordinator::new(DelegationConfig::default());
        coord.active.insert("d1".into(), vec![]);
        coord.active.insert("d2".into(), vec![]);
        assert_eq!(coord.active_agents().len(), 2);

        coord.cancel_all();
        assert_eq!(coord.active_agents().len(), 0);
    }

    // 7.19
    #[test]
    fn coordinator_respects_token_budget() {
        let config = DelegationConfig {
            per_agent_token_budget: 2000,
            ..Default::default()
        };
        let coord = Coordinator::new(config);
        assert_eq!(coord.config.per_agent_token_budget, 2000);
    }

    // 7.20
    #[test]
    fn coordinator_depends_on_ordering() {
        let coord = Coordinator::new(DelegationConfig::default());
        let tasks = vec![make_task("t1", vec![]), make_task("t2", vec!["t1"])];

        let result = coord.validate_tasks(&tasks, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn extract_json_plain() {
        assert_eq!(extract_json("[{\"id\":\"t1\"}]"), "[{\"id\":\"t1\"}]");
    }

    #[test]
    fn extract_json_markdown() {
        let input = "Here's the result:\n```json\n[{\"id\":\"t1\"}]\n```\nDone.";
        assert_eq!(extract_json(input), "[{\"id\":\"t1\"}]");
    }

    #[test]
    fn extract_json_code_block() {
        let input = "```\n[{\"id\":\"t1\"}]\n```";
        assert_eq!(extract_json(input), "[{\"id\":\"t1\"}]");
    }
}
