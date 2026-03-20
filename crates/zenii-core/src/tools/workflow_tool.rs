use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::event_bus::EventBus;
use crate::tools::ToolRegistry;
use crate::workflows::executor::WorkflowExecutor;
use crate::workflows::{Workflow, WorkflowRegistry, WorkflowStep};
use crate::{Result, ZeniiError};

use super::traits::{Tool, ToolResult};

/// Agent tool for managing multi-step workflows (create/list/get/run/delete/history).
pub struct WorkflowTool {
    registry: Arc<WorkflowRegistry>,
    executor: Arc<WorkflowExecutor>,
    tools: Arc<ToolRegistry>,
    event_bus: Arc<dyn EventBus>,
    #[cfg(feature = "scheduler")]
    scheduler: Option<Arc<crate::scheduler::TokioScheduler>>,
}

impl WorkflowTool {
    pub fn new(
        registry: Arc<WorkflowRegistry>,
        executor: Arc<WorkflowExecutor>,
        tools: Arc<ToolRegistry>,
        event_bus: Arc<dyn EventBus>,
        #[cfg(feature = "scheduler")] scheduler: Option<Arc<crate::scheduler::TokioScheduler>>,
    ) -> Self {
        Self {
            registry,
            executor,
            tools,
            event_bus,
            #[cfg(feature = "scheduler")]
            scheduler,
        }
    }
}

#[async_trait]
impl Tool for WorkflowTool {
    fn name(&self) -> &str {
        "workflows"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::Medium
    }

    fn description(&self) -> &str {
        "Create, run, list, or manage multi-step workflows. Use this when a task requires \
         chaining multiple actions together — e.g., search the web THEN summarize THEN send \
         via channel. Each workflow is a DAG of steps (tool calls, LLM prompts, conditions, \
         delays) that execute in dependency order with retry and failure policies. If the \
         workflow should run on a schedule, set the 'schedule' field (cron expression) and a \
         scheduler job will be created automatically. For simple single-action timers with no \
         multi-step logic, use the scheduler tool instead."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "list", "get", "run", "delete", "history"],
                    "description": "The workflow operation to perform"
                },
                "workflow_id": {
                    "type": "string",
                    "description": "Workflow ID (required for get/run/delete/history)"
                },
                "name": {
                    "type": "string",
                    "description": "Workflow name (required for create)"
                },
                "description": {
                    "type": "string",
                    "description": "Workflow description (optional for create)"
                },
                "steps": {
                    "type": "array",
                    "description": "Array of workflow steps (required for create). Each step has: name, type (tool/llm/delay/condition/parallel), and type-specific fields. Use depends_on to chain steps.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "type": {
                                "type": "string",
                                "enum": ["tool", "llm", "delay", "condition", "parallel"]
                            },
                            "tool": { "type": "string", "description": "Tool name (for type=tool)" },
                            "args": { "type": "object", "description": "Tool arguments (for type=tool)" },
                            "prompt": { "type": "string", "description": "LLM prompt (for type=llm). Use {{steps.<name>.output}} for interpolation." },
                            "model": { "type": "string", "description": "Model override (for type=llm)" },
                            "seconds": { "type": "integer", "description": "Delay seconds (for type=delay)" },
                            "expression": { "type": "string", "description": "Condition expression (for type=condition)" },
                            "if_true": { "type": "string", "description": "Step to run if true (for type=condition)" },
                            "if_false": { "type": "string", "description": "Step to run if false (for type=condition)" },
                            "steps": { "type": "array", "items": { "type": "string" }, "description": "Step names to run in parallel (for type=parallel)" },
                            "depends_on": { "type": "array", "items": { "type": "string" }, "description": "Steps this depends on" },
                            "timeout_secs": { "type": "integer", "description": "Step timeout in seconds" }
                        },
                        "required": ["name", "type"]
                    }
                },
                "schedule": {
                    "type": "string",
                    "description": "Cron expression for automatic scheduling (optional for create). When set, a scheduler job is created to trigger this workflow."
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| ZeniiError::Validation("missing 'action' field".into()))?;

        match action {
            "create" => self.create_workflow(&args).await,
            "list" => self.list_workflows().await,
            "get" => {
                let id = args["workflow_id"]
                    .as_str()
                    .ok_or_else(|| ZeniiError::Validation("missing 'workflow_id' for get".into()))?;
                match self.registry.get(id) {
                    Some(wf) => {
                        let json = serde_json::to_string_pretty(&wf).unwrap_or_default();
                        Ok(ToolResult::ok(json))
                    }
                    None => Ok(ToolResult::err(format!("Workflow '{id}' not found"))),
                }
            }
            "run" => self.run_workflow(&args).await,
            "delete" => {
                let id = args["workflow_id"].as_str().ok_or_else(|| {
                    ZeniiError::Validation("missing 'workflow_id' for delete".into())
                })?;
                match self.registry.delete(id) {
                    Ok(true) => Ok(ToolResult::ok(format!("Workflow '{id}' deleted"))),
                    Ok(false) => Ok(ToolResult::err(format!("Workflow '{id}' not found"))),
                    Err(e) => Ok(ToolResult::err(format!("Failed to delete workflow: {e}"))),
                }
            }
            "history" => {
                let id = args["workflow_id"].as_str().ok_or_else(|| {
                    ZeniiError::Validation("missing 'workflow_id' for history".into())
                })?;
                match self.executor.get_history(id).await {
                    Ok(runs) => {
                        let json = serde_json::to_string_pretty(&runs).unwrap_or_default();
                        Ok(ToolResult::ok(json))
                    }
                    Err(e) => Ok(ToolResult::err(format!(
                        "Failed to get workflow history: {e}"
                    ))),
                }
            }
            other => Ok(ToolResult::err(format!(
                "Unknown action '{other}'. Valid actions: create, list, get, run, delete, history"
            ))),
        }
    }
}

impl WorkflowTool {
    async fn create_workflow(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| ZeniiError::Validation("missing 'name' for create".into()))?;
        let description = args["description"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let schedule = args["schedule"].as_str().map(|s| s.to_string());

        let steps_json = args["steps"]
            .as_array()
            .ok_or_else(|| ZeniiError::Validation("missing 'steps' array for create".into()))?;

        if steps_json.is_empty() {
            return Ok(ToolResult::err("Workflow must have at least one step"));
        }

        let mut steps = Vec::with_capacity(steps_json.len());
        for step_json in steps_json {
            match self.parse_step(step_json) {
                Ok(step) => steps.push(step),
                Err(e) => return Ok(ToolResult::err(format!("Invalid step: {e}"))),
            }
        }

        // Validate DAG before saving
        if let Err(e) = WorkflowExecutor::build_dag(&steps) {
            return Ok(ToolResult::err(format!("Invalid workflow DAG: {e}")));
        }

        let workflow_id: String = name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");

        // Check for duplicate
        if self.registry.get(&workflow_id).is_some() {
            return Ok(ToolResult::err(format!(
                "Workflow '{workflow_id}' already exists"
            )));
        }

        let now = chrono::Utc::now().to_rfc3339();
        let workflow = Workflow {
            id: workflow_id.clone(),
            name: name.to_string(),
            description,
            schedule: schedule.clone(),
            steps,
            created_at: now.clone(),
            updated_at: now,
        };

        if let Err(e) = self.registry.save(workflow) {
            return Ok(ToolResult::err(format!("Failed to save workflow: {e}")));
        }

        // If schedule is provided, create a scheduler job to trigger this workflow
        #[cfg(feature = "scheduler")]
        if let Some(ref cron_expr) = schedule
            && let Some(ref sched) = self.scheduler
        {
            use crate::scheduler::traits::{JobPayload, Schedule, ScheduledJob, Scheduler};

            let job = ScheduledJob {
                id: uuid::Uuid::new_v4().to_string(),
                name: format!("workflow:{workflow_id}"),
                schedule: Schedule::Cron {
                    expr: cron_expr.clone(),
                },
                session_target: Default::default(),
                payload: JobPayload::Workflow {
                    workflow_id: workflow_id.clone(),
                },
                enabled: true,
                error_count: 0,
                next_run: None,
                active_hours: None,
                delete_after_run: false,
            };
            match sched.add_job(job).await {
                Ok(job_id) => {
                    return Ok(ToolResult::ok(format!(
                        "Workflow '{workflow_id}' created and scheduled (job ID: {job_id})"
                    )));
                }
                Err(e) => {
                    return Ok(ToolResult::ok(format!(
                        "Workflow '{workflow_id}' created but scheduling failed: {e}"
                    )));
                }
            }
        }

        Ok(ToolResult::ok(format!("Workflow '{workflow_id}' created")))
    }

    fn parse_step(&self, json: &serde_json::Value) -> Result<WorkflowStep> {
        let name = json["name"]
            .as_str()
            .ok_or_else(|| ZeniiError::Validation("step missing 'name'".into()))?
            .to_string();
        let step_type_str = json["type"]
            .as_str()
            .ok_or_else(|| ZeniiError::Validation("step missing 'type'".into()))?;

        let step_type = match step_type_str {
            "tool" => {
                let tool = json["tool"]
                    .as_str()
                    .ok_or_else(|| {
                        ZeniiError::Validation("tool step missing 'tool' field".into())
                    })?
                    .to_string();
                let args = json.get("args").cloned().unwrap_or(json!({}));
                crate::workflows::StepType::Tool { tool, args }
            }
            "llm" => {
                let prompt = json["prompt"]
                    .as_str()
                    .ok_or_else(|| {
                        ZeniiError::Validation("llm step missing 'prompt' field".into())
                    })?
                    .to_string();
                let model = json["model"].as_str().map(|s| s.to_string());
                crate::workflows::StepType::Llm { prompt, model }
            }
            "delay" => {
                let seconds = json["seconds"].as_u64().ok_or_else(|| {
                    ZeniiError::Validation("delay step missing 'seconds' field".into())
                })?;
                crate::workflows::StepType::Delay { seconds }
            }
            "condition" => {
                let expression = json["expression"]
                    .as_str()
                    .ok_or_else(|| {
                        ZeniiError::Validation("condition step missing 'expression'".into())
                    })?
                    .to_string();
                let if_true = json["if_true"]
                    .as_str()
                    .ok_or_else(|| {
                        ZeniiError::Validation("condition step missing 'if_true'".into())
                    })?
                    .to_string();
                let if_false = json["if_false"].as_str().map(|s| s.to_string());
                crate::workflows::StepType::Condition {
                    expression,
                    if_true,
                    if_false,
                }
            }
            "parallel" => {
                let step_names = json["steps"]
                    .as_array()
                    .ok_or_else(|| {
                        ZeniiError::Validation("parallel step missing 'steps' array".into())
                    })?
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
                crate::workflows::StepType::Parallel { steps: step_names }
            }
            other => {
                return Err(ZeniiError::Validation(format!(
                    "unknown step type '{other}'. Valid: tool, llm, delay, condition, parallel"
                )));
            }
        };

        let depends_on = json
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let timeout_secs = json["timeout_secs"].as_u64();

        Ok(WorkflowStep {
            name,
            step_type,
            depends_on,
            retry: None,
            failure_policy: crate::workflows::FailurePolicy::Stop,
            timeout_secs,
        })
    }

    async fn list_workflows(&self) -> Result<ToolResult> {
        let workflows = self.registry.list();
        if workflows.is_empty() {
            return Ok(ToolResult::ok("No workflows found"));
        }

        let summary: Vec<serde_json::Value> = workflows
            .iter()
            .map(|wf| {
                json!({
                    "id": wf.id,
                    "name": wf.name,
                    "description": wf.description,
                    "steps": wf.steps.len(),
                    "schedule": wf.schedule,
                })
            })
            .collect();
        let json = serde_json::to_string_pretty(&summary).unwrap_or_default();
        Ok(ToolResult::ok(json))
    }

    async fn run_workflow(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let id = args["workflow_id"]
            .as_str()
            .ok_or_else(|| ZeniiError::Validation("missing 'workflow_id' for run".into()))?;

        let workflow = match self.registry.get(id) {
            Some(wf) => wf,
            None => return Ok(ToolResult::err(format!("Workflow '{id}' not found"))),
        };

        match self
            .executor
            .execute(&workflow, &self.tools, self.event_bus.as_ref())
            .await
        {
            Ok(run) => {
                let json = serde_json::to_string_pretty(&run).unwrap_or_default();
                Ok(ToolResult::ok(json))
            }
            Err(e) => Ok(ToolResult::err(format!("Workflow execution failed: {e}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::db;
    use crate::event_bus::TokioBroadcastBus;

    async fn setup() -> (tempfile::TempDir, WorkflowTool) {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = AppConfig::default();
        let event_bus: Arc<dyn EventBus> = Arc::new(TokioBroadcastBus::new(256));

        let wf_dir = dir.path().join("workflows");
        let registry = Arc::new(WorkflowRegistry::new(wf_dir).unwrap());
        let executor = Arc::new(WorkflowExecutor::new(
            pool.clone(),
            config.workflow_max_steps,
            config.workflow_step_timeout_secs,
            config.workflow_step_max_retries,
        ));
        let tools = Arc::new(ToolRegistry::new());
        tools
            .register(Arc::new(crate::tools::system_info::SystemInfoTool::new()))
            .unwrap();

        let tool = WorkflowTool::new(
            registry,
            executor,
            tools,
            event_bus,
            #[cfg(feature = "scheduler")]
            None,
        );
        (dir, tool)
    }

    // WT.1 — Create workflow returns success
    #[tokio::test]
    async fn workflow_tool_create() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "create",
                "name": "test workflow",
                "description": "A test workflow",
                "steps": [
                    {
                        "name": "wait",
                        "type": "delay",
                        "seconds": 1
                    }
                ]
            }))
            .await
            .unwrap();

        assert!(result.success, "output: {}", result.output);
        assert!(result.output.contains("test-workflow"));
        assert!(result.output.contains("created"));
    }

    // WT.2 — List workflows returns created workflows
    #[tokio::test]
    async fn workflow_tool_list() {
        let (_dir, tool) = setup().await;

        tool.execute(json!({
            "action": "create",
            "name": "list test",
            "steps": [{ "name": "s1", "type": "delay", "seconds": 0 }]
        }))
        .await
        .unwrap();

        let result = tool
            .execute(json!({ "action": "list" }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("list-test"));
    }

    // WT.3 — Get workflow returns details
    #[tokio::test]
    async fn workflow_tool_get() {
        let (_dir, tool) = setup().await;

        tool.execute(json!({
            "action": "create",
            "name": "get test",
            "steps": [{ "name": "s1", "type": "delay", "seconds": 0 }]
        }))
        .await
        .unwrap();

        let result = tool
            .execute(json!({ "action": "get", "workflow_id": "get-test" }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("get-test"));
        assert!(result.output.contains("s1"));
    }

    // WT.4 — Run workflow executes and returns result
    #[tokio::test]
    async fn workflow_tool_run() {
        let (_dir, tool) = setup().await;

        tool.execute(json!({
            "action": "create",
            "name": "run test",
            "steps": [{ "name": "wait", "type": "delay", "seconds": 0 }]
        }))
        .await
        .unwrap();

        let result = tool
            .execute(json!({ "action": "run", "workflow_id": "run-test" }))
            .await
            .unwrap();
        assert!(result.success, "output: {}", result.output);
        assert!(result.output.contains("Completed") || result.output.contains("completed"));
    }

    // WT.5 — Delete workflow removes it
    #[tokio::test]
    async fn workflow_tool_delete() {
        let (_dir, tool) = setup().await;

        tool.execute(json!({
            "action": "create",
            "name": "delete me",
            "steps": [{ "name": "s1", "type": "delay", "seconds": 0 }]
        }))
        .await
        .unwrap();

        let result = tool
            .execute(json!({ "action": "delete", "workflow_id": "delete-me" }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("deleted"));

        // Verify it's gone
        let get_result = tool
            .execute(json!({ "action": "get", "workflow_id": "delete-me" }))
            .await
            .unwrap();
        assert!(!get_result.success);
        assert!(get_result.output.contains("not found"));
    }

    // WT.6 — History returns execution records
    #[tokio::test]
    async fn workflow_tool_history() {
        let (_dir, tool) = setup().await;

        tool.execute(json!({
            "action": "create",
            "name": "history test",
            "steps": [{ "name": "s1", "type": "delay", "seconds": 0 }]
        }))
        .await
        .unwrap();

        // Run it first to create history
        tool.execute(json!({ "action": "run", "workflow_id": "history-test" }))
            .await
            .unwrap();

        let result = tool
            .execute(json!({ "action": "history", "workflow_id": "history-test" }))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("history-test"));
    }

    // WT.7 — Invalid action returns error
    #[tokio::test]
    async fn workflow_tool_invalid_action() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({ "action": "invalid" }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("Unknown action"));
    }

    // WT.8 — Create with missing steps returns error
    #[tokio::test]
    async fn workflow_tool_create_missing_steps() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "create",
                "name": "no-steps"
            }))
            .await;
        assert!(result.is_err());
    }

    // WT.9 — Create with empty steps returns error
    #[tokio::test]
    async fn workflow_tool_create_empty_steps() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "create",
                "name": "empty",
                "steps": []
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("at least one step"));
    }

    // WT.10 — Create duplicate returns error
    #[tokio::test]
    async fn workflow_tool_create_duplicate() {
        let (_dir, tool) = setup().await;
        let args = json!({
            "action": "create",
            "name": "dup workflow",
            "steps": [{ "name": "s1", "type": "delay", "seconds": 0 }]
        });
        let first = tool.execute(args.clone()).await.unwrap();
        assert!(first.success);

        let second = tool.execute(args).await.unwrap();
        assert!(!second.success);
        assert!(second.output.contains("already exists"));
    }

    // WT.11 — Tool schema validation
    #[tokio::test]
    async fn workflow_tool_schema() {
        let (_dir, tool) = setup().await;
        assert_eq!(tool.name(), "workflows");
        assert!(tool.description().contains("multi-step"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("action")));
    }

    // WT.12 — Run nonexistent workflow returns error
    #[tokio::test]
    async fn workflow_tool_run_nonexistent() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({ "action": "run", "workflow_id": "nope" }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("not found"));
    }

    // WT.13 — Create multi-step workflow with dependencies
    #[tokio::test]
    async fn workflow_tool_create_multi_step() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "create",
                "name": "multi step",
                "steps": [
                    {
                        "name": "fetch",
                        "type": "tool",
                        "tool": "system_info",
                        "args": { "action": "os" }
                    },
                    {
                        "name": "summarize",
                        "type": "llm",
                        "prompt": "Summarize: {{steps.fetch.output}}",
                        "depends_on": ["fetch"]
                    }
                ]
            }))
            .await
            .unwrap();
        assert!(result.success, "output: {}", result.output);
    }

    // WT.14 — Create with cyclic dependencies returns error
    #[tokio::test]
    async fn workflow_tool_create_cyclic() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "create",
                "name": "cyclic",
                "steps": [
                    { "name": "a", "type": "delay", "seconds": 1, "depends_on": ["b"] },
                    { "name": "b", "type": "delay", "seconds": 1, "depends_on": ["a"] }
                ]
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("cyclic"));
    }
}
