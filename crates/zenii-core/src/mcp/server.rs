use std::sync::Arc;

use arc_swap::ArcSwap;
use rmcp::ErrorData as McpError;
use rmcp::RoleServer;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, ListToolsResult,
    ServerCapabilities, ServerInfo, ToolsCapability,
};
use tracing::{debug, error, info};

use crate::config::AppConfig;
use crate::security::policy::{SecurityPolicy, ValidationResult};
use crate::tools::ToolInfo;
use crate::tools::registry::ToolRegistry;

use super::convert;

/// MCP server handler that exposes Zenii's tools via the Model Context Protocol.
///
/// Implements `ServerHandler` manually (not via `#[tool_router]`) because tools
/// are dynamic — loaded from `ToolRegistry` at runtime.
#[derive(Clone)]
pub struct ZeniiMcpServer {
    tools: Arc<ToolRegistry>,
    security: Arc<SecurityPolicy>,
    config: Arc<ArcSwap<AppConfig>>,
}

impl ZeniiMcpServer {
    pub fn new(
        tools: Arc<ToolRegistry>,
        security: Arc<SecurityPolicy>,
        config: Arc<ArcSwap<AppConfig>>,
    ) -> Self {
        Self {
            tools,
            security,
            config,
        }
    }

    /// Filter tools based on exposed/hidden config lists.
    fn visible_tools(&self) -> Vec<ToolInfo> {
        let cfg = self.config.load();
        self.tools
            .list()
            .into_iter()
            .filter(|t| {
                let name = &t.name;
                let in_allowlist = cfg.mcp_server_exposed_tools.is_empty()
                    || cfg.mcp_server_exposed_tools.contains(name);
                let not_hidden = !cfg.mcp_server_hidden_tools.contains(name);
                in_allowlist && not_hidden
            })
            .collect()
    }

    /// Resolve an MCP tool name to an internal tool name.
    ///
    /// Enforces both prefix matching and visibility (allowlist/denylist).
    /// Returns `None` if the prefix doesn't match, the tool is hidden,
    /// or the tool is not in the allowlist.
    fn resolve_tool_name<'a>(&self, mcp_name: &'a str) -> Option<&'a str> {
        let cfg = self.config.load();
        let name = convert::strip_tool_prefix(mcp_name, &cfg.mcp_server_tool_prefix)?;
        if self.visible_tools().iter().any(|t| t.name == name) {
            Some(name)
        } else {
            None
        }
    }
}

impl ServerHandler for ZeniiMcpServer {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(ToolsCapability::default());

        ServerInfo::new(caps)
            .with_server_info(Implementation::new("zenii", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "Zenii MCP server exposes local AI backend tools: \
                 memory, web search, file ops, shell, and more.",
            )
    }

    async fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let cfg = self.config.load();
        let prefix = &cfg.mcp_server_tool_prefix;
        let visible = self.visible_tools();
        let tools: Vec<_> = visible
            .iter()
            .map(|t| convert::tool_info_to_mcp(t, prefix))
            .collect();

        debug!(tool_count = tools.len(), "MCP list_tools");
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        // Enforce prefix + visibility — hidden tools are indistinguishable from nonexistent
        let zenii_name = self.resolve_tool_name(&request.name).ok_or_else(|| {
            McpError::invalid_params(format!("unknown tool: {}", request.name), None)
        })?;

        info!(tool = zenii_name, "MCP call_tool");

        // Convert arguments from Option<Map> to serde_json::Value
        let args = match request.arguments {
            Some(map) => serde_json::Value::Object(map),
            None => serde_json::Value::Object(serde_json::Map::new()),
        };

        // Security check
        match self.security.validate_tool_execution(zenii_name, &args) {
            ValidationResult::Allowed => {}
            ValidationResult::NeedsApproval => {
                return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Tool '{zenii_name}' requires approval (supervised mode)"
                ))]));
            }
            ValidationResult::Denied(reason) => {
                return Ok(CallToolResult::error(vec![Content::text(reason)]));
            }
        }

        // Lookup tool (guaranteed to exist after visibility check)
        let tool = self.tools.get(zenii_name).ok_or_else(|| {
            McpError::invalid_params(format!("unknown tool: {}", request.name), None)
        })?;

        // Execute
        match tool.execute(args).await {
            Ok(result) => Ok(convert::tool_result_to_mcp(&result)),
            Err(e) => {
                error!(tool = zenii_name, error = %e, "MCP tool execution failed");
                Ok(CallToolResult::error(vec![Content::text(e.to_string())]))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::AutonomyLevel;
    use crate::tools::traits::ToolResult;
    use async_trait::async_trait;

    /// Minimal tool for testing
    struct EchoTool;

    #[async_trait]
    impl crate::tools::traits::Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echoes input"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string"}
                }
            })
        }
        async fn execute(&self, args: serde_json::Value) -> crate::Result<ToolResult> {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("no input");
            Ok(ToolResult::ok(text))
        }
    }

    struct FailTool;

    #[async_trait]
    impl crate::tools::traits::Tool for FailTool {
        fn name(&self) -> &str {
            "fail"
        }
        fn description(&self) -> &str {
            "Always fails"
        }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        async fn execute(&self, _args: serde_json::Value) -> crate::Result<ToolResult> {
            Err(crate::ZeniiError::Tool("forced failure".into()))
        }
    }

    fn make_server(autonomy: AutonomyLevel) -> ZeniiMcpServer {
        let registry = ToolRegistry::new();
        registry
            .register(Arc::new(EchoTool))
            .expect("register echo");
        registry
            .register(Arc::new(FailTool))
            .expect("register fail");

        let config = AppConfig::default();
        let security = SecurityPolicy::new(autonomy, None, vec![], 60, 60, 1000);

        ZeniiMcpServer::new(
            Arc::new(registry),
            Arc::new(security),
            Arc::new(ArcSwap::from_pointee(config)),
        )
    }

    #[test]
    fn get_info_returns_zenii() {
        let server = make_server(AutonomyLevel::Full);
        let info = server.get_info();
        assert_eq!(info.server_info.name, "zenii");
    }

    #[test]
    fn visible_tools_returns_all_by_default() {
        let server = make_server(AutonomyLevel::Full);
        let tools = server.visible_tools();
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn visible_tools_respects_hidden_list() {
        let server = make_server(AutonomyLevel::Full);
        {
            let mut cfg = (**server.config.load()).clone();
            cfg.mcp_server_hidden_tools = vec!["fail".into()];
            server.config.store(Arc::new(cfg));
        }
        let tools = server.visible_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
    }

    #[test]
    fn visible_tools_respects_exposed_list() {
        let server = make_server(AutonomyLevel::Full);
        {
            let mut cfg = (**server.config.load()).clone();
            cfg.mcp_server_exposed_tools = vec!["echo".into()];
            server.config.store(Arc::new(cfg));
        }
        let tools = server.visible_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");
    }

    #[test]
    fn resolve_rejects_hidden_tool() {
        let server = make_server(AutonomyLevel::Full);
        {
            let mut cfg = (**server.config.load()).clone();
            cfg.mcp_server_hidden_tools = vec!["echo".into()];
            server.config.store(Arc::new(cfg));
        }
        // Hidden tool must be indistinguishable from nonexistent
        assert!(server.resolve_tool_name("zenii_echo").is_none());
        // Non-hidden tool still resolves
        assert_eq!(server.resolve_tool_name("zenii_fail"), Some("fail"));
    }

    #[test]
    fn resolve_rejects_non_exposed_tool() {
        let server = make_server(AutonomyLevel::Full);
        {
            let mut cfg = (**server.config.load()).clone();
            cfg.mcp_server_exposed_tools = vec!["echo".into()];
            server.config.store(Arc::new(cfg));
        }
        // Tool not in allowlist must be rejected
        assert!(server.resolve_tool_name("zenii_fail").is_none());
        // Allowlisted tool resolves
        assert_eq!(server.resolve_tool_name("zenii_echo"), Some("echo"));
    }

    #[test]
    fn resolve_rejects_unprefixed_name() {
        let server = make_server(AutonomyLevel::Full);
        // Default prefix is "zenii_" — calling without prefix must fail
        assert!(server.resolve_tool_name("echo").is_none());
        assert!(server.resolve_tool_name("fail").is_none());
    }

    #[test]
    fn resolve_accepts_visible_prefixed_tool() {
        let server = make_server(AutonomyLevel::Full);
        assert_eq!(server.resolve_tool_name("zenii_echo"), Some("echo"));
        assert_eq!(server.resolve_tool_name("zenii_fail"), Some("fail"));
    }
}
