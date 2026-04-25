#[cfg(feature = "mcp-client")]
pub use inner::McpClientManager;
#[cfg(feature = "mcp-client")]
pub use inner::McpToolInfo;

#[cfg(feature = "mcp-client")]
mod inner {
    use std::collections::HashMap;

    use rmcp::RoleClient;
    use rmcp::service::Peer;
    use serde_json::Value;

    use crate::config::{McpServerConfig, McpTransport};
    use crate::{Result, ZeniiError};

    const CONNECT_TIMEOUT_SECS: u64 = 15;
    const DISCOVER_TIMEOUT_SECS: u64 = 10;
    const CALL_TIMEOUT_SECS: u64 = 60;

    /// Info about a single tool discovered from an MCP server.
    #[derive(Debug, Clone)]
    pub struct McpToolInfo {
        pub name: String,
        pub description: String,
        pub schema: Value,
    }

    /// Manages connections to external MCP servers and caches their tool lists.
    /// Each enabled stdio server gets a persistent child-process session whose
    /// `Peer<RoleClient>` is stored here for reuse across tool calls.
    pub struct McpClientManager {
        /// server_id → list of tools on that server
        tools: HashMap<String, Vec<McpToolInfo>>,
        /// server_id → config (needed for call_tool routing)
        configs: HashMap<String, McpServerConfig>,
        /// server_id → live rmcp peer (reused across calls, no respawn per call)
        peers: HashMap<String, Peer<RoleClient>>,
    }

    impl McpClientManager {
        /// Connect to all enabled servers, discover their tools.
        /// Servers that fail to connect are skipped with a warning rather than
        /// aborting the whole startup.
        pub async fn connect_all(configs: &[McpServerConfig]) -> Result<Self> {
            let mut tools: HashMap<String, Vec<McpToolInfo>> = HashMap::new();
            let mut cfg_map: HashMap<String, McpServerConfig> = HashMap::new();
            let mut peers: HashMap<String, Peer<RoleClient>> = HashMap::new();

            for cfg in configs {
                if !cfg.enabled {
                    continue;
                }
                cfg_map.insert(cfg.id.clone(), cfg.clone());

                match Self::connect_server(cfg).await {
                    Ok((server_tools, peer)) => {
                        tracing::info!(
                            server = %cfg.id,
                            count = server_tools.len(),
                            "mcp-client: discovered tools"
                        );
                        tools.insert(cfg.id.clone(), server_tools);
                        peers.insert(cfg.id.clone(), peer);
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
                peers,
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

        /// Execute a tool on the given server via the live session.
        pub async fn call_tool(
            &self,
            config: &McpServerConfig,
            tool_name: &str,
            args: Value,
        ) -> Result<Value> {
            match &config.transport {
                McpTransport::Stdio { .. } => self.call_stdio_tool(config, tool_name, args).await,
                McpTransport::Http { url, .. } => Err(ZeniiError::Mcp(format!(
                    "HTTP MCP transport not yet supported (server: {}, url: {})",
                    config.id, url
                ))),
            }
        }

        // ── Private helpers ───────────────────────────────────────────────────

        async fn connect_server(
            cfg: &McpServerConfig,
        ) -> Result<(Vec<McpToolInfo>, Peer<RoleClient>)> {
            match &cfg.transport {
                McpTransport::Stdio { .. } => Self::connect_stdio(cfg).await,
                McpTransport::Http { url, .. } => Err(ZeniiError::Mcp(format!(
                    "HTTP MCP transport not yet supported (server: {}, url: {})",
                    cfg.id, url
                ))),
            }
        }

        async fn connect_stdio(
            cfg: &McpServerConfig,
        ) -> Result<(Vec<McpToolInfo>, Peer<RoleClient>)> {
            use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
            use tokio::process::Command;

            let McpTransport::Stdio { command, args, env } = &cfg.transport else {
                unreachable!()
            };

            let env_clone = env.clone();
            let args_clone = args.clone();
            let command_clone = command.clone();
            let transport =
                TokioChildProcess::new(Command::new(&command_clone).configure(move |cmd| {
                    cmd.args(&args_clone);
                    for (k, v) in &env_clone {
                        cmd.env(k, v);
                    }
                }))
                .map_err(|e| {
                    ZeniiError::Mcp(format!("failed to spawn MCP server '{}': {e}", cfg.id))
                })?;

            let session = tokio::time::timeout(
                std::time::Duration::from_secs(CONNECT_TIMEOUT_SECS),
                rmcp::serve_client((), transport),
            )
            .await
            .map_err(|_| {
                ZeniiError::Mcp(format!(
                    "timed out initializing MCP server '{}' ({}s)",
                    cfg.id, CONNECT_TIMEOUT_SECS
                ))
            })?
            .map_err(|e| {
                ZeniiError::Mcp(format!("MCP server '{}' handshake failed: {e}", cfg.id))
            })?;

            let peer = session.peer().clone();

            let tool_list = tokio::time::timeout(
                std::time::Duration::from_secs(DISCOVER_TIMEOUT_SECS),
                peer.list_all_tools(),
            )
            .await
            .map_err(|_| {
                ZeniiError::Mcp(format!(
                    "timed out listing tools from '{}' ({}s)",
                    cfg.id, DISCOVER_TIMEOUT_SECS
                ))
            })?
            .map_err(|e| ZeniiError::Mcp(format!("failed to list tools from '{}': {e}", cfg.id)))?;

            let tool_infos: Vec<McpToolInfo> = tool_list
                .iter()
                .map(|t| McpToolInfo {
                    name: t.name.as_ref().to_string(),
                    description: t.description.as_deref().unwrap_or("").to_string(),
                    schema: serde_json::to_value(t.input_schema.as_ref())
                        .unwrap_or(serde_json::json!({"type": "object"})),
                })
                .collect();

            // Keep the session alive in a background task.
            // When the daemon shuts down, tokio aborts all tasks and the session is cleaned up.
            let server_id = cfg.id.clone();
            tokio::spawn(async move {
                let _session = session;
                std::future::pending::<()>().await;
                // Unreachable — only exits when the task is aborted by the runtime.
                drop(_session);
                tracing::debug!(server = %server_id, "mcp-client session task exiting");
            });

            Ok((tool_infos, peer))
        }

        async fn call_stdio_tool(
            &self,
            config: &McpServerConfig,
            tool_name: &str,
            args: Value,
        ) -> Result<Value> {
            use rmcp::model::CallToolRequestParams;

            let peer = self.peers.get(&config.id).ok_or_else(|| {
                ZeniiError::Mcp(format!(
                    "no active session for MCP server '{}' (connection failed at startup?)",
                    config.id
                ))
            })?;

            let arguments = match args {
                Value::Object(m) => Some(m),
                Value::Null => None,
                other => {
                    let mut m = serde_json::Map::new();
                    m.insert("value".into(), other);
                    Some(m)
                }
            };

            let mut params = CallToolRequestParams::new(tool_name.to_string());
            if let Some(a) = arguments {
                params = params.with_arguments(a);
            }

            let result = tokio::time::timeout(
                std::time::Duration::from_secs(CALL_TIMEOUT_SECS),
                peer.call_tool(params),
            )
            .await
            .map_err(|_| {
                ZeniiError::Mcp(format!(
                    "timed out calling '{}' on MCP server '{}' ({}s)",
                    tool_name, config.id, CALL_TIMEOUT_SECS
                ))
            })?
            .map_err(|e| {
                ZeniiError::Mcp(format!(
                    "MCP tool call '{}' on '{}' failed: {e}",
                    tool_name, config.id
                ))
            })?;

            if result.is_error.unwrap_or(false) {
                let error_text: String = result
                    .content
                    .iter()
                    .filter_map(|c| c.as_text())
                    .map(|t| t.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(ZeniiError::Mcp(format!(
                    "MCP tool '{}' returned error: {}",
                    tool_name, error_text
                )));
            }

            let output: String = result
                .content
                .iter()
                .filter_map(|c| c.as_text())
                .map(|t| t.text.as_str())
                .collect::<Vec<_>>()
                .join("\n");

            Ok(Value::String(output))
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
            // `false` exits immediately — session handshake will fail gracefully
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
        async fn call_tool_stdio_no_session_returns_error() {
            // Server was disabled, so no peer was stored
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
                peers: HashMap::new(),
            };
            assert!(manager.tools_for("nonexistent").is_empty());
        }

        #[test]
        fn config_for_returns_none_for_unknown() {
            let manager = McpClientManager {
                tools: HashMap::new(),
                configs: HashMap::new(),
                peers: HashMap::new(),
            };
            assert!(manager.config_for("x").is_none());
        }
    }
}
