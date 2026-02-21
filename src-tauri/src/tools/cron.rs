//! Agent-initiated job scheduling tool.
//!
//! This tool allows the agent to schedule prompts to run at specific times
//! or intervals. It integrates with the existing scheduler subsystem.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::scheduler::{JobPayload, Schedule, ScheduledJob, Scheduler, SessionTarget};
use crate::security::{RiskLevel, SecurityPolicy, ValidationResult};

use super::traits::{Tool, ToolResult};

/// Tool for agent-initiated job scheduling.
pub struct CronTool {
    policy: Arc<SecurityPolicy>,
    scheduler: Arc<dyn Scheduler>,
}

impl CronTool {
    pub fn new(policy: Arc<SecurityPolicy>, scheduler: Arc<dyn Scheduler>) -> Self {
        Self { policy, scheduler }
    }
}

#[async_trait]
impl Tool for CronTool {
    fn name(&self) -> &str {
        "cron"
    }

    fn description(&self) -> &str {
        "Schedule agent prompts to run at specific times or intervals. \
         Supports cron expressions (e.g., '0 9 * * *' for daily at 9 AM) \
         and interval-based schedules (e.g., every 3600 seconds)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "list", "delete"],
                    "description": "Action to perform: 'create' to add a new job, 'list' to see all jobs, 'delete' to remove a job."
                },
                "schedule": {
                    "type": "object",
                    "description": "Schedule configuration (required for 'create' action).",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["interval", "cron"],
                            "description": "Type of schedule."
                        },
                        "secs": {
                            "type": "integer",
                            "description": "Interval in seconds (for interval type)."
                        },
                        "expr": {
                            "type": "string",
                            "description": "Cron expression, 5 fields (for cron type). E.g., '0 9 * * *' = daily at 9 AM."
                        }
                    },
                    "required": ["type"]
                },
                "prompt": {
                    "type": "string",
                    "description": "Prompt to run when the job executes (required for 'create' action)."
                },
                "job_id": {
                    "type": "string",
                    "description": "Job ID to delete (required for 'delete' action)."
                },
                "name": {
                    "type": "string",
                    "description": "Human-readable name for the job (optional, for 'create' action)."
                },
                "isolated": {
                    "type": "boolean",
                    "description": "If true, run in isolated session instead of main (default: true)."
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let action = args
            .get("action")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'action'")?;

        // Security gate: scheduling is a medium-risk operation.
        match self.policy.validate_command(&format!("cron {action}")) {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Err("cron scheduling requires user approval".into());
            }
            ValidationResult::Denied(reason) => {
                return Err(format!("cron action denied: {reason}"));
            }
        }

        self.policy.log_action(
            self.name(),
            args.clone(),
            RiskLevel::Medium,
            "allowed",
            None,
        );

        match action {
            "create" => self.create_job(&args).await,
            "list" => self.list_jobs().await,
            "delete" => self.delete_job(&args).await,
            _ => Err(format!(
                "unknown action '{action}': expected 'create', 'list', or 'delete'"
            )),
        }
    }
}

impl CronTool {
    /// Create a new scheduled job.
    async fn create_job(&self, args: &Value) -> Result<ToolResult, String> {
        let prompt = args
            .get("prompt")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'prompt' for create action")?
            .to_string();

        let schedule = self.parse_schedule(args)?;
        let name = args
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("agent-job-{}", Uuid::new_v4().to_string().split('-').next().unwrap()));

        let isolated = args
            .get("isolated")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        let job = ScheduledJob {
            id: Uuid::new_v4().to_string(),
            name,
            schedule,
            session_target: if isolated {
                SessionTarget::Isolated
            } else {
                SessionTarget::Main
            },
            payload: JobPayload::AgentTurn { prompt },
            enabled: true,
            error_count: 0,
            next_run: None,
            active_hours: None,
            delete_after_run: false,
        };

        let job_id = self.scheduler.add_job(job).await;

        Ok(ToolResult::ok(format!(
            "Created scheduled job '{}' with ID: {}",
            job_id, job_id
        ))
        .with_metadata(json!({
            "job_id": job_id,
            "success": true
        })))
    }

    /// List all scheduled jobs.
    async fn list_jobs(&self) -> Result<ToolResult, String> {
        let jobs = self.scheduler.list_jobs().await;
        let count = jobs.len();

        let job_lines: Vec<String> = jobs
            .iter()
            .map(|j| {
                let status = if j.enabled { "enabled" } else { "disabled" };
                let schedule = match &j.schedule {
                    Schedule::Interval { secs } => format!("every {}s", secs),
                    Schedule::Cron { expr } => format!("cron: {}", expr),
                };
                let next = j
                    .next_run
                    .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
                    .unwrap_or_else(|| "pending".to_string());
                format!(
                    "{:<36} {:<20} {:<15} {:<20} {}",
                    j.id, j.name, status, schedule, next
                )
            })
            .collect();

        Ok(ToolResult::ok(format!(
            "ID                                   NAME                 STATUS          SCHEDULE             NEXT RUN\n{}",
            job_lines.join("\n")
        ))
        .with_metadata(json!({
            "count": count,
            "jobs": jobs.iter().map(|j| json!({
                "id": j.id,
                "name": j.name,
                "enabled": j.enabled,
                "schedule": match &j.schedule {
                    Schedule::Interval { secs } => json!({"type": "interval", "secs": secs}),
                    Schedule::Cron { expr } => json!({"type": "cron", "expr": expr}),
                }
            })).collect::<Vec<_>>()
        })))
    }

    /// Delete a scheduled job.
    async fn delete_job(&self, args: &Value) -> Result<ToolResult, String> {
        let job_id = args
            .get("job_id")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'job_id' for delete action")?
            .to_string();

        let removed = self.scheduler.remove_job(&job_id).await;

        if removed {
            Ok(ToolResult::ok(format!("Deleted job: {}", job_id))
                .with_metadata(json!({
                    "job_id": job_id,
                    "success": true
                })))
        } else {
            Ok(ToolResult::err(format!("Job not found: {}", job_id))
                .with_metadata(json!({
                    "job_id": job_id,
                    "success": false
                })))
        }
    }

    /// Parse the schedule from arguments.
    fn parse_schedule(&self, args: &Value) -> Result<Schedule, String> {
        let schedule_obj = args
            .get("schedule")
            .ok_or("missing required argument 'schedule' for create action")?;

        let schedule_type = schedule_obj
            .get("type")
            .and_then(Value::as_str)
            .ok_or("schedule must have a 'type' field")?;

        match schedule_type {
            "interval" => {
                let secs = schedule_obj
                    .get("secs")
                    .and_then(Value::as_u64)
                    .ok_or("interval schedule requires 'secs' field")?;
                Ok(Schedule::Interval { secs })
            }
            "cron" => {
                let expr = schedule_obj
                    .get("expr")
                    .and_then(Value::as_str)
                    .ok_or("cron schedule requires 'expr' field")?
                    .to_string();
                // Validate cron expression by attempting to parse it.
                // Support either 5-field (min hr dom mon dow) or 6-field (sec min hr dom mon dow).
                let full_expr = if expr.split_whitespace().count() == 5 {
                    format!("0 {expr}")
                } else {
                    expr.clone()
                };
                let _ = cron::Schedule::try_from(&*full_expr)
                    .map_err(|e| format!("invalid cron expression '{}': {}", expr, e))?;
                Ok(Schedule::Cron { expr })
            }
            _ => Err(format!(
                "unknown schedule type '{}': expected 'interval' or 'cron'",
                schedule_type
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scheduler::JobExecution;
    use crate::security::AutonomyLevel;
    use async_trait::async_trait;

    /// Mock scheduler for testing.
    struct MockScheduler {
        jobs: tokio::sync::RwLock<Vec<ScheduledJob>>,
    }

    impl MockScheduler {
        fn new() -> Self {
            Self {
                jobs: tokio::sync::RwLock::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl Scheduler for MockScheduler {
        async fn start(&self) {}
        async fn stop(&self) {}
        async fn add_job(&self, job: ScheduledJob) -> String {
            let id = job.id.clone();
            self.jobs.write().await.push(job);
            id
        }
        async fn remove_job(&self, id: &String) -> bool {
            let mut jobs = self.jobs.write().await;
            if let Some(pos) = jobs.iter().position(|j| &j.id == id) {
                jobs.remove(pos);
                true
            } else {
                false
            }
        }
        async fn list_jobs(&self) -> Vec<ScheduledJob> {
            self.jobs.read().await.clone()
        }
        async fn job_history(&self, _id: &String) -> Vec<JobExecution> {
            Vec::new()
        }
    }

    fn full_policy() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::new(
            AutonomyLevel::Full,
            None,
            vec![],
            3600,
            100,
        ))
    }

    fn test_tool() -> CronTool {
        let scheduler = Arc::new(MockScheduler::new());
        CronTool::new(full_policy(), scheduler)
    }

    #[tokio::test]
    async fn create_interval_job() {
        let tool = test_tool();
        let r = tool
            .execute(json!({
                "action": "create",
                "schedule": {"type": "interval", "secs": 3600},
                "prompt": "Check system status",
                "name": "status-check"
            }))
            .await
            .unwrap();
        assert!(r.success);
        assert!(r.output.contains("Created scheduled job"));
    }

    #[tokio::test]
    async fn create_cron_job() {
        let tool = test_tool();
        let r = tool
            .execute(json!({
                "action": "create",
                "schedule": {"type": "cron", "expr": "0 9 * * *"},
                "prompt": "Daily report"
            }))
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn list_jobs_empty() {
        let tool = test_tool();
        let r = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(r.success);
        // Check metadata for count = 0
        let count = r.metadata.unwrap()["count"].as_u64().unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn list_jobs_after_create() {
        let tool = test_tool();
        tool.execute(json!({
            "action": "create",
            "schedule": {"type": "interval", "secs": 60},
            "prompt": "test"
        }))
        .await
        .unwrap();

        let r = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(r.success);
        // Check metadata for count >= 1
        let count = r.metadata.unwrap()["count"].as_u64().unwrap();
        assert!(count >= 1);
    }

    #[tokio::test]
    async fn delete_job() {
        let tool = test_tool();

        // Create a job first.
        let create_result = tool
            .execute(json!({
                "action": "create",
                "schedule": {"type": "interval", "secs": 60},
                "prompt": "test"
            }))
            .await
            .unwrap();

        let job_id = create_result.metadata.unwrap()["job_id"]
            .as_str()
            .unwrap()
            .to_string();

        // Delete it.
        let r = tool
            .execute(json!({"action": "delete", "job_id": job_id}))
            .await
            .unwrap();
        assert!(r.success);
        assert!(r.output.contains("Deleted job"));
    }

    #[tokio::test]
    async fn delete_nonexistent_job() {
        let tool = test_tool();
        let r = tool
            .execute(json!({"action": "delete", "job_id": "nonexistent"}))
            .await
            .unwrap();
        assert!(!r.success);
        assert!(r.output.contains("not found"));
    }

    #[tokio::test]
    async fn create_missing_prompt_errors() {
        let tool = test_tool();
        let r = tool
            .execute(json!({
                "action": "create",
                "schedule": {"type": "interval", "secs": 60}
            }))
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn create_missing_schedule_errors() {
        let tool = test_tool();
        let r = tool
            .execute(json!({
                "action": "create",
                "prompt": "test"
            }))
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn invalid_cron_expression_errors() {
        let tool = test_tool();
        let r = tool
            .execute(json!({
                "action": "create",
                "schedule": {"type": "cron", "expr": "invalid"},
                "prompt": "test"
            }))
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn unknown_action_errors() {
        let tool = test_tool();
        let r = tool.execute(json!({"action": "invalid"})).await;
        assert!(r.is_err());
    }
}
