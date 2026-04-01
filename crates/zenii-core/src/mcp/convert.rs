use std::sync::Arc;

use rmcp::model::{CallToolResult, Content, Tool as McpTool};
use serde_json::Map;

use crate::tools::{ToolInfo, ToolResult};

/// Convert a Zenii `ToolInfo` into an rmcp `Tool` definition.
pub fn tool_info_to_mcp(info: &ToolInfo, prefix: &str) -> McpTool {
    let name = format!("{prefix}{}", info.name);

    // rmcp expects input_schema as Arc<Map<String, Value>>
    let schema_map = match &info.parameters {
        serde_json::Value::Object(map) => map.clone(),
        _ => {
            let mut m = Map::new();
            m.insert("type".into(), serde_json::Value::String("object".into()));
            m
        }
    };

    McpTool::new(name, info.description.clone(), Arc::new(schema_map))
}

/// Convert a Zenii `ToolResult` into an rmcp `CallToolResult`.
pub fn tool_result_to_mcp(result: &ToolResult) -> CallToolResult {
    let content = vec![Content::text(result.output.clone())];
    if result.success {
        CallToolResult::success(content)
    } else {
        CallToolResult::error(content)
    }
}

/// Strip the prefix from an MCP tool name to get the Zenii tool name.
///
/// Returns `None` if a non-empty prefix is configured but the name doesn't match,
/// enforcing that callers must use the correct prefix.
pub fn strip_tool_prefix<'a>(mcp_name: &'a str, prefix: &str) -> Option<&'a str> {
    if prefix.is_empty() {
        Some(mcp_name)
    } else {
        mcp_name.strip_prefix(prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::RiskLevel;

    fn sample_tool_info() -> ToolInfo {
        ToolInfo {
            name: "web_search".into(),
            description: "Search the web".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search query"}
                },
                "required": ["query"]
            }),
            risk_level: RiskLevel::Low,
        }
    }

    #[test]
    fn tool_info_to_mcp_adds_prefix() {
        let info = sample_tool_info();
        let mcp = tool_info_to_mcp(&info, "zenii_");
        assert_eq!(mcp.name.as_ref(), "zenii_web_search");
    }

    #[test]
    fn tool_info_to_mcp_preserves_description() {
        let info = sample_tool_info();
        let mcp = tool_info_to_mcp(&info, "");
        assert_eq!(mcp.description.as_deref(), Some("Search the web"));
    }

    #[test]
    fn tool_info_to_mcp_preserves_schema() {
        let info = sample_tool_info();
        let mcp = tool_info_to_mcp(&info, "zenii_");
        assert!(mcp.input_schema.get("properties").is_some());
        assert_eq!(mcp.input_schema["type"], "object");
    }

    #[test]
    fn tool_info_to_mcp_empty_prefix() {
        let info = sample_tool_info();
        let mcp = tool_info_to_mcp(&info, "");
        assert_eq!(mcp.name.as_ref(), "web_search");
    }

    #[test]
    fn tool_info_to_mcp_fallback_for_non_object_schema() {
        let info = ToolInfo {
            name: "test".into(),
            description: "test".into(),
            parameters: serde_json::json!("not an object"),
            risk_level: RiskLevel::Low,
        };
        let mcp = tool_info_to_mcp(&info, "");
        assert_eq!(mcp.input_schema["type"], "object");
    }

    #[test]
    fn tool_result_to_mcp_success() {
        let result = ToolResult::ok("hello world");
        let mcp = tool_result_to_mcp(&result);
        assert!(!mcp.is_error.unwrap_or(false));
        assert!(!mcp.content.is_empty());
    }

    #[test]
    fn tool_result_to_mcp_error() {
        let result = ToolResult::err("something failed");
        let mcp = tool_result_to_mcp(&result);
        assert!(mcp.is_error.unwrap_or(false));
    }

    #[test]
    fn strip_prefix_removes_prefix() {
        assert_eq!(
            strip_tool_prefix("zenii_web_search", "zenii_"),
            Some("web_search")
        );
    }

    #[test]
    fn strip_prefix_rejects_missing_prefix() {
        assert_eq!(strip_tool_prefix("web_search", "zenii_"), None);
    }

    #[test]
    fn strip_prefix_empty_prefix_passes_through() {
        assert_eq!(strip_tool_prefix("web_search", ""), Some("web_search"));
    }
}
