use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::{debug, warn};

use crate::tools::ToolResult;
use crate::{Result, ZeniiError};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
    id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Info returned by the `info` JSON-RPC method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginToolInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub parameters_schema: Value,
}

/// Manages an external plugin process lifecycle + JSON-RPC communication.
pub struct PluginProcess {
    name: String,
    binary_path: PathBuf,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout_reader: Option<BufReader<ChildStdout>>,
    request_id: AtomicU64,
    execute_timeout: Duration,
    restart_attempts: u32,
    max_restart_attempts: u32,
}

impl PluginProcess {
    pub fn new(
        name: &str,
        binary_path: PathBuf,
        execute_timeout_secs: u64,
        max_restart_attempts: u32,
    ) -> Self {
        Self {
            name: name.to_string(),
            binary_path,
            child: None,
            stdin: None,
            stdout_reader: None,
            request_id: AtomicU64::new(1),
            execute_timeout: Duration::from_secs(execute_timeout_secs),
            restart_attempts: 0,
            max_restart_attempts,
        }
    }

    /// Spawn the plugin process.
    pub async fn spawn(&mut self) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }

        debug!(
            "Spawning plugin process: {} ({})",
            self.name,
            self.binary_path.display()
        );

        let mut child = Command::new(&self.binary_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                ZeniiError::Plugin(format!("failed to spawn plugin '{}': {e}", self.name))
            })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            ZeniiError::Plugin(format!("plugin '{}' stdin not available", self.name))
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ZeniiError::Plugin(format!("plugin '{}' stdout not available", self.name))
        })?;

        self.child = Some(child);
        self.stdin = Some(stdin);
        self.stdout_reader = Some(BufReader::new(stdout));
        self.restart_attempts = 0;

        debug!("Plugin '{}' spawned successfully", self.name);
        Ok(())
    }

    /// Check if the process is running.
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            matches!(child.try_wait(), Ok(None))
        } else {
            false
        }
    }

    /// Send a JSON-RPC call and wait for the response.
    async fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        if !self.is_running() {
            self.try_restart().await?;
        }

        let id = self.request_id.fetch_add(1, Ordering::Relaxed);
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
            id,
        };

        let mut request_line = serde_json::to_string(&request)
            .map_err(|e| ZeniiError::Plugin(format!("failed to serialize request: {e}")))?;
        request_line.push('\n');

        let stdin = self.stdin.as_mut().ok_or_else(|| {
            ZeniiError::Plugin(format!("plugin '{}' stdin not available", self.name))
        })?;
        let reader = self.stdout_reader.as_mut().ok_or_else(|| {
            ZeniiError::Plugin(format!("plugin '{}' stdout not available", self.name))
        })?;

        // Write request
        stdin
            .write_all(request_line.as_bytes())
            .await
            .map_err(|e| {
                ZeniiError::Plugin(format!("failed to write to plugin '{}': {e}", self.name))
            })?;
        stdin.flush().await.map_err(|e| {
            ZeniiError::Plugin(format!("failed to flush plugin '{}': {e}", self.name))
        })?;

        // Read response with timeout
        let mut response_line = String::new();
        let read_result =
            tokio::time::timeout(self.execute_timeout, reader.read_line(&mut response_line)).await;

        match read_result {
            Ok(Ok(0)) => {
                // Process closed stdout — it crashed
                self.cleanup();
                Err(ZeniiError::Plugin(format!(
                    "plugin '{}' closed unexpectedly",
                    self.name
                )))
            }
            Ok(Ok(_)) => {
                let response: JsonRpcResponse = serde_json::from_str(response_line.trim())
                    .map_err(|e| {
                        ZeniiError::Plugin(format!(
                            "plugin '{}' invalid JSON-RPC response: {e}",
                            self.name
                        ))
                    })?;

                if let Some(error) = response.error {
                    Err(ZeniiError::Plugin(format!(
                        "plugin '{}' error ({}): {}",
                        self.name, error.code, error.message
                    )))
                } else {
                    Ok(response.result.unwrap_or(Value::Null))
                }
            }
            Ok(Err(e)) => {
                self.cleanup();
                Err(ZeniiError::Plugin(format!(
                    "plugin '{}' read error: {e}",
                    self.name
                )))
            }
            Err(_) => {
                // Timeout
                self.cleanup();
                Err(ZeniiError::Plugin(format!(
                    "plugin '{}' execute timed out after {}s",
                    self.name,
                    self.execute_timeout.as_secs()
                )))
            }
        }
    }

    /// Call the `info` JSON-RPC method.
    pub async fn info(&mut self) -> Result<PluginToolInfo> {
        let result = self.call("info", None).await?;
        serde_json::from_value(result).map_err(|e| {
            ZeniiError::Plugin(format!("plugin '{}' invalid info response: {e}", self.name))
        })
    }

    /// Call the `execute` JSON-RPC method.
    pub async fn execute(&mut self, args: Value) -> Result<ToolResult> {
        let result = self.call("execute", Some(args)).await?;

        let output = result
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let success = result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let metadata = result.get("metadata").cloned();

        Ok(ToolResult {
            output,
            success,
            metadata,
        })
    }

    /// Send a shutdown request and wait for exit.
    pub async fn shutdown(&mut self) -> Result<()> {
        if !self.is_running() {
            return Ok(());
        }

        // Try graceful shutdown
        let _ = self.call("shutdown", None).await;

        // Wait briefly for process to exit
        if let Some(ref mut child) = self.child {
            let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
        }

        self.cleanup();
        debug!("Plugin '{}' shut down", self.name);
        Ok(())
    }

    /// Try to restart after crash, with exponential backoff.
    async fn try_restart(&mut self) -> Result<()> {
        if self.restart_attempts >= self.max_restart_attempts {
            return Err(ZeniiError::Plugin(format!(
                "plugin '{}' exceeded max restart attempts ({})",
                self.name, self.max_restart_attempts
            )));
        }

        self.restart_attempts += 1;
        let backoff_ms = 100 * (1u64 << self.restart_attempts.min(10));
        warn!(
            "Plugin '{}' restarting (attempt {}/{}), backoff {}ms",
            self.name, self.restart_attempts, self.max_restart_attempts, backoff_ms
        );

        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        self.cleanup();
        self.spawn().await
    }

    fn cleanup(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
        self.stdin = None;
        self.stdout_reader = None;
    }
}

impl Drop for PluginProcess {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_plugin_script() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let script_path = dir.path().join("mock-plugin.sh");
        // A simple bash script that acts as a JSON-RPC plugin
        std::fs::write(
            &script_path,
            r#"#!/bin/bash
while IFS= read -r line; do
    method=$(echo "$line" | python3 -c "import sys,json; print(json.loads(sys.stdin.read())['method'])" 2>/dev/null || echo "unknown")
    id=$(echo "$line" | python3 -c "import sys,json; print(json.loads(sys.stdin.read())['id'])" 2>/dev/null || echo "0")

    # Re-parse since stdin was consumed
    id=$(echo "$line" | grep -o '"id":[0-9]*' | head -1 | cut -d: -f2)
    method=$(echo "$line" | grep -o '"method":"[^"]*"' | head -1 | cut -d'"' -f4)

    case "$method" in
        info)
            echo "{\"jsonrpc\":\"2.0\",\"result\":{\"name\":\"mock\",\"description\":\"Mock plugin\",\"version\":\"1.0.0\",\"parameters_schema\":{}},\"id\":$id}"
            ;;
        execute)
            echo "{\"jsonrpc\":\"2.0\",\"result\":{\"output\":\"executed\",\"success\":true},\"id\":$id}"
            ;;
        shutdown)
            echo "{\"jsonrpc\":\"2.0\",\"result\":null,\"id\":$id}"
            exit 0
            ;;
        *)
            echo "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32601,\"message\":\"Method not found\"},\"id\":$id}"
            ;;
    esac
done
"#,
        )
        .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Sync to avoid "Text file busy" race on Linux
        let f = std::fs::File::open(&script_path).unwrap();
        f.sync_all().unwrap();

        (dir, script_path)
    }

    // 9.0.6 — Spawn mock plugin process
    #[tokio::test]
    async fn spawn_mock_plugin_process() {
        let (_dir, script_path) = mock_plugin_script();
        let mut process = PluginProcess::new("mock", script_path, 10, 3);
        assert!(!process.is_running());
        process.spawn().await.unwrap();
        assert!(process.is_running());
        process.shutdown().await.unwrap();
    }

    // 9.0.7 — JSON-RPC info call
    #[tokio::test]
    async fn jsonrpc_info_call() {
        let (_dir, script_path) = mock_plugin_script();
        let mut process = PluginProcess::new("mock", script_path, 10, 3);
        process.spawn().await.unwrap();
        let info = process.info().await.unwrap();
        assert_eq!(info.name, "mock");
        assert_eq!(info.version, "1.0.0");
        process.shutdown().await.unwrap();
    }

    // 9.0.8 — JSON-RPC execute call
    #[tokio::test]
    async fn jsonrpc_execute_call() {
        let (_dir, script_path) = mock_plugin_script();
        let mut process = PluginProcess::new("mock", script_path, 10, 3);
        process.spawn().await.unwrap();
        let result = process
            .execute(serde_json::json!({"action": "test"}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output, "executed");
        process.shutdown().await.unwrap();
    }

    // 9.0.9 — Process timeout handling
    #[tokio::test]
    async fn process_timeout_handling() {
        let dir = tempfile::TempDir::new().unwrap();
        let script_path = dir.path().join("slow-plugin.sh");
        std::fs::write(
            &script_path,
            "#!/bin/bash\nwhile IFS= read -r line; do\nsleep 10\ndone\n",
        )
        .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Brief yield to avoid "Text file busy" race on Linux
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut process = PluginProcess::new("slow", script_path, 1, 0);
        process.spawn().await.unwrap();
        let result = process.execute(serde_json::json!({})).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    // 9.0.10 — Process crash recovery
    #[tokio::test]
    async fn process_crash_recovery() {
        let dir = tempfile::TempDir::new().unwrap();
        let script_path = dir.path().join("crash-plugin.sh");
        // This script exits immediately, simulating a crash
        std::fs::write(&script_path, "#!/bin/bash\nexit 1\n").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Brief yield to avoid "Text file busy" race on Linux
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut process = PluginProcess::new("crash", script_path, 2, 2);
        process.spawn().await.unwrap();

        // Process should have exited, so next call triggers restart attempts
        // Eventually it should exhaust restart attempts
        let result = process.execute(serde_json::json!({})).await;
        // Could succeed on restart or fail with max attempts
        // The point is it doesn't panic
        if result.is_err() {
            let err = result.unwrap_err().to_string();
            assert!(
                err.contains("exceeded max restart") || err.contains("closed unexpectedly"),
                "Unexpected error: {err}"
            );
        }
    }

    // ── Group D: Real plugin integration tests (9.1.21–9.1.30) ──

    use crate::plugins::test_helpers::{has_interpreter, real_plugins_path};

    // 9.1.21 — real_word_count_info
    #[tokio::test]
    async fn real_word_count_info() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("python3") {
            eprintln!("SKIP: python3 interpreter not available");
            return;
        }
        let binary = plugins.join("word-count/word-count.py");
        let mut proc = PluginProcess::new("word-count", binary, 30, 1);
        proc.spawn().await.unwrap();
        let info = proc.info().await.unwrap();
        assert_eq!(info.name, "word-count");
        assert_eq!(info.version, "1.0.0");
        assert!(info.parameters_schema.get("properties").is_some());
        proc.shutdown().await.unwrap();
    }

    // 9.1.22 — real_word_count_execute
    #[tokio::test]
    async fn real_word_count_execute() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("python3") {
            eprintln!("SKIP: python3 interpreter not available");
            return;
        }
        let binary = plugins.join("word-count/word-count.py");
        let mut proc = PluginProcess::new("word-count", binary, 30, 1);
        proc.spawn().await.unwrap();
        let result = proc
            .execute(serde_json::json!({"action": "count", "text": "hello world"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(
            result.output.contains("Words: 2"),
            "Expected output to contain 'Words: 2', got: {}",
            result.output
        );
        proc.shutdown().await.unwrap();
    }

    // 9.1.23 — real_json_formatter_info
    #[tokio::test]
    async fn real_json_formatter_info() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("node") {
            eprintln!("SKIP: node interpreter not available");
            return;
        }
        let binary = plugins.join("json-formatter/json-formatter.js");
        let mut proc = PluginProcess::new("json-formatter", binary, 30, 1);
        proc.spawn().await.unwrap();
        let info = proc.info().await.unwrap();
        assert_eq!(info.name, "json-formatter");
        assert_eq!(info.version, "1.0.0");
        proc.shutdown().await.unwrap();
    }

    // 9.1.24 — real_json_formatter_execute
    #[tokio::test]
    async fn real_json_formatter_execute() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("node") {
            eprintln!("SKIP: node interpreter not available");
            return;
        }
        let binary = plugins.join("json-formatter/json-formatter.js");
        let mut proc = PluginProcess::new("json-formatter", binary, 30, 1);
        proc.spawn().await.unwrap();
        let result = proc
            .execute(serde_json::json!({"action": "validate", "json": "{\"a\":1}"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(
            result.output.to_lowercase().contains("valid"),
            "Expected output to contain 'valid' (case-insensitive), got: {}",
            result.output
        );
        proc.shutdown().await.unwrap();
    }

    // 9.1.25 — real_uuid_gen_info
    #[tokio::test]
    async fn real_uuid_gen_info() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("bash") {
            eprintln!("SKIP: bash interpreter not available");
            return;
        }
        let binary = plugins.join("uuid-gen/uuid-gen.sh");
        let mut proc = PluginProcess::new("uuid-gen", binary, 30, 1);
        proc.spawn().await.unwrap();
        let info = proc.info().await.unwrap();
        assert_eq!(info.name, "uuid-gen");
        assert_eq!(info.version, "1.0.0");
        proc.shutdown().await.unwrap();
    }

    // 9.1.26 — real_uuid_gen_execute
    #[tokio::test]
    async fn real_uuid_gen_execute() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("bash") {
            eprintln!("SKIP: bash interpreter not available");
            return;
        }
        let binary = plugins.join("uuid-gen/uuid-gen.sh");
        let mut proc = PluginProcess::new("uuid-gen", binary, 30, 1);
        proc.spawn().await.unwrap();
        let result = proc
            .execute(serde_json::json!({"action": "generate", "count": 1}))
            .await
            .unwrap();
        assert!(result.success);
        // Check for UUID v4 pattern (8-4-4-4-12 hex format) using simple string validation
        let output_lower = result.output.to_lowercase();
        let has_uuid = output_lower.split_whitespace().any(|word| {
            let word = word.trim_matches(|c: char| !c.is_ascii_hexdigit() && c != '-');
            let parts: Vec<&str> = word.split('-').collect();
            parts.len() == 5
                && parts[0].len() == 8
                && parts[1].len() == 4
                && parts[2].len() == 4
                && parts[2].starts_with('4')
                && parts[3].len() == 4
                && parts[4].len() == 12
                && word.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
        });
        assert!(
            has_uuid,
            "Expected output to contain a UUID v4, got: {}",
            result.output
        );
        proc.shutdown().await.unwrap();
    }

    // 9.1.27 — real_timestamp_info
    #[tokio::test]
    async fn real_timestamp_info() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("node") {
            eprintln!("SKIP: node interpreter not available");
            return;
        }
        let binary = plugins.join("timestamp/timestamp.js");
        let mut proc = PluginProcess::new("timestamp", binary, 30, 1);
        proc.spawn().await.unwrap();
        let info = proc.info().await.unwrap();
        assert_eq!(info.name, "timestamp");
        assert_eq!(info.version, "1.0.0");
        proc.shutdown().await.unwrap();
    }

    // 9.1.28 — real_timestamp_execute
    #[tokio::test]
    async fn real_timestamp_execute() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("node") {
            eprintln!("SKIP: node interpreter not available");
            return;
        }
        let binary = plugins.join("timestamp/timestamp.js");
        let mut proc = PluginProcess::new("timestamp", binary, 30, 1);
        proc.spawn().await.unwrap();
        let result = proc
            .execute(serde_json::json!({"action": "now"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(
            result.output.to_lowercase().contains("epoch"),
            "Expected output to contain 'epoch' (case-insensitive), got: {}",
            result.output
        );
        proc.shutdown().await.unwrap();
    }

    // 9.1.29 — real_base64_info
    #[tokio::test]
    async fn real_base64_info() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        let base64_bin = plugins.join("base64-tool/base64-tool");
        if !base64_bin.exists() {
            eprintln!("SKIP: base64-tool binary not built");
            return;
        }
        let mut proc = PluginProcess::new("base64-tool", base64_bin, 30, 1);
        proc.spawn().await.unwrap();
        let info = proc.info().await.unwrap();
        assert_eq!(info.name, "base64-tool");
        proc.shutdown().await.unwrap();
    }

    // 9.1.30 — real_base64_execute
    #[tokio::test]
    async fn real_base64_execute() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        let base64_bin = plugins.join("base64-tool/base64-tool");
        if !base64_bin.exists() {
            eprintln!("SKIP: base64-tool binary not built");
            return;
        }
        let mut proc = PluginProcess::new("base64-tool", base64_bin, 30, 1);
        proc.spawn().await.unwrap();
        let result = proc
            .execute(serde_json::json!({"action": "encode", "input": "hello"}))
            .await
            .unwrap();
        assert!(result.success);
        assert!(
            result.output.contains("aGVsbG8"),
            "Expected output to contain 'aGVsbG8' (base64 of 'hello'), got: {}",
            result.output
        );
        proc.shutdown().await.unwrap();
    }
}
