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
    /// Human-readable parameter summary, e.g. `"(query: string, max_results?: number)"`.
    /// Empty string when no parameters are defined.
    #[serde(default)]
    pub param_summary: String,
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

    /// Check if this tool call needs user approval before execution.
    /// Returns `Some(reason)` if approval is needed, `None` if the tool can proceed.
    /// Default: no approval needed.
    fn needs_approval(&self, _args: &serde_json::Value) -> Option<String> {
        None
    }

    /// Return a concise human-readable parameter summary for NL prompt generation,
    /// e.g. `"(query: string, max_results?: number)"`.
    ///
    /// Derived from `parameters_schema()` by default — extracts property names and types.
    /// Override in concrete tools for a hand-crafted, more informative summary.
    fn param_summary(&self) -> String {
        let schema = self.parameters_schema();
        let props = match schema.get("properties").and_then(|p| p.as_object()) {
            Some(p) => p,
            None => return String::new(),
        };
        let required: std::collections::HashSet<&str> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect()
            })
            .unwrap_or_default();

        let parts: Vec<String> = props
            .iter()
            .map(|(k, v)| {
                let type_str = v
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("any");
                if required.contains(k.as_str()) {
                    format!("{k}: {type_str}")
                } else {
                    format!("{k}?: {type_str}")
                }
            })
            .collect();

        if parts.is_empty() {
            String::new()
        } else {
            format!("({})", parts.join(", "))
        }
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

    // TA.8 — Default needs_approval returns None
    #[test]
    fn default_needs_approval_returns_none() {
        use async_trait::async_trait;

        struct DummyTool;

        #[async_trait]
        impl Tool for DummyTool {
            fn name(&self) -> &str {
                "dummy"
            }
            fn description(&self) -> &str {
                "A dummy tool"
            }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({})
            }
            async fn execute(&self, _args: serde_json::Value) -> crate::Result<ToolResult> {
                Ok(ToolResult::ok("ok"))
            }
        }

        let tool = DummyTool;
        let args = serde_json::json!({"command": "echo hello"});
        assert!(tool.needs_approval(&args).is_none());
    }

    // TA.9 — Default param_summary derives from parameters_schema properties
    #[test]
    fn default_param_summary_from_schema() {
        use async_trait::async_trait;

        struct SchemaTool;

        #[async_trait]
        impl Tool for SchemaTool {
            fn name(&self) -> &str { "schema_tool" }
            fn description(&self) -> &str { "Tool with schema" }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "limit": {"type": "integer"}
                    },
                    "required": ["query"]
                })
            }
            async fn execute(&self, _args: serde_json::Value) -> crate::Result<ToolResult> {
                Ok(ToolResult::ok("ok"))
            }
        }

        let tool = SchemaTool;
        let summary = tool.param_summary();
        assert!(!summary.is_empty(), "param_summary should not be empty");
        assert!(summary.contains("query"), "should contain required param 'query'");
        assert!(summary.contains("limit"), "should contain optional param 'limit'");
        assert!(summary.contains("query: string"), "required param should not have '?'");
        assert!(summary.contains("limit?: integer"), "optional param should have '?'");
    }

    // TA.10 — Default param_summary returns empty string when schema has no properties
    #[test]
    fn default_param_summary_empty_when_no_properties() {
        use async_trait::async_trait;

        struct NoParamTool;

        #[async_trait]
        impl Tool for NoParamTool {
            fn name(&self) -> &str { "no_param_tool" }
            fn description(&self) -> &str { "Tool without params" }
            fn parameters_schema(&self) -> serde_json::Value {
                serde_json::json!({"type": "object"})
            }
            async fn execute(&self, _args: serde_json::Value) -> crate::Result<ToolResult> {
                Ok(ToolResult::ok("ok"))
            }
        }

        let tool = NoParamTool;
        assert_eq!(tool.param_summary(), "");
    }
}
