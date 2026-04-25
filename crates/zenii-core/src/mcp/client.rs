#[cfg(feature = "mcp-client")]
pub use inner::McpClientManager;
#[cfg(feature = "mcp-client")]
pub use inner::McpToolInfo;

#[cfg(feature = "mcp-client")]
mod inner {
    use std::collections::HashMap;

    use serde_json::Value;

    use crate::config::{McpServerConfig, McpTransport};
    use crate::{Result, ZeniiError};

    /// Info about a single tool discovered from an MCP server.
    #[derive(Debug, Clone)]
    pub struct McpToolInfo {
        pub name: String,
        pub description: String,
        pub schema: Value,
    }

    /// Manages connections to external MCP servers and caches their tool lists.
    pub struct McpClientManager {
        /// server_id → list of tools on that server
        tools: HashMap<String, Vec<McpToolInfo>>,
        /// server_id → config (needed for call_tool)
        configs: HashMap<String, McpServerConfig>,
    }

    impl McpClientManager {
        /// Connect to all enabled servers, discover their tools.
        pub async fn connect_all(configs: &[McpServerConfig]) -> Result<Self> {
            let mut tools: HashMap<String, Vec<McpToolInfo>> = HashMap::new();
            let mut cfg_map: HashMap<String, McpServerConfig> = HashMap::new();
            for cfg in configs {
                if !cfg.enabled {
                    continue;
                }
                cfg_map.insert(cfg.id.clone(), cfg.clone());
                match Self::discover_server_tools(cfg).await {
                    Ok(server_tools) => {
                        tracing::info!(
                            server = %cfg.id,
                            count = server_tools.len(),
                            "mcp-client: discovered tools"
                        );
                        tools.insert(cfg.id.clone(), server_tools);
                    }
                    Err(e) => {
                        tracing::warn!(
                            server = %cfg.id,
                            error = %e,
                            "mcp-client: failed to connect"
                        );
                    }
                }
            }
            Ok(Self {
                tools,
                configs: cfg_map,
            })
        }

        /// Return tool infos for a connected server.
        pub fn tools_for(&self, server_id: &str) -> &[McpToolInfo] {
            self.tools.get(server_id).map(Vec::as_slice).unwrap_or(&[])
        }

        /// All (server_id, tool_info) pairs across every connected server.
        pub fn all_tools(&self) -> impl Iterator<Item = (&str, &McpToolInfo)> {
            self.tools
                .iter()
                .flat_map(|(id, tools)| tools.iter().map(move |t| (id.as_str(), t)))
        }

        /// Config for a server by id.
        pub fn config_for(&self, server_id: &str) -> Option<&McpServerConfig> {
            self.configs.get(server_id)
        }

        /// Execute a tool on the given server.
        pub async fn call_tool(
            &self,
            config: &McpServerConfig,
            tool_name: &str,
            args: Value,
        ) -> Result<Value> {
            Self::execute_tool(config, tool_name, args).await
        }

        async fn discover_server_tools(cfg: &McpServerConfig) -> Result<Vec<McpToolInfo>> {
            match &cfg.transport {
                McpTransport::Stdio { .. } => Self::discover_stdio(cfg).await,
                McpTransport::Http { url, .. } => Err(ZeniiError::Mcp(format!(
                    "HTTP MCP transport not yet supported (server: {}, url: {})",
                    cfg.id, url
                ))),
            }
        }

        async fn discover_stdio(cfg: &McpServerConfig) -> Result<Vec<McpToolInfo>> {
            // rmcp client feature not yet wired — return empty tool list and log.
            // When rmcp gains a stable client API, replace this stub with:
            //   use rmcp::transport::TokioChildProcess;
            //   let transport = TokioChildProcess::new(&mut cmd)?;
            //   let client = rmcp::ClientSession::new(transport).await?;
            //   let result = client.list_tools(Default::default()).await?;
            tracing::warn!(
                server = %cfg.id,
                "mcp-client: stdio discovery not yet implemented (stub)"
            );
            Err(ZeniiError::Mcp(format!(
                "MCP client not yet wired for server '{}'",
                cfg.id
            )))
        }

        async fn execute_tool(
            cfg: &McpServerConfig,
            tool_name: &str,
            args: Value,
        ) -> Result<Value> {
            let _ = args;
            match &cfg.transport {
                McpTransport::Stdio { .. } => {
                    // rmcp client feature not yet wired — stub.
                    Err(ZeniiError::Mcp(format!(
                        "MCP client not yet wired: cannot call '{}' on server '{}'",
                        tool_name, cfg.id
                    )))
                }
                McpTransport::Http { url, .. } => Err(ZeniiError::Mcp(format!(
                    "HTTP MCP transport not yet supported (server: {}, url: {})",
                    cfg.id, url
                ))),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::config::McpTransport;
        use std::collections::HashMap;

        fn stdio_cfg(id: &str, enabled: bool) -> McpServerConfig {
            McpServerConfig {
                id: id.into(),
                transport: McpTransport::Stdio {
                    command: "false".into(),
                    args: vec![],
                    env: HashMap::new(),
                },
                tools_prefix: None,
                enabled,
            }
        }

        fn http_cfg(id: &str) -> McpServerConfig {
            McpServerConfig {
                id: id.into(),
                transport: McpTransport::Http {
                    url: "http://localhost:9999".into(),
                    headers: HashMap::new(),
                },
                tools_prefix: None,
                enabled: true,
            }
        }

        #[tokio::test]
        async fn connect_all_skips_disabled() {
            let cfgs = vec![stdio_cfg("a", false), stdio_cfg("b", false)];
            let manager = McpClientManager::connect_all(&cfgs).await.unwrap();
            assert!(manager.tools_for("a").is_empty());
            assert!(manager.tools_for("b").is_empty());
        }

        #[tokio::test]
        async fn connect_all_tolerates_failed_server() {
            // Enabled servers that fail to connect should not abort connect_all
            let cfgs = vec![stdio_cfg("bad", true)];
            let manager = McpClientManager::connect_all(&cfgs).await.unwrap();
            assert!(manager.tools_for("bad").is_empty());
        }

        #[tokio::test]
        async fn connect_all_empty_list() {
            let manager = McpClientManager::connect_all(&[]).await.unwrap();
            assert_eq!(manager.all_tools().count(), 0);
        }

        #[tokio::test]
        async fn call_tool_http_returns_error() {
            let cfg = http_cfg("remote");
            let manager = McpClientManager::connect_all(&[]).await.unwrap();
            let result = manager.call_tool(&cfg, "ping", serde_json::json!({})).await;
            assert!(result.is_err());
            let msg = result.unwrap_err().to_string();
            assert!(msg.contains("HTTP MCP transport not yet supported"));
        }

        #[tokio::test]
        async fn call_tool_stdio_returns_stub_error() {
            let cfg = stdio_cfg("local", true);
            let manager = McpClientManager::connect_all(&[]).await.unwrap();
            let result = manager
                .call_tool(&cfg, "list_files", serde_json::json!({}))
                .await;
            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), ZeniiError::Mcp(_)));
        }

        #[test]
        fn tools_for_unknown_server_is_empty() {
            let manager = McpClientManager {
                tools: HashMap::new(),
                configs: HashMap::new(),
            };
            assert!(manager.tools_for("nonexistent").is_empty());
        }

        #[test]
        fn config_for_returns_none_for_unknown() {
            let manager = McpClientManager {
                tools: HashMap::new(),
                configs: HashMap::new(),
            };
            assert!(manager.config_for("x").is_none());
        }
    }
}
