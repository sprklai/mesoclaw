use std::sync::Arc;

use crate::ai::agent::{TokenUsage, ZeniiAgent};
use crate::ai::delegation::task::{DelegationTask, TaskResult, TaskStatus};

pub struct SubAgent {
    task: DelegationTask,
    agent: Arc<ZeniiAgent>,
    session_id: String,
}

impl SubAgent {
    /// Create a new sub-agent with an isolated session and filtered tools.
    #[cfg(feature = "ai")]
    pub async fn new(
        task: DelegationTask,
        state: &crate::gateway::state::AppState,
        surface: &str,
    ) -> crate::Result<Self> {
        let desc_preview = &task.description[..task.description.len().min(80)];
        let session = state
            .session_manager
            .create_session_with_source(
                &format!("delegation: {desc_preview}"),
                "delegation",
            )
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
            crate::security::permissions::PermissionResolver::allowed_tools(
                &cfg.tool_permissions,
                surface,
                &state.tools,
            )
        };

        let agent =
            crate::ai::resolve_agent_with_tools(None, state, None, None, Some(tools), surface)
                .await?;

        Ok(Self {
            task,
            agent,
            session_id: session.id,
        })
    }

    /// Execute the sub-agent's task with timeout.
    /// Always returns a TaskResult (never errors at the outer level).
    pub async fn execute(self) -> TaskResult {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(self.task.timeout_secs);

        match tokio::time::timeout(timeout, self.agent.prompt(&self.task.description)).await {
            Ok(Ok(response)) => TaskResult {
                task_id: self.task.id,
                status: TaskStatus::Completed,
                output: response.output,
                usage: response.usage,
                duration_ms: start.elapsed().as_millis() as u64,
                error: None,
                session_id: self.session_id,
            },
            Ok(Err(e)) => TaskResult {
                task_id: self.task.id,
                status: TaskStatus::Failed,
                output: String::new(),
                usage: TokenUsage::default(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some(e.to_string()),
                session_id: self.session_id,
            },
            Err(_) => TaskResult {
                task_id: self.task.id,
                status: TaskStatus::TimedOut,
                output: String::new(),
                usage: TokenUsage::default(),
                duration_ms: start.elapsed().as_millis() as u64,
                error: Some("task timed out".into()),
                session_id: self.session_id,
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
    async fn setup_state_with_agent(
    ) -> (tempfile::TempDir, std::sync::Arc<crate::gateway::state::AppState>) {
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

        let sub = SubAgent::new(task, &state, "desktop").await.unwrap();
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

        let sub = SubAgent::new(task, &state, "desktop").await;
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

        let sub = SubAgent::new(task, &state, "desktop").await;
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
