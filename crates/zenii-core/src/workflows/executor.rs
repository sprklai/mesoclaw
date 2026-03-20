use std::collections::HashMap;

use petgraph::algo::toposort;
use petgraph::graph::DiGraph;
use tracing::info;

use crate::db::{self, DbPool};
use crate::{Result, ZeniiError};

use super::definition::*;

pub struct WorkflowExecutor {
    db: DbPool,
    max_steps: usize,
    step_timeout_secs: u64,
    step_max_retries: u32,
}

impl WorkflowExecutor {
    pub fn new(db: DbPool, max_steps: usize, step_timeout_secs: u64, step_max_retries: u32) -> Self {
        Self {
            db,
            max_steps,
            step_timeout_secs,
            step_max_retries,
        }
    }

    /// Build a petgraph DAG from workflow steps. Returns (graph, node_index_by_name).
    pub fn build_dag(
        steps: &[WorkflowStep],
    ) -> Result<(
        DiGraph<usize, ()>,
        HashMap<String, petgraph::graph::NodeIndex>,
    )> {
        if steps.is_empty() {
            return Err(ZeniiError::Workflow("workflow has no steps".into()));
        }
        let mut graph = DiGraph::new();
        let mut indices = HashMap::new();

        // Add nodes
        for (i, step) in steps.iter().enumerate() {
            let idx = graph.add_node(i);
            indices.insert(step.name.clone(), idx);
        }

        // Add edges (dependency -> step)
        for step in steps {
            if let Some(&step_idx) = indices.get(&step.name) {
                for dep in &step.depends_on {
                    let dep_idx = indices.get(dep).ok_or_else(|| {
                        ZeniiError::Workflow(format!(
                            "step '{}' depends on unknown step '{}'",
                            step.name, dep
                        ))
                    })?;
                    graph.add_edge(*dep_idx, step_idx, ());
                }
            }
        }

        // Validate acyclic
        toposort(&graph, None).map_err(|_| {
            ZeniiError::Workflow("workflow contains cyclic dependencies".into())
        })?;

        Ok((graph, indices))
    }

    /// Execute a workflow, persisting the run to DB.
    pub async fn execute(
        &self,
        workflow: &Workflow,
        tools: &crate::tools::ToolRegistry,
        event_bus: &dyn crate::event_bus::EventBus,
    ) -> Result<WorkflowRun> {
        // Validate step count
        if workflow.steps.len() > self.max_steps {
            return Err(ZeniiError::Workflow(format!(
                "workflow has {} steps (max {})",
                workflow.steps.len(),
                self.max_steps
            )));
        }

        let (graph, _indices) = Self::build_dag(&workflow.steps)?;
        let topo = toposort(&graph, None).map_err(|_| {
            ZeniiError::Workflow("workflow contains cyclic dependencies".into())
        })?;

        let run_id = uuid::Uuid::new_v4().to_string();
        let started_at = chrono::Utc::now().to_rfc3339();

        // Persist run start
        self.persist_run_start(&run_id, &workflow.id, &workflow.name, &started_at)
            .await?;

        // Publish start event
        let _ = event_bus.publish(crate::event_bus::AppEvent::WorkflowStarted {
            workflow_id: workflow.id.clone(),
            run_id: run_id.clone(),
        });

        info!(
            workflow_id = %workflow.id,
            run_id = %run_id,
            "workflow execution started"
        );

        let mut step_outputs: HashMap<String, StepOutput> = HashMap::new();
        let mut overall_status = WorkflowRunStatus::Completed;
        let mut overall_error = None;

        // Create name->step lookup
        let step_map: HashMap<String, &WorkflowStep> =
            workflow.steps.iter().map(|s| (s.name.clone(), s)).collect();

        // Execute in topological order
        let remaining_steps: Vec<&WorkflowStep> = topo
            .iter()
            .map(|&idx| {
                let step_idx = graph[idx];
                &workflow.steps[step_idx]
            })
            .collect();

        for step in remaining_steps {
            let output = self
                .execute_step(step, &step_outputs, tools, &step_map)
                .await;

            let output = match output {
                Ok(out) => out,
                Err(e) => StepOutput {
                    step_name: step.name.clone(),
                    output: String::new(),
                    success: false,
                    duration_ms: 0,
                    error: Some(e.to_string()),
                },
            };

            // Publish step event
            let _ = event_bus.publish(crate::event_bus::AppEvent::WorkflowStepCompleted {
                workflow_id: workflow.id.clone(),
                run_id: run_id.clone(),
                step_name: step.name.clone(),
                success: output.success,
            });

            // Persist step result
            self.persist_step_result(&run_id, &output).await.ok();

            let step_failed = !output.success;
            step_outputs.insert(step.name.clone(), output);

            if step_failed {
                match &step.failure_policy {
                    FailurePolicy::Stop => {
                        overall_status = WorkflowRunStatus::Failed;
                        overall_error = Some(format!("step '{}' failed", step.name));
                        break;
                    }
                    FailurePolicy::Continue => {
                        // Continue to next step
                    }
                    FailurePolicy::Fallback {
                        step: fallback_name,
                    } => {
                        if let Some(fallback_step) = step_map.get(fallback_name) {
                            let fb_result = self
                                .execute_step(fallback_step, &step_outputs, tools, &step_map)
                                .await;
                            let fb_output = match fb_result {
                                Ok(out) => out,
                                Err(e) => StepOutput {
                                    step_name: fallback_name.clone(),
                                    output: String::new(),
                                    success: false,
                                    duration_ms: 0,
                                    error: Some(e.to_string()),
                                },
                            };
                            step_outputs.insert(fallback_name.clone(), fb_output);
                        }
                    }
                }
            }
        }

        let completed_at = chrono::Utc::now().to_rfc3339();
        let status_str = match overall_status {
            WorkflowRunStatus::Completed => "completed",
            WorkflowRunStatus::Failed => "failed",
            _ => "failed",
        };

        self.persist_run_end(&run_id, status_str, overall_error.as_deref(), &completed_at)
            .await?;

        let _ = event_bus.publish(crate::event_bus::AppEvent::WorkflowCompleted {
            workflow_id: workflow.id.clone(),
            run_id: run_id.clone(),
            status: status_str.into(),
        });

        info!(
            workflow_id = %workflow.id,
            run_id = %run_id,
            status = %status_str,
            "workflow execution finished"
        );

        Ok(WorkflowRun {
            id: run_id,
            workflow_id: workflow.id.clone(),
            status: overall_status,
            step_results: step_outputs.into_values().collect(),
            started_at,
            completed_at: Some(completed_at),
            error: overall_error,
        })
    }

    async fn execute_step(
        &self,
        step: &WorkflowStep,
        step_outputs: &HashMap<String, StepOutput>,
        tools: &crate::tools::ToolRegistry,
        _step_map: &HashMap<String, &WorkflowStep>,
    ) -> Result<StepOutput> {
        let timeout = step.timeout_secs.unwrap_or(self.step_timeout_secs);
        let max_retries = step
            .retry
            .as_ref()
            .map(|r| r.max_retries)
            .unwrap_or(self.step_max_retries);
        let retry_delay = step
            .retry
            .as_ref()
            .map(|r| r.retry_delay_ms)
            .unwrap_or(1000);

        #[allow(unused_assignments)]
        let mut last_err: Option<String> = None;
        for attempt in 0..=max_retries {
            if attempt > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(retry_delay)).await;
            }

            let start = std::time::Instant::now();
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(timeout),
                super::runtime::dispatch_step(&step.step_type, step_outputs, tools),
            )
            .await;
            let elapsed_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(Ok(output)) => {
                    return Ok(StepOutput {
                        step_name: step.name.clone(),
                        output,
                        success: true,
                        duration_ms: elapsed_ms,
                        error: None,
                    });
                }
                Ok(Err(e)) => {
                    last_err = Some(e.to_string());
                    if attempt == max_retries {
                        return Ok(StepOutput {
                            step_name: step.name.clone(),
                            output: String::new(),
                            success: false,
                            duration_ms: elapsed_ms,
                            error: last_err,
                        });
                    }
                }
                Err(_) => {
                    return Ok(StepOutput {
                        step_name: step.name.clone(),
                        output: String::new(),
                        success: false,
                        duration_ms: elapsed_ms,
                        error: Some("step timed out".into()),
                    });
                }
            }
        }
        unreachable!()
    }

    async fn persist_run_start(
        &self,
        run_id: &str,
        workflow_id: &str,
        workflow_name: &str,
        started_at: &str,
    ) -> Result<()> {
        let rid = run_id.to_string();
        let wid = workflow_id.to_string();
        let wname = workflow_name.to_string();
        let sat = started_at.to_string();
        db::with_db(&self.db, move |conn| {
            conn.execute(
                "INSERT INTO workflow_runs (id, workflow_id, workflow_name, status, started_at) VALUES (?1, ?2, ?3, 'running', ?4)",
                rusqlite::params![rid, wid, wname, sat],
            )?;
            Ok(())
        })
        .await
    }

    async fn persist_step_result(&self, run_id: &str, output: &StepOutput) -> Result<()> {
        let rid = run_id.to_string();
        let step_id = uuid::Uuid::new_v4().to_string();
        let step_name = output.step_name.clone();
        let out = output.output.clone();
        let success = output.success;
        let duration = output.duration_ms as i64;
        let error = output.error.clone();
        db::with_db(&self.db, move |conn| {
            conn.execute(
                "INSERT INTO workflow_step_results (id, run_id, step_name, output, success, duration_ms, error) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![step_id, rid, step_name, out, success, duration, error],
            )?;
            Ok(())
        })
        .await
    }

    async fn persist_run_end(
        &self,
        run_id: &str,
        status: &str,
        error: Option<&str>,
        completed_at: &str,
    ) -> Result<()> {
        let rid = run_id.to_string();
        let st = status.to_string();
        let err = error.map(|s| s.to_string());
        let cat = completed_at.to_string();
        db::with_db(&self.db, move |conn| {
            conn.execute(
                "UPDATE workflow_runs SET status = ?1, error = ?2, completed_at = ?3 WHERE id = ?4",
                rusqlite::params![st, err, cat, rid],
            )?;
            Ok(())
        })
        .await
    }

    /// Get run history for a workflow.
    pub async fn get_history(&self, workflow_id: &str) -> Result<Vec<WorkflowRun>> {
        let wid = workflow_id.to_string();
        db::with_db(&self.db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, workflow_id, status, started_at, completed_at, error FROM workflow_runs WHERE workflow_id = ?1 ORDER BY started_at DESC",
            )?;
            let runs = stmt
                .query_map(rusqlite::params![wid], |row| {
                    Ok(WorkflowRun {
                        id: row.get(0)?,
                        workflow_id: row.get(1)?,
                        status: match row.get::<_, String>(2)?.as_str() {
                            "completed" => WorkflowRunStatus::Completed,
                            "failed" => WorkflowRunStatus::Failed,
                            "cancelled" => WorkflowRunStatus::Cancelled,
                            _ => WorkflowRunStatus::Running,
                        },
                        step_results: vec![], // Loaded separately if needed
                        started_at: row.get(3)?,
                        completed_at: row.get(4)?,
                        error: row.get(5)?,
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(runs)
        })
        .await
    }

    /// Get a specific run with step results.
    pub async fn get_run(&self, run_id: &str) -> Result<Option<WorkflowRun>> {
        let rid = run_id.to_string();
        db::with_db(&self.db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, workflow_id, status, started_at, completed_at, error FROM workflow_runs WHERE id = ?1",
            )?;
            let mut rows = stmt.query(rusqlite::params![rid])?;
            if let Some(row) = rows.next()? {
                let run_id: String = row.get(0)?;
                let mut run = WorkflowRun {
                    id: run_id.clone(),
                    workflow_id: row.get(1)?,
                    status: match row.get::<_, String>(2)?.as_str() {
                        "completed" => WorkflowRunStatus::Completed,
                        "failed" => WorkflowRunStatus::Failed,
                        "cancelled" => WorkflowRunStatus::Cancelled,
                        _ => WorkflowRunStatus::Running,
                    },
                    step_results: vec![],
                    started_at: row.get(3)?,
                    completed_at: row.get(4)?,
                    error: row.get(5)?,
                };
                // Load step results
                let mut step_stmt = conn.prepare(
                    "SELECT step_name, output, success, duration_ms, error FROM workflow_step_results WHERE run_id = ?1 ORDER BY executed_at",
                )?;
                run.step_results = step_stmt
                    .query_map(rusqlite::params![run_id], |row| {
                        Ok(StepOutput {
                            step_name: row.get(0)?,
                            output: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                            success: row.get(2)?,
                            duration_ms: row.get::<_, i64>(3)? as u64,
                            error: row.get(4)?,
                        })
                    })?
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                Ok(Some(run))
            } else {
                Ok(None)
            }
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn simple_workflow(steps: Vec<WorkflowStep>) -> Workflow {
        Workflow {
            id: "test".into(),
            name: "Test".into(),
            description: "test workflow".into(),
            schedule: None,
            steps,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    fn tool_step(name: &str, tool: &str) -> WorkflowStep {
        WorkflowStep {
            name: name.into(),
            step_type: StepType::Tool {
                tool: tool.into(),
                args: serde_json::json!({}),
            },
            depends_on: vec![],
            retry: None,
            failure_policy: FailurePolicy::Stop,
            timeout_secs: None,
        }
    }

    fn delay_step(name: &str, seconds: u64) -> WorkflowStep {
        WorkflowStep {
            name: name.into(),
            step_type: StepType::Delay { seconds },
            depends_on: vec![],
            retry: None,
            failure_policy: FailurePolicy::Stop,
            timeout_secs: None,
        }
    }

    // 5.25
    #[test]
    fn build_dag_simple() {
        let mut s2 = tool_step("s2", "system_info");
        s2.depends_on = vec!["s1".into()];
        let steps = vec![tool_step("s1", "system_info"), s2];
        let result = WorkflowExecutor::build_dag(&steps);
        assert!(result.is_ok());
        let (graph, indices) = result.unwrap();
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert!(indices.contains_key("s1"));
        assert!(indices.contains_key("s2"));
    }

    // 5.26
    #[test]
    fn build_dag_parallel() {
        let steps = vec![tool_step("s1", "a"), tool_step("s2", "b")];
        let (graph, _) = WorkflowExecutor::build_dag(&steps).unwrap();
        assert_eq!(graph.edge_count(), 0);
    }

    // 5.27
    #[test]
    fn build_dag_cyclic_errors() {
        let mut s1 = tool_step("s1", "a");
        s1.depends_on = vec!["s2".into()];
        let mut s2 = tool_step("s2", "b");
        s2.depends_on = vec!["s1".into()];
        let result = WorkflowExecutor::build_dag(&[s1, s2]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cyclic"));
    }

    // 5.28
    #[tokio::test]
    async fn execute_single_tool_step() {
        let dir = tempfile::TempDir::new().unwrap();
        let pool = crate::db::init_pool(&dir.path().join("test.db")).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();

        let executor = WorkflowExecutor::new(pool, 50, 300, 3);
        let tools = crate::tools::ToolRegistry::new();
        tools
            .register(Arc::new(
                crate::tools::system_info::SystemInfoTool::new(),
            ))
            .unwrap();
        let bus = crate::event_bus::TokioBroadcastBus::new(16);

        let mut step = tool_step("info", "system_info");
        step.step_type = StepType::Tool {
            tool: "system_info".into(),
            args: serde_json::json!({"action": "os"}),
        };
        let wf = simple_workflow(vec![step]);
        let run = executor.execute(&wf, &tools, &bus).await.unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Completed);
        assert_eq!(run.step_results.len(), 1);
        assert!(run.step_results[0].success);
    }

    // 5.29
    #[tokio::test]
    async fn execute_delay_step() {
        let dir = tempfile::TempDir::new().unwrap();
        let pool = crate::db::init_pool(&dir.path().join("test.db")).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();

        let executor = WorkflowExecutor::new(pool, 50, 300, 3);
        let tools = crate::tools::ToolRegistry::new();
        let bus = crate::event_bus::TokioBroadcastBus::new(16);

        let wf = simple_workflow(vec![delay_step("wait", 0)]); // 0 seconds for fast test
        let run = executor.execute(&wf, &tools, &bus).await.unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Completed);
    }

    // 5.30
    #[test]
    fn build_dag_empty_errors() {
        let result = WorkflowExecutor::build_dag(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no steps"));
    }

    // 5.31
    #[test]
    fn build_dag_unknown_dependency_errors() {
        let mut s1 = tool_step("s1", "a");
        s1.depends_on = vec!["nonexistent".into()];
        let result = WorkflowExecutor::build_dag(&[s1]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown step"));
    }

    // 5.32
    #[tokio::test]
    async fn execute_failure_stop() {
        let dir = tempfile::TempDir::new().unwrap();
        let pool = crate::db::init_pool(&dir.path().join("test.db")).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();

        let executor = WorkflowExecutor::new(pool, 50, 300, 3);
        let tools = crate::tools::ToolRegistry::new();
        let bus = crate::event_bus::TokioBroadcastBus::new(16);

        // Use a tool that doesn't exist -> will fail
        let wf = simple_workflow(vec![tool_step("bad", "nonexistent_tool")]);
        let run = executor.execute(&wf, &tools, &bus).await.unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Failed);
    }

    // 5.33
    #[tokio::test]
    async fn execute_failure_continue() {
        let dir = tempfile::TempDir::new().unwrap();
        let pool = crate::db::init_pool(&dir.path().join("test.db")).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();

        let executor = WorkflowExecutor::new(pool, 50, 300, 3);
        let tools = crate::tools::ToolRegistry::new();
        tools
            .register(Arc::new(
                crate::tools::system_info::SystemInfoTool::new(),
            ))
            .unwrap();
        let bus = crate::event_bus::TokioBroadcastBus::new(16);

        let mut bad = tool_step("bad", "nonexistent_tool");
        bad.failure_policy = FailurePolicy::Continue;
        let mut good = tool_step("good", "system_info");
        good.step_type = StepType::Tool {
            tool: "system_info".into(),
            args: serde_json::json!({"action": "os"}),
        };
        let wf = simple_workflow(vec![bad, good]);
        let run = executor.execute(&wf, &tools, &bus).await.unwrap();
        // Should complete because first step uses Continue policy
        assert_eq!(run.status, WorkflowRunStatus::Completed);
    }

    // 5.34
    #[test]
    fn build_dag_diamond_dependency() {
        //   s1
        //  / \
        // s2  s3
        //  \ /
        //   s4
        let s1 = tool_step("s1", "a");
        let mut s2 = tool_step("s2", "a");
        s2.depends_on = vec!["s1".into()];
        let mut s3 = tool_step("s3", "a");
        s3.depends_on = vec!["s1".into()];
        let mut s4 = tool_step("s4", "a");
        s4.depends_on = vec!["s2".into(), "s3".into()];
        let result = WorkflowExecutor::build_dag(&[s1, s2, s3, s4]);
        assert!(result.is_ok());
        let (graph, _) = result.unwrap();
        assert_eq!(graph.node_count(), 4);
        assert_eq!(graph.edge_count(), 4);
    }

    // 5.35
    #[tokio::test]
    async fn execute_timeout() {
        let dir = tempfile::TempDir::new().unwrap();
        let pool = crate::db::init_pool(&dir.path().join("test.db")).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();

        // 1 second timeout
        let executor = WorkflowExecutor::new(pool, 50, 1, 0);
        let tools = crate::tools::ToolRegistry::new();
        let bus = crate::event_bus::TokioBroadcastBus::new(16);

        // Delay step that exceeds timeout
        let wf = simple_workflow(vec![delay_step("slow", 10)]);
        let run = executor.execute(&wf, &tools, &bus).await.unwrap();
        assert_eq!(run.status, WorkflowRunStatus::Failed);
    }

    // 5.36
    #[tokio::test]
    async fn execute_max_steps_exceeded() {
        let dir = tempfile::TempDir::new().unwrap();
        let pool = crate::db::init_pool(&dir.path().join("test.db")).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();

        // max_steps = 1, but workflow has 2 steps
        let executor = WorkflowExecutor::new(pool, 1, 300, 0);
        let tools = crate::tools::ToolRegistry::new();
        let bus = crate::event_bus::TokioBroadcastBus::new(16);

        let wf = simple_workflow(vec![delay_step("s1", 0), delay_step("s2", 0)]);
        let result = executor.execute(&wf, &tools, &bus).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("max"));
    }

    // 5.37
    #[tokio::test]
    async fn execute_persists_run() {
        let dir = tempfile::TempDir::new().unwrap();
        let pool = crate::db::init_pool(&dir.path().join("test.db")).unwrap();
        crate::db::with_db(&pool, crate::db::run_migrations)
            .await
            .unwrap();

        let executor = WorkflowExecutor::new(pool, 50, 300, 3);
        let tools = crate::tools::ToolRegistry::new();
        tools
            .register(Arc::new(
                crate::tools::system_info::SystemInfoTool::new(),
            ))
            .unwrap();
        let bus = crate::event_bus::TokioBroadcastBus::new(16);

        let mut step = tool_step("info", "system_info");
        step.step_type = StepType::Tool {
            tool: "system_info".into(),
            args: serde_json::json!({"action": "os"}),
        };
        let wf = simple_workflow(vec![step]);
        let run = executor.execute(&wf, &tools, &bus).await.unwrap();

        // Verify persisted
        let history = executor.get_history("test").await.unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, run.id);

        let details = executor.get_run(&run.id).await.unwrap();
        assert!(details.is_some());
        assert_eq!(details.unwrap().step_results.len(), 1);
    }
}
