use std::sync::Arc;

use async_trait::async_trait;

use crate::security::policy::{SecurityPolicy, ValidationResult};
use crate::{MesoError, Result};

use super::traits::{Tool, ToolResult};

pub struct ShellTool {
    policy: Arc<SecurityPolicy>,
    timeout_secs: u64,
}

impl ShellTool {
    pub fn new(policy: Arc<SecurityPolicy>, timeout_secs: u64) -> Self {
        Self {
            policy,
            timeout_secs,
        }
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn risk_level(&self) -> crate::security::RiskLevel {
        crate::security::RiskLevel::High
    }

    fn description(&self) -> &str {
        "Execute a shell command"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "command": { "type": "string", "description": "The shell command to execute" }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult> {
        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MesoError::Tool("missing 'command' argument".into()))?;

        match self.policy.validate_command(command) {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Ok(ToolResult::err(format!(
                    "Command needs approval: {command}"
                )));
            }
            ValidationResult::Denied(reason) => {
                return Ok(ToolResult::err(format!("Denied: {reason}")));
            }
        }

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output(),
        )
        .await
        .map_err(|_| MesoError::Tool("command timed out".into()))?
        .map_err(|e| MesoError::Tool(format!("command failed: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(ToolResult::ok(stdout))
        } else {
            Ok(ToolResult::err(
                format!("{stdout}\n{stderr}").trim().to_string(),
            ))
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
    async fn echo_succeeds() {
        let tool = ShellTool::new(policy(AutonomyLevel::Full), 30);
        let result = tool
            .execute(serde_json::json!({"command": "echo hello"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(result.output.contains("hello"));
    }

    #[tokio::test]
    async fn missing_command_arg_errors() {
        let tool = ShellTool::new(policy(AutonomyLevel::Full), 30);
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn blocked_by_readonly() {
        let tool = ShellTool::new(policy(AutonomyLevel::ReadOnly), 30);
        let result = tool
            .execute(serde_json::json!({"command": "mkdir /tmp/test"}))
            .await
            .unwrap();
        assert!(!result.success);
    }

    #[tokio::test]
    async fn blocked_rm_always() {
        let tool = ShellTool::new(policy(AutonomyLevel::Full), 30);
        let result = tool
            .execute(serde_json::json!({"command": "rm -rf /"}))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.output.contains("Denied"));
    }

    #[test]
    fn schema_is_valid_json_object() {
        let tool = ShellTool::new(
            Arc::new(SecurityPolicy::new(
                AutonomyLevel::Supervised,
                None,
                vec![],
                60,
                60,
                100,
            )),
            30,
        );
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
        assert!(schema.get("properties").is_some());
    }
}
