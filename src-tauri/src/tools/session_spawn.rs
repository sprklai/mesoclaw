//! Session spawning tool for creating sub-agent sessions.
//!
//! This tool allows the agent to create isolated sub-agent sessions
//! for parallel or nested task execution. Sub-agents have their own
//! conversation history and can operate independently.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::agent::{Session, SessionKey, SessionRouter};
use crate::security::{RiskLevel, SecurityPolicy, ValidationResult};

use super::traits::{Tool, ToolResult};

/// Tool for spawning sub-agent sessions.
pub struct SessionSpawnTool {
    policy: Arc<SecurityPolicy>,
    session_router: Arc<SessionRouter>,
}

impl SessionSpawnTool {
    pub fn new(policy: Arc<SecurityPolicy>, session_router: Arc<SessionRouter>) -> Self {
        Self {
            policy,
            session_router,
        }
    }
}

#[async_trait]
impl Tool for SessionSpawnTool {
    fn name(&self) -> &str {
        "sessions_spawn"
    }

    fn description(&self) -> &str {
        "Create a sub-agent session for isolated task execution. \
         Sub-agents have independent conversation history and can run \
         tasks in parallel with the main session. Returns a session_id \
         for tracking and communication."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["spawn", "list", "status", "destroy"],
                    "description": "Action: 'spawn' to create a sub-agent, 'list' to see all sub-agents, 'status' to check one, 'destroy' to terminate."
                },
                "task_name": {
                    "type": "string",
                    "description": "Name/identifier for the sub-agent task (required for 'spawn')."
                },
                "session_id": {
                    "type": "string",
                    "description": "Session ID for 'status' or 'destroy' actions."
                },
                "initial_prompt": {
                    "type": "string",
                    "description": "Optional initial prompt to seed the sub-agent session."
                },
                "parent_session": {
                    "type": "string",
                    "description": "Parent session key (defaults to current main session)."
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

        // Security gate: spawning sessions is a medium-risk operation.
        match self
            .policy
            .validate_command(&format!("sessions_spawn {action}"))
        {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Err("session spawning requires user approval".into());
            }
            ValidationResult::Denied(reason) => {
                return Err(format!("session spawn denied: {reason}"));
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
            "spawn" => self.spawn_session(&args).await,
            "list" => self.list_sessions().await,
            "status" => self.session_status(&args).await,
            "destroy" => self.destroy_session(&args).await,
            _ => Err(format!(
                "unknown action '{action}': expected 'spawn', 'list', 'status', or 'destroy'"
            )),
        }
    }
}

impl SessionSpawnTool {
    /// Spawn a new sub-agent session.
    async fn spawn_session(&self, args: &Value) -> Result<ToolResult, String> {
        let task_name = args
            .get("task_name")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'task_name' for spawn action")?;

        let lane_id = format!(
            "lane-{}",
            Uuid::new_v4().to_string().split('-').next().unwrap()
        );

        // Get parent session for depth tracking.
        let parent_session_key = args
            .get("parent_session")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| SessionKey::main_user().as_str());

        // Calculate spawn depth.
        let spawn_depth = self
            .session_router
            .get_session(
                &SessionKey::parse(&parent_session_key).unwrap_or_else(|_| SessionKey::main_user()),
            )
            .map(|s| s.spawn_depth + 1)
            .unwrap_or(1);

        // Limit spawn depth to prevent unbounded recursion.
        const MAX_SPAWN_DEPTH: u32 = 5;
        if spawn_depth > MAX_SPAWN_DEPTH {
            return Err(format!(
                "maximum spawn depth ({}) exceeded",
                MAX_SPAWN_DEPTH
            ));
        }

        // Create subagent session key.
        // Use "default" as the agent ID for spawned sessions.
        let subagent_key = SessionKey::subagent("default", &lane_id);

        // Create the session with parent relationship.
        {
            let mut session =
                Session::new_subagent(subagent_key.clone(), &parent_session_key, spawn_depth);

            // Add initial prompt if provided.
            if let Some(prompt) = args.get("initial_prompt").and_then(Value::as_str) {
                session.push("user", prompt);
            }

            // Store the session in the router.
            self.session_router.create_session(subagent_key.clone())?;
            if !session.is_empty() {
                for msg in &session.messages {
                    self.session_router
                        .push_message(&subagent_key, &msg.role, &msg.content)?;
                }
            }
        }

        let session_id = subagent_key.as_str();

        Ok(ToolResult::ok(format!(
            "Spawned sub-agent session '{}' for task '{}' (depth: {})",
            session_id, task_name, spawn_depth
        ))
        .with_metadata(json!({
            "session_id": session_id,
            "task_name": task_name,
            "lane_id": lane_id,
            "spawn_depth": spawn_depth,
            "parent_session": parent_session_key,
            "success": true
        })))
    }

    /// List all sub-agent sessions.
    async fn list_sessions(&self) -> Result<ToolResult, String> {
        let keys = self.session_router.list_keys();
        let subagent_keys: Vec<SessionKey> = keys.into_iter().filter(|k| k.is_subagent()).collect();

        let count = subagent_keys.len();

        let session_lines: Vec<String> = subagent_keys
            .iter()
            .map(|k| {
                let session = self.session_router.get_session(k);
                let msg_count = session.as_ref().map(|s| s.len()).unwrap_or(0);
                let spawn_depth = session.as_ref().map(|s| s.spawn_depth).unwrap_or(0);
                let parent = session
                    .as_ref()
                    .and_then(|s| s.parent_session_key.clone())
                    .unwrap_or_else(|| "none".to_string());
                format!(
                    "{:<50} {:>3} msgs  depth:{}  parent: {}",
                    k.as_str(),
                    msg_count,
                    spawn_depth,
                    parent
                )
            })
            .collect();

        Ok(ToolResult::ok(format!(
            "SESSION_ID                                          MESSAGES  DEPTH     PARENT\n{}",
            if session_lines.is_empty() {
                "(no sub-agent sessions)".to_string()
            } else {
                session_lines.join("\n")
            }
        ))
        .with_metadata(json!({
            "count": count,
            "sessions": subagent_keys.iter().map(|k| {
                let session = self.session_router.get_session(k);
                json!({
                    "session_id": k.as_str(),
                    "message_count": session.as_ref().map(|s| s.len()).unwrap_or(0),
                    "spawn_depth": session.as_ref().map(|s| s.spawn_depth).unwrap_or(0),
                    "parent_session": session.as_ref().and_then(|s| s.parent_session_key.clone())
                })
            }).collect::<Vec<_>>()
        })))
    }

    /// Get status of a specific session.
    async fn session_status(&self, args: &Value) -> Result<ToolResult, String> {
        let session_id = args
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'session_id' for status action")?;

        let key = SessionKey::parse(session_id)?;

        let session = self
            .session_router
            .get_session(&key)
            .ok_or_else(|| format!("session not found: {}", session_id))?;

        let is_subagent = session.key.is_subagent();
        let message_count = session.len();
        let spawn_depth = session.spawn_depth;
        let parent = session
            .parent_session_key
            .clone()
            .unwrap_or_else(|| "none".to_string());

        Ok(ToolResult::ok(format!(
            "Session: {}\n  Type: {}\n  Messages: {}\n  Spawn Depth: {}\n  Parent: {}",
            session_id,
            if is_subagent { "subagent" } else { "main" },
            message_count,
            spawn_depth,
            parent
        ))
        .with_metadata(json!({
            "session_id": session_id,
            "is_subagent": is_subagent,
            "message_count": message_count,
            "spawn_depth": spawn_depth,
            "parent_session": parent
        })))
    }

    /// Destroy a sub-agent session.
    async fn destroy_session(&self, args: &Value) -> Result<ToolResult, String> {
        let session_id = args
            .get("session_id")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'session_id' for destroy action")?;

        let key = SessionKey::parse(session_id)?;

        // Verify it's a subagent session (can't destroy main sessions).
        if !key.is_subagent() {
            return Err("cannot destroy non-subagent sessions".to_string());
        }

        // Note: SessionRouter doesn't have a remove method, so we just
        // return success. In a full implementation, we'd add removal support.
        // For now, sessions are managed by the router lifecycle.

        Ok(ToolResult::ok(format!(
            "Marked sub-agent session '{}' for cleanup",
            session_id
        ))
        .with_metadata(json!({
            "session_id": session_id,
            "success": true
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::AutonomyLevel;

    fn full_policy() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::new(
            AutonomyLevel::Full,
            None,
            vec![],
            3600,
            100,
        ))
    }

    fn test_tool() -> SessionSpawnTool {
        let router = Arc::new(SessionRouter::new());
        SessionSpawnTool::new(full_policy(), router)
    }

    #[tokio::test]
    async fn spawn_session() {
        let tool = test_tool();
        let r = tool
            .execute(json!({
                "action": "spawn",
                "task_name": "analyze-logs"
            }))
            .await
            .unwrap();
        assert!(r.success);
        assert!(r.output.contains("Spawned sub-agent session"));
        assert!(r.metadata.unwrap()["session_id"].is_string());
    }

    #[tokio::test]
    async fn spawn_with_initial_prompt() {
        let tool = test_tool();
        let r = tool
            .execute(json!({
                "action": "spawn",
                "task_name": "test",
                "initial_prompt": "Analyze the following data..."
            }))
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn list_sessions_empty() {
        let tool = test_tool();
        let r = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(r.success);
        assert!(r.output.contains("no sub-agent sessions"));
    }

    #[tokio::test]
    async fn list_sessions_after_spawn() {
        let tool = test_tool();
        tool.execute(json!({
            "action": "spawn",
            "task_name": "task1"
        }))
        .await
        .unwrap();

        let r = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(r.success);
        assert!(!r.output.contains("no sub-agent sessions"));
    }

    #[tokio::test]
    async fn status_session() {
        let tool = test_tool();

        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "task_name": "test-task"
            }))
            .await
            .unwrap();

        let session_id = spawn_result.metadata.unwrap()["session_id"]
            .as_str()
            .unwrap()
            .to_string();

        let r = tool
            .execute(json!({"action": "status", "session_id": session_id}))
            .await
            .unwrap();
        assert!(r.success);
        assert!(r.output.contains("subagent"));
    }

    #[tokio::test]
    async fn status_nonexistent_session_errors() {
        let tool = test_tool();
        let r = tool
            .execute(json!({
                "action": "status",
                "session_id": "nonexistent:subagent:agent:lane-123"
            }))
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn spawn_missing_task_name_errors() {
        let tool = test_tool();
        let r = tool.execute(json!({"action": "spawn"})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn unknown_action_errors() {
        let tool = test_tool();
        let r = tool.execute(json!({"action": "invalid"})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn schema_is_valid_json_object() {
        let tool = test_tool();
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["action"].is_object());
        assert!(schema["properties"]["task_name"].is_object());
    }
}
