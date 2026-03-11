use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use crate::scheduler::TokioScheduler;
use crate::scheduler::traits::{ActiveHours, JobPayload, Schedule, ScheduledJob, Scheduler};
use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

/// Agent tool for managing scheduled jobs (create/list/delete/toggle/history).
pub struct SchedulerTool {
    scheduler: Arc<TokioScheduler>,
}

impl SchedulerTool {
    pub fn new(scheduler: Arc<TokioScheduler>) -> Self {
        Self { scheduler }
    }
}

#[async_trait]
impl Tool for SchedulerTool {
    fn name(&self) -> &str {
        "scheduler"
    }

    fn description(&self) -> &str {
        "Manage scheduled jobs. Actions: 'create' (add a job), 'list' (view all), 'delete' (remove), 'toggle' (enable/disable), 'history' (execution log). IMPORTANT: Before creating a job, ALWAYS call with action='list' first to check existing jobs and avoid duplicates. For one-time reminders (e.g. 'remind me tonight', 'at 5pm today'), ALWAYS set one_shot=true so the job auto-deletes after firing. Use cron for recurring and interval for periodic jobs."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "list", "delete", "toggle", "history"],
                    "description": "The scheduler operation to perform"
                },
                "job_id": {
                    "type": "string",
                    "description": "Job ID (required for delete/toggle/history)"
                },
                "name": {
                    "type": "string",
                    "description": "Job name (required for create)"
                },
                "schedule_type": {
                    "type": "string",
                    "enum": ["cron", "interval"],
                    "description": "Schedule type (required for create)"
                },
                "cron_expr": {
                    "type": "string",
                    "description": "Cron expression (required if schedule_type=cron)"
                },
                "interval_secs": {
                    "type": "integer",
                    "description": "Interval in seconds (required if schedule_type=interval)"
                },
                "payload_type": {
                    "type": "string",
                    "enum": ["heartbeat", "agent_turn", "notify", "send_via_channel"],
                    "description": "What the job does when it fires (required for create)"
                },
                "prompt": {
                    "type": "string",
                    "description": "Prompt for agent_turn payload"
                },
                "message": {
                    "type": "string",
                    "description": "Message for notify or send_via_channel payload"
                },
                "channel": {
                    "type": "string",
                    "description": "Channel name for send_via_channel payload"
                },
                "one_shot": {
                    "type": "boolean",
                    "description": "Set to true for one-time events (e.g. 'remind me today/tonight/tomorrow at X'). The job auto-deletes after its first successful execution. ALWAYS use this for non-recurring reminders."
                },
                "active_hours_start": {
                    "type": "integer",
                    "description": "Start hour (0-23) for active hours window"
                },
                "active_hours_end": {
                    "type": "integer",
                    "description": "End hour (0-23) for active hours window"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args["action"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'action' field".into()))?;

        match action {
            "create" => self.create_job(&args).await,
            "list" => self.list_jobs().await,
            "delete" => {
                let id = args["job_id"]
                    .as_str()
                    .ok_or_else(|| MesoError::Validation("missing 'job_id' for delete".into()))?;
                match self.scheduler.remove_job(id).await {
                    Ok(()) => Ok(ToolResult::ok(format!("Job '{id}' deleted"))),
                    Err(e) => Ok(ToolResult::err(format!("Failed to delete job: {e}"))),
                }
            }
            "toggle" => {
                let id = args["job_id"]
                    .as_str()
                    .ok_or_else(|| MesoError::Validation("missing 'job_id' for toggle".into()))?;
                match self.scheduler.toggle_job(id).await {
                    Ok(enabled) => Ok(ToolResult::ok(format!(
                        "Job '{id}' is now {}",
                        if enabled { "enabled" } else { "disabled" }
                    ))),
                    Err(e) => Ok(ToolResult::err(format!("Failed to toggle job: {e}"))),
                }
            }
            "history" => {
                let id = args["job_id"]
                    .as_str()
                    .ok_or_else(|| MesoError::Validation("missing 'job_id' for history".into()))?;
                let entries = self.scheduler.job_history(id).await;
                let json = serde_json::to_string_pretty(&entries).unwrap_or_default();
                Ok(ToolResult::ok(json))
            }
            other => Ok(ToolResult::err(format!(
                "Unknown action '{other}'. Valid actions: create, list, delete, toggle, history"
            ))),
        }
    }
}

impl SchedulerTool {
    async fn create_job(&self, args: &serde_json::Value) -> Result<ToolResult> {
        let name = args["name"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'name' for create".into()))?;
        let schedule_type = args["schedule_type"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'schedule_type' for create".into()))?;
        let payload_type = args["payload_type"]
            .as_str()
            .ok_or_else(|| MesoError::Validation("missing 'payload_type' for create".into()))?;

        let schedule = match schedule_type {
            "cron" => {
                let expr = args["cron_expr"].as_str().ok_or_else(|| {
                    MesoError::Validation("missing 'cron_expr' for cron schedule".into())
                })?;
                Schedule::Cron {
                    expr: expr.to_string(),
                }
            }
            "interval" => {
                let secs = args["interval_secs"].as_u64().ok_or_else(|| {
                    MesoError::Validation("missing 'interval_secs' for interval schedule".into())
                })?;
                Schedule::Interval { secs }
            }
            other => {
                return Ok(ToolResult::err(format!(
                    "Unknown schedule_type '{other}'. Valid: cron, interval"
                )));
            }
        };

        let payload = match payload_type {
            "heartbeat" => JobPayload::Heartbeat,
            "agent_turn" => {
                let prompt = args["prompt"].as_str().ok_or_else(|| {
                    MesoError::Validation("missing 'prompt' for agent_turn payload".into())
                })?;
                JobPayload::AgentTurn {
                    prompt: prompt.to_string(),
                }
            }
            "notify" => {
                let message = args["message"].as_str().ok_or_else(|| {
                    MesoError::Validation("missing 'message' for notify payload".into())
                })?;
                JobPayload::Notify {
                    message: message.to_string(),
                }
            }
            "send_via_channel" => {
                let channel = args["channel"].as_str().ok_or_else(|| {
                    MesoError::Validation("missing 'channel' for send_via_channel".into())
                })?;
                let message = args["message"].as_str().ok_or_else(|| {
                    MesoError::Validation("missing 'message' for send_via_channel".into())
                })?;
                JobPayload::SendViaChannel {
                    channel: channel.to_string(),
                    message: message.to_string(),
                }
            }
            other => {
                return Ok(ToolResult::err(format!(
                    "Unknown payload_type '{other}'. Valid: heartbeat, agent_turn, notify, send_via_channel"
                )));
            }
        };

        let active_hours = match (
            args["active_hours_start"].as_u64(),
            args["active_hours_end"].as_u64(),
        ) {
            (Some(start), Some(end)) => Some(ActiveHours {
                start_hour: start as u8,
                end_hour: end as u8,
            }),
            _ => None,
        };

        let delete_after_run = args["one_shot"].as_bool().unwrap_or(false);

        let job = ScheduledJob {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            schedule,
            session_target: Default::default(),
            payload,
            enabled: true,
            error_count: 0,
            next_run: None,
            active_hours,
            delete_after_run,
        };

        match self.scheduler.add_job(job).await {
            Ok(id) => Ok(ToolResult::ok(format!("Job created with ID: {id}"))),
            Err(e) => Ok(ToolResult::err(format!("Failed to create job: {e}"))),
        }
    }

    async fn list_jobs(&self) -> Result<ToolResult> {
        let jobs = self.scheduler.list_jobs().await;
        let json = serde_json::to_string_pretty(&jobs).unwrap_or_default();
        Ok(ToolResult::ok(json))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::db;
    use crate::event_bus::TokioBroadcastBus;

    async fn setup() -> (tempfile::TempDir, SchedulerTool) {
        let dir = tempfile::TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let pool = db::init_pool(&db_path).unwrap();
        db::with_db(&pool, db::run_migrations).await.unwrap();

        let config = AppConfig::default();
        let event_bus = Arc::new(TokioBroadcastBus::new(256));
        let scheduler = TokioScheduler::new(pool, event_bus, &config);

        let tool = SchedulerTool::new(scheduler);
        (dir, tool)
    }

    // 17.1 — Create job via tool returns success + job ID
    #[tokio::test]
    async fn scheduler_tool_create_job() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "create",
                "name": "test-job",
                "schedule_type": "interval",
                "interval_secs": 300,
                "payload_type": "heartbeat"
            }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("Job created with ID:"));
    }

    // 17.2 — List jobs via tool returns created jobs
    #[tokio::test]
    async fn scheduler_tool_list_jobs() {
        let (_dir, tool) = setup().await;

        // Create a job first
        tool.execute(json!({
            "action": "create",
            "name": "list-test",
            "schedule_type": "interval",
            "interval_secs": 60,
            "payload_type": "notify",
            "message": "test notification"
        }))
        .await
        .unwrap();

        let result = tool.execute(json!({ "action": "list" })).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("list-test"));
    }

    // 17.3 — Delete job via tool removes job
    #[tokio::test]
    async fn scheduler_tool_delete_job() {
        let (_dir, tool) = setup().await;

        // Create, extract ID, delete
        let create_result = tool
            .execute(json!({
                "action": "create",
                "name": "delete-me",
                "schedule_type": "interval",
                "interval_secs": 60,
                "payload_type": "heartbeat"
            }))
            .await
            .unwrap();

        let id = create_result
            .output
            .strip_prefix("Job created with ID: ")
            .unwrap();

        let result = tool
            .execute(json!({ "action": "delete", "job_id": id }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("deleted"));
    }

    // 17.4 — Toggle job via tool changes enabled state
    #[tokio::test]
    async fn scheduler_tool_toggle_job() {
        let (_dir, tool) = setup().await;

        let create_result = tool
            .execute(json!({
                "action": "create",
                "name": "toggle-me",
                "schedule_type": "interval",
                "interval_secs": 60,
                "payload_type": "heartbeat"
            }))
            .await
            .unwrap();

        let id = create_result
            .output
            .strip_prefix("Job created with ID: ")
            .unwrap();

        let result = tool
            .execute(json!({ "action": "toggle", "job_id": id }))
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.output.contains("disabled")); // was enabled, now disabled
    }

    // 17.5 — History via tool returns entries for job
    #[tokio::test]
    async fn scheduler_tool_history() {
        let (_dir, tool) = setup().await;

        let create_result = tool
            .execute(json!({
                "action": "create",
                "name": "history-test",
                "schedule_type": "interval",
                "interval_secs": 60,
                "payload_type": "heartbeat"
            }))
            .await
            .unwrap();

        let id = create_result
            .output
            .strip_prefix("Job created with ID: ")
            .unwrap();

        let result = tool
            .execute(json!({ "action": "history", "job_id": id }))
            .await
            .unwrap();

        assert!(result.success);
        // Empty history is valid JSON array
        assert!(result.output.contains('['));
    }

    // 17.6 — Invalid action returns error
    #[tokio::test]
    async fn scheduler_tool_invalid_action() {
        let (_dir, tool) = setup().await;
        let result = tool.execute(json!({ "action": "invalid" })).await.unwrap();

        assert!(!result.success);
        assert!(result.output.contains("Unknown action"));
    }

    // 17.7 — Create with missing name returns error
    #[tokio::test]
    async fn scheduler_tool_create_missing_fields() {
        let (_dir, tool) = setup().await;
        let result = tool
            .execute(json!({
                "action": "create",
                "schedule_type": "interval",
                "interval_secs": 60,
                "payload_type": "heartbeat"
            }))
            .await;

        assert!(result.is_err());
    }

    // 17.9 — Duplicate job name returns tool error
    #[tokio::test]
    async fn scheduler_tool_duplicate_name_rejected() {
        let (_dir, tool) = setup().await;
        let args = json!({
            "action": "create",
            "name": "dup-job",
            "schedule_type": "interval",
            "interval_secs": 300,
            "payload_type": "heartbeat"
        });
        let first = tool.execute(args.clone()).await.unwrap();
        assert!(first.success);

        let second = tool.execute(args).await.unwrap();
        assert!(!second.success);
        assert!(second.output.contains("already exists"));
    }

    // 17.8 — Tool name/description/schema validation
    #[tokio::test]
    async fn scheduler_tool_schema() {
        let (_dir, tool) = setup().await;
        assert_eq!(tool.name(), "scheduler");
        assert!(tool.description().contains("scheduled"));

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("action")));
    }
}
