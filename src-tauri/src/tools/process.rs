//! Process management tool for listing and terminating processes.
//!
//! This tool allows the agent to inspect running processes and optionally
//! terminate them. Due to the sensitive nature of process termination,
//! this tool is restricted to Full autonomy level only.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::security::{AutonomyLevel, RiskLevel, SecurityPolicy, ValidationResult};

use super::traits::{Tool, ToolResult};

/// Process management tool for listing and killing processes.
pub struct ProcessTool {
    policy: Arc<SecurityPolicy>,
}

impl ProcessTool {
    pub fn new(policy: Arc<SecurityPolicy>) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl Tool for ProcessTool {
    fn name(&self) -> &str {
        "process"
    }

    fn description(&self) -> &str {
        "List running processes or terminate a process by PID. \
         Process termination requires Full autonomy level and is subject to security policy approval."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "kill"],
                    "description": "Action to perform: 'list' to show processes, 'kill' to terminate a process."
                },
                "pid": {
                    "type": "integer",
                    "description": "Process ID to terminate (required for 'kill' action)."
                },
                "filter": {
                    "type": "string",
                    "description": "Optional filter string to match process names (for 'list' action)."
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

        match action {
            "list" => self.list_processes(&args).await,
            "kill" => self.kill_process(&args).await,
            _ => Err(format!(
                "unknown action '{action}': expected 'list' or 'kill'"
            )),
        }
    }
}

impl ProcessTool {
    /// List running processes, optionally filtered by name.
    async fn list_processes(&self, args: &Value) -> Result<ToolResult, String> {
        // List action is low risk, allowed in all modes.
        self.policy
            .log_action(self.name(), args.clone(), RiskLevel::Low, "allowed", None);

        let filter = args
            .get("filter")
            .and_then(Value::as_str)
            .map(str::to_lowercase);

        let result =
            tokio::task::spawn_blocking(move || list_processes_blocking(filter.as_deref()))
                .await
                .map_err(|e| format!("blocking task panicked: {e}"))??;

        Ok(result)
    }

    /// Kill a process by PID.
    async fn kill_process(&self, args: &Value) -> Result<ToolResult, String> {
        // Kill action is high risk - only allow in Full mode.
        if self.policy.autonomy_level != AutonomyLevel::Full {
            return Err(
                "process kill requires Full autonomy level for security reasons".to_string(),
            );
        }

        let pid = args
            .get("pid")
            .and_then(Value::as_u64)
            .ok_or("missing required argument 'pid' for kill action")? as u32;

        // Validate with security policy.
        let decision = self.policy.validate_command(&format!("kill {}", pid));
        match decision {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Err("process kill requires user approval".into());
            }
            ValidationResult::Denied(reason) => {
                return Err(format!("process kill denied: {reason}"));
            }
        }

        self.policy
            .log_action(self.name(), args.clone(), RiskLevel::High, "allowed", None);

        let result = tokio::task::spawn_blocking(move || kill_process_blocking(pid))
            .await
            .map_err(|e| format!("blocking task panicked: {e}"))??;

        Ok(result)
    }
}

/// List processes in a blocking context.
fn list_processes_blocking(filter: Option<&str>) -> Result<ToolResult, String> {
    use std::process::Command;

    // Use `ps` on Unix systems for process listing.
    let output = Command::new("ps")
        .args(["aux"])
        .output()
        .map_err(|e| format!("failed to execute ps: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(format!("ps command failed: {stderr}"));
    }

    // Parse and filter processes.
    let processes: Vec<ProcessInfo> = stdout
        .lines()
        .skip(1) // Skip header line
        .filter_map(|line| parse_ps_line(line))
        .filter(|p| {
            filter
                .map(|f| p.name.to_lowercase().contains(&f.to_lowercase()))
                .unwrap_or(true)
        })
        .collect();

    let count = processes.len();
    let output_lines: Vec<String> = processes
        .iter()
        .map(|p| format!("{:>7} {::>6} {:>6} {}", p.pid, p.cpu, p.mem, p.name))
        .collect();

    Ok(ToolResult::ok(format!(
        "PID       CPU%   MEM% COMMAND\n{}",
        output_lines.join("\n")
    ))
    .with_metadata(json!({
        "count": count,
        "processes": processes.iter().map(|p| json!({
            "pid": p.pid,
            "cpu": p.cpu,
            "mem": p.mem,
            "name": p.name
        })).collect::<Vec<_>>()
    })))
}

/// Kill a process in a blocking context.
fn kill_process_blocking(pid: u32) -> Result<ToolResult, String> {
    use std::process::Command;

    // Use `kill` command to terminate the process.
    let output = Command::new("kill")
        .args([pid.to_string()])
        .output()
        .map_err(|e| format!("failed to execute kill: {e}"))?;

    if output.status.success() {
        Ok(ToolResult::ok(format!(
            "Successfully sent termination signal to process {}",
            pid
        ))
        .with_metadata(json!({
            "pid": pid,
            "success": true
        })))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Ok(
            ToolResult::err(format!("Failed to kill process {}: {}", pid, stderr)).with_metadata(
                json!({
                    "pid": pid,
                    "success": false,
                    "error": stderr.to_string()
                }),
            ),
        )
    }
}

/// Parsed process information from `ps aux`.
#[derive(Debug, Clone)]
struct ProcessInfo {
    pid: u32,
    cpu: f32,
    mem: f32,
    name: String,
}

/// Parse a line from `ps aux` output.
fn parse_ps_line(line: &str) -> Option<ProcessInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 11 {
        return None;
    }

    let pid = parts[1].parse().ok()?;
    let cpu = parts[2].parse().ok()?;
    let mem = parts[3].parse().ok()?;
    let name = parts[10].to_string();

    Some(ProcessInfo {
        pid,
        cpu,
        mem,
        name,
    })
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

    fn supervised_policy() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::new(
            AutonomyLevel::Supervised,
            None,
            vec![],
            3600,
            100,
        ))
    }

    fn readonly_policy() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::new(
            AutonomyLevel::ReadOnly,
            None,
            vec![],
            3600,
            100,
        ))
    }

    #[tokio::test]
    async fn list_processes_succeeds() {
        let tool = ProcessTool::new(full_policy());
        let r = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(r.success);
        assert!(r.output.contains("PID"));
    }

    #[tokio::test]
    async fn list_with_filter() {
        let tool = ProcessTool::new(full_policy());
        let r = tool
            .execute(json!({"action": "list", "filter": "systemd"}))
            .await
            .unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn list_works_in_readonly_mode() {
        let tool = ProcessTool::new(readonly_policy());
        let r = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(r.success);
    }

    #[tokio::test]
    async fn kill_requires_full_mode() {
        let tool = ProcessTool::new(readonly_policy());
        let r = tool.execute(json!({"action": "kill", "pid": 99999})).await;
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Full autonomy level"));

        let tool = ProcessTool::new(supervised_policy());
        let r = tool.execute(json!({"action": "kill", "pid": 99999})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn kill_missing_pid_errors() {
        let tool = ProcessTool::new(full_policy());
        let r = tool.execute(json!({"action": "kill"})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn unknown_action_errors() {
        let tool = ProcessTool::new(full_policy());
        let r = tool.execute(json!({"action": "invalid"})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn schema_is_valid_json_object() {
        let tool = ProcessTool::new(full_policy());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["action"].is_object());
        assert!(schema["properties"]["pid"].is_object());
    }

    #[test]
    fn parse_ps_line_valid() {
        let line = "root         1  0.0  0.1 123456 7890 ?        Ss   10:00   0:01 /sbin/init";
        let info = parse_ps_line(line).unwrap();
        assert_eq!(info.pid, 1);
        assert_eq!(info.cpu, 0.0);
        assert_eq!(info.mem, 0.1);
        assert_eq!(info.name, "/sbin/init");
    }

    #[test]
    fn parse_ps_line_too_short() {
        let line = "root 1 0.0";
        assert!(parse_ps_line(line).is_none());
    }
}
