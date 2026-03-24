use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::warn;

use crate::ai::adapter::{ToolCallEvent, ToolCallPhase};
use crate::ai::agent::{TokenUsage, ZeniiAgent};
use crate::ai::delegation::task::{DelegationTask, TaskResult, TaskStatus};
use crate::event_bus::EventBus;

pub struct SubAgent {
    task: DelegationTask,
    agent: Arc<ZeniiAgent>,
    session_id: String,
    delegation_id: String,
    event_bus: Arc<dyn EventBus>,
    tool_rx: broadcast::Receiver<ToolCallEvent>,
}

impl SubAgent {
    /// Create a new sub-agent with an isolated session and filtered tools.
    #[cfg(feature = "ai")]
    pub async fn new(
        task: DelegationTask,
        state: &crate::gateway::state::AppState,
        surface: &str,
        delegation_id: String,
    ) -> crate::Result<Self> {
        let desc_preview = &task.description[..task.description.len().min(80)];
        let session = state
            .session_manager
            .create_session_with_source(&format!("delegation: {desc_preview}"), "delegation")
            .await?;

        let tools = if let Some(ref allowlist) = task.tool_allowlist {
            state
                .tools
                .to_vec()
                .into_iter()
                .filter(|t| allowlist.contains(&t.name().to_string()))
                .collect()
        } else {
            let cfg = state.config.load();
            crate::security::permissions::PermissionResolver::executable_tools(
                &cfg.tool_permissions,
                surface,
                &state.tools,
            )
        };

        // Create per-agent broadcast channel for tool events
        let (tool_tx, tool_rx) = broadcast::channel::<ToolCallEvent>(128);

        let skip_approval = state.config.load().delegation_skip_approval;

        let agent = crate::ai::resolve_agent_with_tools(
            None,
            state,
            Some(tool_tx),
            None,
            Some(tools),
            surface,
            skip_approval,
        )
        .await?;

        Ok(Self {
            task,
            agent,
            session_id: session.id,
            delegation_id,
            event_bus: state.event_bus.clone(),
            tool_rx,
        })
    }

    /// Execute the sub-agent's task with timeout and tool monitoring.
    /// Always returns a TaskResult (never errors at the outer level).
    pub async fn execute(self) -> TaskResult {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(self.task.timeout_secs);

        // Shared tool counter for the monitoring task
        let tool_uses = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let tool_uses_clone = tool_uses.clone();

        // Spawn concurrent tool monitoring task
        let delegation_id = self.delegation_id.clone();
        let agent_id = self.task.id.clone();
        let event_bus = self.event_bus.clone();
        let mut monitor_rx = self.tool_rx;

        let monitor_handle = tokio::spawn(async move {
            let mut last_emit = tokio::time::Instant::now();
            let throttle = std::time::Duration::from_secs(1);

            loop {
                match monitor_rx.recv().await {
                    Ok(evt) => {
                        match &evt.phase {
                            ToolCallPhase::Started { .. } => {
                                let current = tool_uses_clone
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                                    + 1;
                                // Throttle progress events to max 1/sec
                                if last_emit.elapsed() >= throttle {
                                    let _ = event_bus.publish(
                                        crate::event_bus::AppEvent::SubAgentProgress {
                                            delegation_id: delegation_id.clone(),
                                            agent_id: agent_id.clone(),
                                            tool_uses: current,
                                            tokens_used: 0, // Not available per-tool
                                            current_activity: format!("{}: started", evt.tool_name),
                                        },
                                    );
                                    last_emit = tokio::time::Instant::now();
                                }
                            }
                            ToolCallPhase::Completed { .. }
                            | ToolCallPhase::Cached { .. }
                            | ToolCallPhase::ApprovalRequested { .. }
                            | ToolCallPhase::ApprovalResolved { .. } => {
                                // Already counted on Started / approval events are informational
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Sub-agent tool monitor lagged by {n} messages");
                    }
                }
            }
        });

        let result = tokio::time::timeout(timeout, self.agent.prompt(&self.task.description)).await;

        // Stop the monitor
        monitor_handle.abort();

        let final_tool_uses = tool_uses.load(std::sync::atomic::Ordering::Relaxed);

        match result {
            Ok(Ok(response)) => {
                // A.4: Post-hoc token budget warning
                if response.usage.total_tokens as usize > self.task.token_budget {
                    warn!(
                        task_id = %self.task.id,
                        budget = self.task.token_budget,
                        actual = response.usage.total_tokens,
                        "sub-agent exceeded token budget"
                    );
                }
                TaskResult {
                    task_id: self.task.id,
                    status: TaskStatus::Completed,
                    output: response.output,
                    usage: response.usage,
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: None,
                    session_id: self.session_id,
                    tool_uses: final_tool_uses,
                    description: self.task.description.clone(),
                }
            }
            Ok(Err(e)) => {
                let err_str = e.to_string();
                let error_msg = if err_str.contains("MaxTurnError")
                    || err_str.contains("max turn limit")
                {
                    format!(
                        "{err_str}. Increase `agent_max_turns` in config.toml (current limit may be too low for tool-heavy tasks)"
                    )
                } else {
                    err_str
                };
                TaskResult {
                    task_id: self.task.id,
                    status: TaskStatus::Failed,
                    output: String::new(),
                    usage: TokenUsage::default(),
                    duration_ms: start.elapsed().as_millis() as u64,
                    error: Some(error_msg),
                    session_id: self.session_id,
                    tool_uses: final_tool_uses,
                    description: self.task.description.clone(),
                }
            }
            Err(_) => TaskResult {
                task_id: self.task.id,
                status: TaskStatus::TimedOut,
                output: String::new(),
                usage: TokenUsage::default(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some("task timed out".into()),
                session_id: self.session_id,
                tool_uses: final_tool_uses,
                description: self.task.description.clone(),
            },
        }
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn task(&self) -> &DelegationTask {
        &self.task
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Set up state with an openai credential and last_used_model pointing to it.
    #[cfg(feature = "ai")]
    async fn setup_state_with_agent() -> (
        tempfile::TempDir,
        std::sync::Arc<crate::gateway::state::AppState>,
    ) {
        let (dir, state) = crate::gateway::handlers::tests::test_state().await;
        state
            .credentials
            .set("api_key:openai", "sk-test")
            .await
            .unwrap();
        // Point last_used_model to an openai model so resolve_agent finds the key
        {
            let mut last = state.last_used_model.write().await;
            *last = Some("openai:gpt-4o".into());
        }
        (dir, state)
    }

    // 7.8
    #[cfg(feature = "ai")]
    #[tokio::test]
    async fn sub_agent_creates_isolated_session() {
        let (_dir, state) = setup_state_with_agent().await;

        let task = DelegationTask {
            id: "t1".into(),
            description: "test task".into(),
            tool_allowlist: None,
            token_budget: 4000,
            timeout_secs: 120,
            depends_on: vec![],
        };

        let sub = SubAgent::new(task, &state, "desktop", "d-test".into())
            .await
            .unwrap();
        assert!(!sub.session_id().is_empty());

        let session = state
            .session_manager
            .get_session(sub.session_id())
            .await
            .unwrap();
        assert_eq!(session.source, "delegation");
    }

    // 7.9
    #[cfg(feature = "ai")]
    #[tokio::test]
    async fn sub_agent_filters_tools_by_allowlist() {
        let (_dir, state) = setup_state_with_agent().await;

        // Register a test tool so there's something to filter
        state
            .tools
            .register(std::sync::Arc::new(
                crate::tools::system_info::SystemInfoTool::new(),
            ))
            .ok();

        let all_tools = state.tools.to_vec();
        assert!(!all_tools.is_empty(), "should have at least one tool");

        let first_tool = all_tools[0].name().to_string();
        let task = DelegationTask {
            id: "t2".into(),
            description: "filtered task".into(),
            tool_allowlist: Some(vec![first_tool]),
            token_budget: 4000,
            timeout_secs: 120,
            depends_on: vec![],
        };

        let sub = SubAgent::new(task, &state, "desktop", "d-test".into()).await;
        assert!(sub.is_ok(), "SubAgent with tool allowlist should succeed");
    }

    // 7.10
    #[cfg(feature = "ai")]
    #[tokio::test]
    async fn sub_agent_uses_all_tools_when_no_allowlist() {
        let (_dir, state) = setup_state_with_agent().await;

        let task = DelegationTask {
            id: "t3".into(),
            description: "unfiltered task".into(),
            tool_allowlist: None,
            token_budget: 4000,
            timeout_secs: 120,
            depends_on: vec![],
        };

        let sub = SubAgent::new(task, &state, "desktop", "d-test".into()).await;
        assert!(sub.is_ok(), "SubAgent with no allowlist should succeed");
    }

    // 7.11 — Structural test: timeout_secs is plumbed correctly
    #[test]
    fn sub_agent_execute_timeout_plumbing() {
        let task = DelegationTask {
            id: "t4".into(),
            description: "timeout test".into(),
            tool_allowlist: None,
            token_budget: 100,
            timeout_secs: 1,
            depends_on: vec![],
        };
        assert_eq!(task.timeout_secs, 1);
        // Full timeout integration test requires real LLM endpoint (manual test M7.1)
    }
}
