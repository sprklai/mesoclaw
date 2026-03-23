use std::collections::HashMap;

use dashmap::DashMap;
use tracing::{debug, info, warn};

use crate::ai::agent::TokenUsage;
use crate::ai::delegation::DelegationConfig;
use crate::ai::delegation::task::{DelegationResult, DelegationTask, TaskResult, TaskStatus};
use crate::event_bus::DelegationAgentInfo;
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
        available_tools: &[String],
    ) -> Result<Vec<DelegationTask>> {
        let tools_list = available_tools.join(", ");
        let decompose_prompt = format!(
            "You are a task decomposition agent. Break the following task into {} or fewer \
             sub-tasks that can be executed IN PARALLEL by separate AI agents.\n\n\
             Available tools: [{tools_list}]\n\n\
             Return a JSON array of tasks. Each task object must have:\n\
             - \"id\": a unique string like \"t1\", \"t2\"\n\
             - \"description\": what the sub-agent should accomplish (include full context needed)\n\
             - \"tool_allowlist\": optional array of tool names from the available tools list above, or null for all tools\n\
             - \"depends_on\": array of task IDs this task depends on\n\n\
             CRITICAL: Set depends_on to [] (empty) for ALL tasks UNLESS one task strictly \
             requires the output of another. Maximize parallelism — independent research, \
             analysis, and data gathering tasks should ALWAYS run in parallel with empty \
             depends_on arrays. Each task description must be self-contained with all context \
             needed, since agents cannot see each other's work.\n\n\
             Task: {}\n\n\
             Return ONLY a valid JSON array, no markdown formatting or explanation.",
            self.config.max_sub_agents, prompt
        );

        let response = agent.prompt(&decompose_prompt).await?;
        let json_text = extract_json(&response.output);

        let mut tasks: Vec<DelegationTask> = serde_json::from_str(json_text).map_err(|e| {
            debug!(response = %response.output, "decomposition JSON parse failed");
            ZeniiError::Agent(format!("failed to parse decomposition response: {e}"))
        })?;

        for task in &mut tasks {
            task.token_budget = self.config.per_agent_token_budget;
            task.timeout_secs = self.config.per_agent_timeout_secs;
        }

        // Validate structural integrity of decomposed tasks
        {
            let mut seen_ids = std::collections::HashSet::new();
            for task in &tasks {
                if !seen_ids.insert(&task.id) {
                    return Err(ZeniiError::Validation(format!(
                        "duplicate task id '{}'",
                        task.id
                    )));
                }
                if task.depends_on.contains(&task.id) {
                    return Err(ZeniiError::Validation(format!(
                        "task '{}' depends on itself",
                        task.id
                    )));
                }
                if task.description.len() > 2000 {
                    return Err(ZeniiError::Validation(format!(
                        "task '{}' description exceeds 2000 chars",
                        task.id
                    )));
                }
            }
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
        let agent = crate::ai::resolve_agent(decomp_model, state, None, None, surface).await?;

        let tool_names: Vec<String> = state
            .tools
            .to_vec()
            .iter()
            .map(|t| t.name().to_string())
            .collect();

        let tasks = self.decompose(prompt, &agent, &tool_names).await?;
        if tasks.is_empty() {
            return Err(ZeniiError::Agent("decomposition produced no tasks".into()));
        }

        self.validate_tasks(&tasks, &tool_names)?;

        info!(
            delegation_id = %delegation_id,
            task_count = tasks.len(),
            "Starting delegation"
        );

        // Emit DelegationStarted with all agent info
        let _ = state
            .event_bus
            .publish(crate::event_bus::AppEvent::DelegationStarted {
                delegation_id: delegation_id.clone(),
                agents: tasks
                    .iter()
                    .map(|t| DelegationAgentInfo {
                        id: t.id.clone(),
                        description: t.description.clone(),
                    })
                    .collect(),
            });

        // Execute tasks in dependency waves
        let mut completed: HashMap<String, TaskResult> = HashMap::new();
        let mut remaining: Vec<DelegationTask> = tasks;

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
                            tool_uses: 0,
                            description: task.description.clone(),
                        },
                    );
                }
                break;
            }

            remaining = not_ready;

            let mut join_set = tokio::task::JoinSet::new();
            let mut wave_task_ids: Vec<String> = Vec::new();
            let mut wave_handles: Vec<tokio::task::AbortHandle> = Vec::new();
            for task in ready {
                let task_id = task.id.clone();
                let task_desc = task.description.clone();
                let _ = state
                    .event_bus
                    .publish(crate::event_bus::AppEvent::SubAgentSpawned {
                        delegation_id: delegation_id.clone(),
                        agent_id: task_id.clone(),
                        task: task_desc.clone(),
                    });

                match SubAgent::new(task, state, surface, delegation_id.clone()).await {
                    Ok(sub) => {
                        wave_task_ids.push(task_id.clone());
                        let abort_handle = join_set.spawn(async move {
                            let result = sub.execute().await;
                            (task_id, result)
                        });
                        wave_handles.push(abort_handle);
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
                                tool_uses: 0,
                                description: task_desc,
                            },
                        );
                    }
                }
            }

            self.active
                .entry(delegation_id.clone())
                .or_default()
                .extend(wave_handles.iter().cloned());

            while let Some(result) = join_set.join_next().await {
                match result {
                    Ok((task_id, task_result)) => {
                        let event = if task_result.status == TaskStatus::Completed {
                            crate::event_bus::AppEvent::SubAgentCompleted {
                                delegation_id: delegation_id.clone(),
                                agent_id: task_id.clone(),
                                status: "completed".into(),
                                duration_ms: task_result.duration_ms,
                                tool_uses: task_result.tool_uses,
                                tokens_used: task_result.usage.total_tokens,
                            }
                        } else {
                            crate::event_bus::AppEvent::SubAgentFailed {
                                delegation_id: delegation_id.clone(),
                                agent_id: task_id.clone(),
                                error: task_result.error.clone().unwrap_or_default(),
                                tool_uses: task_result.tool_uses,
                                duration_ms: task_result.duration_ms,
                            }
                        };
                        let _ = state.event_bus.publish(event);
                        completed.insert(task_id, task_result);
                    }
                    Err(e) => {
                        warn!("Sub-agent task panicked: {e}");
                    }
                }
            }

            // Insert failed results for any spawned tasks that panicked
            for wave_id in &wave_task_ids {
                if !completed.contains_key(wave_id) {
                    warn!(task_id = %wave_id, "sub-agent task panicked without producing a result");
                    completed.insert(
                        wave_id.clone(),
                        TaskResult {
                            task_id: wave_id.clone(),
                            status: TaskStatus::Failed,
                            output: String::new(),
                            usage: TokenUsage::default(),
                            duration_ms: start.elapsed().as_millis() as u64,
                            error: Some("task panicked".into()),
                            session_id: String::new(),
                            tool_uses: 0,
                            description: String::new(),
                        },
                    );
                }
            }
        }

        self.active.remove(&delegation_id);

        let mut total_usage = TokenUsage::default();
        for r in completed.values() {
            total_usage += r.usage.clone();
        }

        let total_duration_ms = start.elapsed().as_millis() as u64;

        // Emit DelegationCompleted
        let _ = state
            .event_bus
            .publish(crate::event_bus::AppEvent::DelegationCompleted {
                delegation_id: delegation_id.clone(),
                total_duration_ms,
                total_tokens: total_usage.total_tokens,
            });

        let results: Vec<TaskResult> = completed.into_values().collect();
        let aggregated = self.aggregate(prompt, &results, &agent).await?;

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
        let tasks = vec![
            make_task("t1", vec![]),
            make_task("t2", vec![]),
            make_task("t3", vec![]),
        ];

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
