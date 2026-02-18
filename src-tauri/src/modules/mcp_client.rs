//! MCP (Model Context Protocol) client for sidecar modules.
//!
//! When a module manifest specifies `type = "mcp"`, the module is an MCP
//! server that exposes a dynamic set of tools via the JSON-RPC 2.0 protocol
//! over stdin/stdout.
//!
//! # Lifecycle
//! 1. On module load: spawn the server process.
//! 2. Send `initialize` to complete the MCP handshake.
//! 3. Send `tools/list` to discover available tools.
//! 4. Register each discovered tool as a [`McpToolProxy`] in the `ToolRegistry`
//!    with the naming convention `mcp:{module_id}:{tool_name}`.
//! 5. When an agent calls a tool: forward to `tools/call` and return the result.
//! 6. Periodic `ping` keeps the server alive; restart on failure.
//!
//! # Feature gate
//! Compiled only with `--features mcp-client`.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::Mutex,
};

use crate::tools::{Tool, ToolResult};

use super::manifest::ModuleManifest;

// ─── JSON-RPC 2.0 types ───────────────────────────────────────────────────────

/// A JSON-RPC 2.0 request sent to the MCP server.
#[derive(Debug, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A JSON-RPC 2.0 response received from the MCP server.
#[derive(Debug, Deserialize)]
pub struct JsonRpcResponse {
    pub id: Value,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    /// Convert into `Ok(result)` or `Err(message)`.
    pub fn into_result(self) -> Result<Value, String> {
        if let Some(err) = self.error {
            return Err(format!(
                "MCP error {} ({}): {}",
                err.code,
                err.message,
                err.data.unwrap_or(Value::Null)
            ));
        }
        Ok(self.result.unwrap_or(Value::Null))
    }
}

// ─── MCP tool descriptor ──────────────────────────────────────────────────────

/// A tool exposed by an MCP server, as returned by `tools/list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name as reported by the server.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema describing the tool's input parameters.
    #[serde(rename = "inputSchema", default)]
    pub input_schema: Value,
}

// ─── McpClient internals ──────────────────────────────────────────────────────

/// Live process state: the open stdin/stdout handles plus a monotonic request id.
struct McpProcess {
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    next_id: u64,
}

impl McpProcess {
    fn next_id(&mut self) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        Value::Number(id.into())
    }

    async fn call(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<JsonRpcResponse, String> {
        let id = self.next_id();
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        };

        // Serialize and send (newline-delimited).
        let mut line =
            serde_json::to_string(&req).map_err(|e| format!("MCP serialize error: {e}"))?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("MCP write error: {e}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| format!("MCP flush error: {e}"))?;

        // Read one response line.
        let mut resp_line = String::new();
        self.stdout
            .read_line(&mut resp_line)
            .await
            .map_err(|e| format!("MCP read error: {e}"))?;

        if resp_line.is_empty() {
            return Err("MCP server closed the connection".to_string());
        }

        serde_json::from_str::<JsonRpcResponse>(&resp_line)
            .map_err(|e| format!("MCP deserialize error: {e}"))
    }
}

// ─── McpClient ────────────────────────────────────────────────────────────────

/// Manages a long-running MCP server process.
///
/// All method calls are serialized via an internal `Mutex` so that concurrent
/// tool invocations don't interleave requests/responses on the shared stdio stream.
pub struct McpClient {
    module_id: String,
    process: Mutex<Option<McpProcess>>,
    manifest: ModuleManifest,
    /// Tools discovered during `initialize()`.
    tools: Mutex<Vec<McpTool>>,
}

impl McpClient {
    /// Create a new client (process not yet started).
    pub fn new(manifest: ModuleManifest) -> Self {
        let module_id = manifest.module.id.clone();
        Self {
            module_id,
            process: Mutex::new(None),
            manifest,
            tools: Mutex::new(vec![]),
        }
    }

    /// Start the server process and complete the MCP handshake.
    ///
    /// Returns the list of tools advertised by the server.
    pub async fn start(&self) -> Result<Vec<McpTool>, String> {
        let mut proc_guard = self.process.lock().await;

        // Spawn the server process.
        let mut child = tokio::process::Command::new(&self.manifest.runtime.command)
            .args(&self.manifest.runtime.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| {
                format!(
                    "failed to start MCP server '{}': {e}",
                    self.manifest.runtime.command
                )
            })?;

        let stdin = child.stdin.take().ok_or("MCP child has no stdin")?;
        let stdout_raw = child.stdout.take().ok_or("MCP child has no stdout")?;
        let stdout = BufReader::new(stdout_raw);

        let mut proc = McpProcess {
            stdin,
            stdout,
            next_id: 0,
        };

        // ── MCP handshake ────────────────────────────────────────────────────
        // 1. initialize
        let init_params = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "mesoclaw",
                "version": env!("CARGO_PKG_VERSION")
            }
        });
        proc.call("initialize", Some(init_params))
            .await
            .and_then(|r| r.into_result())?;

        // 2. initialized notification (fire-and-forget, no response expected)
        let notif = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let mut notif_line = serde_json::to_string(&notif)
            .map_err(|e| format!("MCP notification serialize error: {e}"))?;
        notif_line.push('\n');
        proc.stdin
            .write_all(notif_line.as_bytes())
            .await
            .map_err(|e| format!("MCP notification write error: {e}"))?;

        // 3. tools/list — discover available tools
        let tools_result = proc.call("tools/list", None).await?.into_result()?;

        let tools: Vec<McpTool> = tools_result
            .get("tools")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        *proc_guard = Some(proc);

        let mut tool_guard = self.tools.lock().await;
        *tool_guard = tools.clone();

        log::info!(
            "McpClient: module '{}' started, {} tool(s) discovered",
            self.module_id,
            tools.len()
        );

        Ok(tools)
    }

    /// Stop the server process.
    pub async fn stop(&self) {
        let mut proc_guard = self.process.lock().await;
        *proc_guard = None; // Dropping McpProcess closes stdin → server sees EOF.
        log::info!("McpClient: module '{}' stopped", self.module_id);
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(&self, tool_name: &str, input: Value) -> Result<Value, String> {
        let mut proc_guard = self.process.lock().await;
        let proc = proc_guard
            .as_mut()
            .ok_or_else(|| format!("MCP server '{}' is not running", self.module_id))?;

        let params = serde_json::json!({
            "name": tool_name,
            "arguments": input
        });

        let resp = proc.call("tools/call", Some(params)).await?;
        let result = resp.into_result()?;

        // MCP tools/call result: `{ "content": [...], "isError": bool }`
        if result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let msg = result
                .get("content")
                .and_then(|c| c.as_array())
                .and_then(|arr| arr.first())
                .and_then(|item| item.get("text"))
                .and_then(|t| t.as_str())
                .unwrap_or("tool returned an error");
            return Err(msg.to_string());
        }

        Ok(result)
    }

    /// Return the module ID.
    pub fn module_id(&self) -> &str {
        &self.module_id
    }

    /// Return the most recently discovered tool list.
    pub async fn tools(&self) -> Vec<McpTool> {
        self.tools.lock().await.clone()
    }
}

// ─── McpToolProxy ─────────────────────────────────────────────────────────────

/// A `Tool` implementation that delegates execution to an [`McpClient`].
///
/// The tool name follows the convention `mcp:{module_id}:{tool_name}`.
pub struct McpToolProxy {
    /// Fully-qualified tool name: `mcp:{module_id}:{tool_name}`.
    full_name: String,
    /// Tool name as reported by the MCP server.
    tool_name: String,
    description: String,
    input_schema: Value,
    client: Arc<McpClient>,
}

impl McpToolProxy {
    /// Create a new proxy for a single MCP tool.
    pub fn new(module_id: &str, tool: &McpTool, client: Arc<McpClient>) -> Self {
        Self {
            full_name: format!("mcp:{}:{}", module_id, tool.name),
            tool_name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
            client,
        }
    }
}

#[async_trait]
impl Tool for McpToolProxy {
    fn name(&self) -> &str {
        &self.full_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.input_schema.clone()
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        match self.client.call_tool(&self.tool_name, args).await {
            Ok(value) => {
                let output =
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string());
                Ok(ToolResult::ok(output))
            }
            Err(e) => Ok(ToolResult::err(e)),
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── JSON-RPC serialization ────────────────────────────────────────────────

    #[test]
    fn jsonrpc_request_serializes_correctly() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: Value::Number(1.into()),
            method: "tools/list".to_string(),
            params: None,
        };
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("\"jsonrpc\":\"2.0\""));
        assert!(s.contains("\"method\":\"tools/list\""));
        assert!(s.contains("\"id\":1"));
        // params omitted when None
        assert!(!s.contains("params"));
    }

    #[test]
    fn jsonrpc_request_with_params_serializes() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0",
            id: Value::String("abc".to_string()),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({"name": "my-tool", "arguments": {}})),
        };
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("\"params\""));
        assert!(s.contains("\"name\":\"my-tool\""));
    }

    #[test]
    fn jsonrpc_response_ok_deserializes() {
        let json = r#"{"id":1,"result":{"tools":[]}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn jsonrpc_response_error_deserializes() {
        let json = r#"{"id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
    }

    #[test]
    fn jsonrpc_response_into_result_ok() {
        let json = r#"{"id":1,"result":{"value":42}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let result = resp.into_result().unwrap();
        assert_eq!(result["value"], 42);
    }

    #[test]
    fn jsonrpc_response_into_result_error() {
        let json = r#"{"id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let resp: JsonRpcResponse = serde_json::from_str(json).unwrap();
        let err = resp.into_result().unwrap_err();
        assert!(err.contains("Invalid Request"));
    }

    // ── McpTool ───────────────────────────────────────────────────────────────

    #[test]
    fn mcp_tool_deserializes() {
        let json =
            r#"{"name":"my-tool","description":"Does stuff","inputSchema":{"type":"object"}}"#;
        let tool: McpTool = serde_json::from_str(json).unwrap();
        assert_eq!(tool.name, "my-tool");
        assert_eq!(tool.description, "Does stuff");
        assert_eq!(tool.input_schema["type"], "object");
    }

    #[test]
    fn mcp_tool_missing_schema_defaults_to_null() {
        let json = r#"{"name":"t","description":"d"}"#;
        let tool: McpTool = serde_json::from_str(json).unwrap();
        assert!(tool.input_schema.is_null());
    }

    // ── McpToolProxy ──────────────────────────────────────────────────────────

    #[test]
    fn mcp_tool_proxy_name_follows_convention() {
        use crate::modules::manifest::{
            ModuleInfo, ModuleType, ParametersConfig, RuntimeConfig, RuntimeType, SecurityConfig,
        };
        use std::collections::HashMap;

        let manifest = ModuleManifest {
            module: ModuleInfo {
                id: "my-module".to_string(),
                name: "My Module".to_string(),
                version: "1.0.0".to_string(),
                description: "test".to_string(),
                module_type: ModuleType::Mcp,
            },
            runtime: RuntimeConfig {
                runtime_type: RuntimeType::Native,
                command: "mcp-server".to_string(),
                args: vec![],
                env: HashMap::new(),
                timeout_secs: None,
            },
            security: SecurityConfig::default(),
            parameters: ParametersConfig::default(),
        };

        let client = Arc::new(McpClient::new(manifest));
        let tool = McpTool {
            name: "do-thing".to_string(),
            description: "Does a thing".to_string(),
            input_schema: Value::Null,
        };

        let proxy = McpToolProxy::new("my-module", &tool, client);
        assert_eq!(proxy.name(), "mcp:my-module:do-thing");
        assert_eq!(proxy.description(), "Does a thing");
    }
}
