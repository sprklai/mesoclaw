#[cfg(feature = "mcp-client")]
pub use inner::McpClientTool;

#[cfg(feature = "mcp-client")]
mod inner {
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::Value;

    use crate::Result;
    use crate::config::McpServerConfig;
    use crate::mcp::client::{McpClientManager, McpToolInfo};
    use crate::security::RiskLevel;
    use crate::tools::traits::{Tool, ToolResult};

    /// Wraps a single tool discovered from an external MCP server as a Zenii `Tool`.
    pub struct McpClientTool {
        /// Prefixed name exposed to the agent, e.g. "github/list_repos".
        display_name: String,
        /// Original tool name as reported by the MCP server.
        raw_name: String,
        description: String,
        schema: Value,
        server_config: McpServerConfig,
        manager: Arc<McpClientManager>,
    }

    impl McpClientTool {
        pub fn new(
            server_cfg: &McpServerConfig,
            tool_info: McpToolInfo,
            manager: Arc<McpClientManager>,
        ) -> Self {
            let display_name = match &server_cfg.tools_prefix {
                Some(prefix) => format!("{}{}", prefix, tool_info.name),
                None => tool_info.name.clone(),
            };
            Self {
                display_name,
                raw_name: tool_info.name,
                description: tool_info.description,
                schema: tool_info.schema,
                server_config: server_cfg.clone(),
                manager,
            }
        }
    }

    #[async_trait]
    impl Tool for McpClientTool {
        fn name(&self) -> &str {
            &self.display_name
        }

        fn description(&self) -> &str {
            &self.description
        }

        fn parameters_schema(&self) -> Value {
            self.schema.clone()
        }

        async fn execute(&self, args: Value) -> Result<ToolResult> {
            let result = self
                .manager
                .call_tool(&self.server_config, &self.raw_name, args)
                .await?;
            let output = match &result {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            Ok(ToolResult {
                output,
                success: true,
                metadata: None,
            })
        }

        fn risk_level(&self) -> RiskLevel {
            RiskLevel::Medium
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::config::McpTransport;
        use std::collections::HashMap;

        fn make_cfg(prefix: Option<&str>) -> McpServerConfig {
            McpServerConfig {
                id: "test-server".into(),
                transport: McpTransport::Stdio {
                    command: "false".into(),
                    args: vec![],
                    env: HashMap::new(),
                },
                tools_prefix: prefix.map(Into::into),
                enabled: true,
            }
        }

        fn make_info(name: &str) -> McpToolInfo {
            McpToolInfo {
                name: name.into(),
                description: "A test tool".into(),
                schema: serde_json::json!({"type": "object"}),
            }
        }

        async fn make_tool(prefix: Option<&str>, tool_name: &str) -> McpClientTool {
            let cfg = make_cfg(prefix);
            let info = make_info(tool_name);
            let manager = Arc::new(McpClientManager::connect_all(&[]).await.unwrap());
            McpClientTool::new(&cfg, info, manager)
        }

        #[tokio::test]
        async fn display_name_with_prefix() {
            let tool = make_tool(Some("github/"), "list_repos").await;
            assert_eq!(tool.name(), "github/list_repos");
        }

        #[tokio::test]
        async fn display_name_without_prefix() {
            let tool = make_tool(None, "list_repos").await;
            assert_eq!(tool.name(), "list_repos");
        }

        #[tokio::test]
        async fn raw_name_preserved() {
            let tool = make_tool(Some("prefix_"), "my_tool").await;
            assert_eq!(tool.raw_name, "my_tool");
        }

        #[tokio::test]
        async fn description_forwarded() {
            let tool = make_tool(None, "t").await;
            assert_eq!(tool.description(), "A test tool");
        }

        #[tokio::test]
        async fn parameters_schema_forwarded() {
            let tool = make_tool(None, "t").await;
            assert_eq!(
                tool.parameters_schema(),
                serde_json::json!({"type": "object"})
            );
        }

        #[tokio::test]
        async fn risk_level_is_medium() {
            let tool = make_tool(None, "t").await;
            assert_eq!(tool.risk_level(), RiskLevel::Medium);
        }

        #[tokio::test]
        async fn execute_propagates_mcp_error() {
            let tool = make_tool(None, "noop").await;
            let result = tool.execute(serde_json::json!({})).await;
            assert!(result.is_err());
        }
    }
}
