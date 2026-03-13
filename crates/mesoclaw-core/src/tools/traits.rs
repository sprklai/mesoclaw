use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;
use crate::security::RiskLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub output: String,
    pub success: bool,
    pub metadata: Option<serde_json::Value>,
}

impl ToolResult {
    pub fn ok(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: true,
            metadata: None,
        }
    }

    pub fn err(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: false,
            metadata: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub risk_level: RiskLevel,
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult>;

    /// Risk classification for this tool. Default: Low (safe for all surfaces).
    fn risk_level(&self) -> RiskLevel {
        RiskLevel::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_result_ok_is_success() {
        let r = ToolResult::ok("output");
        assert!(r.success);
        assert_eq!(r.output, "output");
    }

    #[test]
    fn tool_result_err_is_not_success() {
        let r = ToolResult::err("error");
        assert!(!r.success);
        assert_eq!(r.output, "error");
    }
}
