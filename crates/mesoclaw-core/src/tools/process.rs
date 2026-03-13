use std::sync::Arc;

use async_trait::async_trait;

use crate::security::policy::{AutonomyLevel, SecurityPolicy};
use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

pub struct ProcessTool {
    policy: Arc<SecurityPolicy>,
    list_limit: usize,
}

impl ProcessTool {
    pub fn new(policy: Arc<SecurityPolicy>, list_limit: usize) -> Self {
        Self { policy, list_limit }
    }
}

#[async_trait]
impl Tool for ProcessTool {
    fn name(&self) -> &str {
        "process"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::High
    }

    fn description(&self) -> &str {
        "List or manage system processes"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "action": { "type": "string", "enum": ["list", "kill"], "description": "Action to perform" },
                "filter": { "type": "string", "description": "Filter processes by name" },
                "pid": { "type": "integer", "description": "Process ID for kill action" }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MesoError::Tool("missing 'action' argument".into()))?;

        match action {
            "list" => {
                let filter = args
                    .get("filter")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_lowercase());
                let limit = self.list_limit;

                let result = tokio::task::spawn_blocking(move || {
                    let sys = sysinfo::System::new_with_specifics(
                        sysinfo::RefreshKind::nothing()
                            .with_processes(sysinfo::ProcessRefreshKind::nothing()),
                    );
                    let mut processes: Vec<serde_json::Value> = sys
                        .processes()
                        .iter()
                        .filter(|(_, proc_)| {
                            if let Some(f) = &filter {
                                proc_
                                    .name()
                                    .to_string_lossy()
                                    .to_lowercase()
                                    .contains(f.as_str())
                            } else {
                                true
                            }
                        })
                        .take(limit)
                        .map(|(pid, proc_)| {
                            serde_json::json!({
                                "pid": pid.as_u32(),
                                "name": proc_.name().to_string_lossy(),
                            })
                        })
                        .collect();
                    processes.sort_by_key(|p| p["pid"].as_u64().unwrap_or(0));
                    serde_json::to_string_pretty(&processes).unwrap_or_default()
                })
                .await
                .map_err(|e| MesoError::Tool(format!("spawn error: {e}")))?;

                Ok(ToolResult::ok(result))
            }
            "kill" => {
                if self.policy.autonomy_level != AutonomyLevel::Full {
                    return Ok(ToolResult::err("Kill requires Full autonomy mode"));
                }
                let pid = args
                    .get("pid")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| MesoError::Tool("missing 'pid' argument for kill".into()))?;

                let pid_u32 = pid as u32;
                let result = tokio::task::spawn_blocking(move || {
                    let sys = sysinfo::System::new_with_specifics(
                        sysinfo::RefreshKind::nothing()
                            .with_processes(sysinfo::ProcessRefreshKind::nothing()),
                    );
                    let sysinfo_pid = sysinfo::Pid::from_u32(pid_u32);
                    match sys.process(sysinfo_pid) {
                        Some(process) => {
                            if process.kill() {
                                format!("Process {pid_u32} killed successfully")
                            } else {
                                format!("Failed to kill process {pid_u32}")
                            }
                        }
                        None => format!("Process {pid_u32} not found"),
                    }
                })
                .await
                .map_err(|e| MesoError::Tool(format!("spawn error: {e}")))?;

                if result.contains("not found") || result.contains("Failed") {
                    Ok(ToolResult::err(result))
                } else {
                    Ok(ToolResult::ok(result))
                }
            }
            unknown => Ok(ToolResult::err(format!("Unknown action: {unknown}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::policy::AutonomyLevel;

    fn policy(level: AutonomyLevel) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::new(level, None, vec![], 60, 60, 100))
    }

    #[tokio::test]
    async fn list_processes_succeeds() {
        let tool = ProcessTool::new(policy(AutonomyLevel::Full), 200);
        let result = tool
            .execute(serde_json::json!({"action": "list"}))
            .await
            .unwrap();
        assert!(result.success);
        let parsed: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert!(parsed.is_array());
    }

    #[tokio::test]
    async fn list_with_filter() {
        let tool = ProcessTool::new(policy(AutonomyLevel::Full), 200);
        let result = tool
            .execute(serde_json::json!({"action": "list", "filter": "cargo"}))
            .await
            .unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn kill_requires_full_mode() {
        let tool = ProcessTool::new(policy(AutonomyLevel::Supervised), 200);
        let result = tool
            .execute(serde_json::json!({"action": "kill", "pid": 1}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("Full autonomy"));
    }

    #[tokio::test]
    async fn kill_missing_pid_errors() {
        let tool = ProcessTool::new(policy(AutonomyLevel::Full), 200);
        let result = tool.execute(serde_json::json!({"action": "kill"})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn unknown_action_errors() {
        let tool = ProcessTool::new(policy(AutonomyLevel::Full), 200);
        let result = tool
            .execute(serde_json::json!({"action": "restart"}))
            .await
            .unwrap();
        assert!(!result.success);
    }

    // 8.9.1 — kill with non-Full autonomy returns error
    #[tokio::test]
    async fn kill_non_full_autonomy_errors() {
        let tool = ProcessTool::new(policy(AutonomyLevel::ReadOnly), 200);
        let result = tool
            .execute(serde_json::json!({"action": "kill", "pid": 1}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("Full autonomy"));
    }

    // 8.9.2 — kill without pid arg returns MesoError::Tool
    #[tokio::test]
    async fn kill_without_pid_returns_error() {
        let tool = ProcessTool::new(policy(AutonomyLevel::Full), 200);
        let result = tool.execute(serde_json::json!({"action": "kill"})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::MesoError::Tool(_)));
    }

    // 8.9.3 — kill with nonexistent PID returns "not found"
    #[tokio::test]
    async fn kill_nonexistent_pid_not_found() {
        let tool = ProcessTool::new(policy(AutonomyLevel::Full), 200);
        let result = tool
            .execute(serde_json::json!({"action": "kill", "pid": 999999999}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("not found"));
    }

    #[test]
    fn schema_is_valid() {
        let tool = ProcessTool::new(policy(AutonomyLevel::Full), 200);
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
        assert!(schema["properties"]["action"].is_object());
    }
}
