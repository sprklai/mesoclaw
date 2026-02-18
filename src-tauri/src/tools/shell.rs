use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::security::{SecurityPolicy, ValidationResult};

use super::traits::{Tool, ToolResult};

/// Executes shell commands via `/bin/sh -c`, subject to the active
/// [`SecurityPolicy`].
pub struct ShellTool {
    policy: Arc<SecurityPolicy>,
}

impl ShellTool {
    pub fn new(policy: Arc<SecurityPolicy>) -> Self {
        Self { policy }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its stdout/stderr output. \
         Only safe, non-destructive commands are permitted by the active security policy."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute."
                },
                "working_dir": {
                    "type": "string",
                    "description": "Optional working directory for the command."
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        let command = args
            .get("command")
            .and_then(Value::as_str)
            .ok_or("missing required argument 'command'")?
            .to_string();

        let working_dir = args
            .get("working_dir")
            .and_then(Value::as_str)
            .map(str::to_string);

        // Security gate.
        let risk = self.policy.classify_command_risk(&command);
        let decision = self.policy.validate_command(&command);

        let decision_str = match &decision {
            ValidationResult::Allowed => "allowed",
            ValidationResult::NeedsApproval => "needs_approval",
            ValidationResult::Denied(_) => "denied",
        };
        self.policy
            .log_action(self.name(), args.clone(), risk, decision_str, None);

        match decision {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Err("command requires user approval before execution".into());
            }
            ValidationResult::Denied(reason) => {
                return Err(format!("command denied: {reason}"));
            }
        }

        // Run the command in a blocking thread to avoid blocking the async runtime.
        let result = tokio::task::spawn_blocking(move || {
            let mut cmd = std::process::Command::new("sh");
            cmd.arg("-c").arg(&command);
            if let Some(dir) = &working_dir {
                cmd.current_dir(dir);
            }
            cmd.output()
                .map_err(|e| format!("failed to spawn process: {e}"))
        })
        .await
        .map_err(|e| format!("blocking task panicked: {e}"))??;

        let stdout = String::from_utf8_lossy(&result.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&result.stderr).into_owned();
        let success = result.status.success();

        let output = if stderr.is_empty() {
            stdout
        } else if stdout.is_empty() {
            format!("STDERR: {stderr}")
        } else {
            format!("{stdout}\nSTDERR: {stderr}")
        };

        let meta = json!({ "exit_code": result.status.code() });
        if success {
            Ok(ToolResult::ok(output).with_metadata(meta))
        } else {
            Ok(ToolResult::err(output).with_metadata(meta))
        }
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
    async fn echo_succeeds() {
        let tool = ShellTool::new(full_policy());
        let r = tool
            .execute(json!({"command": "echo hello"}))
            .await
            .unwrap();
        assert!(r.success);
        assert!(r.output.contains("hello"));
    }

    #[tokio::test]
    async fn missing_command_arg_errors() {
        let tool = ShellTool::new(full_policy());
        let r = tool.execute(json!({})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn blocked_by_readonly() {
        let tool = ShellTool::new(readonly_policy());
        // mkdir is Medium risk — blocked in ReadOnly mode.
        let r = tool
            .execute(json!({"command": "mkdir /tmp/test_readonly_blocked"}))
            .await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn blocked_rm_always() {
        let tool = ShellTool::new(full_policy());
        let r = tool.execute(json!({"command": "rm -rf /"})).await;
        assert!(r.is_err());
    }

    #[tokio::test]
    async fn schema_is_valid_json_object() {
        let tool = ShellTool::new(full_policy());
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["command"].is_object());
    }
}
