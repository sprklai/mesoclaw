use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex;

use super::process::PluginProcess;
use crate::Result;
use crate::tools::{Tool, ToolResult};

/// Wraps a PluginProcess to implement the Tool trait.
/// This makes plugin tools indistinguishable from built-in tools.
pub struct PluginToolAdapter {
    name: String,
    description: String,
    parameters_schema: Value,
    process: Arc<Mutex<PluginProcess>>,
}

impl PluginToolAdapter {
    pub fn new(
        name: String,
        description: String,
        parameters_schema: Value,
        process: Arc<Mutex<PluginProcess>>,
    ) -> Self {
        Self {
            name,
            description,
            parameters_schema,
            process,
        }
    }
}

#[async_trait]
impl Tool for PluginToolAdapter {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters_schema(&self) -> Value {
        self.parameters_schema.clone()
    }

    async fn execute(&self, args: Value) -> Result<ToolResult> {
        let mut proc = self.process.lock().await;
        if !proc.is_running() {
            proc.spawn().await?;
        }
        proc.execute(args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::test_helpers::{has_interpreter, real_plugins_path};
    use std::path::PathBuf;

    fn mock_plugin_script() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let script_path = dir.path().join("mock-adapter.sh");
        std::fs::write(
            &script_path,
            r#"#!/bin/bash
while IFS= read -r line; do
    id=$(echo "$line" | grep -o '"id":[0-9]*' | head -1 | cut -d: -f2)
    method=$(echo "$line" | grep -o '"method":"[^"]*"' | head -1 | cut -d'"' -f4)
    case "$method" in
        info)
            echo "{\"jsonrpc\":\"2.0\",\"result\":{\"name\":\"adapter-test\",\"description\":\"Test\",\"version\":\"1.0.0\",\"parameters_schema\":{}},\"id\":$id}"
            ;;
        execute)
            echo "{\"jsonrpc\":\"2.0\",\"result\":{\"output\":\"adapter executed\",\"success\":true},\"id\":$id}"
            ;;
        shutdown)
            echo "{\"jsonrpc\":\"2.0\",\"result\":null,\"id\":$id}"
            exit 0
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

    // 9.0.11 — Adapter implements Tool trait
    #[tokio::test]
    async fn adapter_implements_tool_trait() {
        let (_dir, script_path) = mock_plugin_script();
        let process = PluginProcess::new("adapter-test", script_path, 10, 3);
        let process = Arc::new(Mutex::new(process));

        let adapter = PluginToolAdapter::new(
            "adapter-test".into(),
            "Test adapter".into(),
            serde_json::json!({}),
            process,
        );

        assert_eq!(adapter.name(), "adapter-test");
        assert_eq!(adapter.description(), "Test adapter");

        let result = adapter
            .execute(serde_json::json!({"action": "test"}))
            .await
            .unwrap();
        assert!(result.success);
        assert_eq!(result.output, "adapter executed");
    }

    // 9.0.12 — Adapter lazy-starts process
    #[tokio::test]
    async fn adapter_lazy_starts_process() {
        let (_dir, script_path) = mock_plugin_script();
        let process = PluginProcess::new("lazy-test", script_path, 10, 3);
        // Process is NOT spawned yet
        let process = Arc::new(Mutex::new(process));

        let adapter = PluginToolAdapter::new(
            "lazy-test".into(),
            "Test lazy start".into(),
            serde_json::json!({}),
            process.clone(),
        );

        // Verify not running
        {
            let mut proc = process.lock().await;
            assert!(!proc.is_running());
        }

        // Execute should auto-spawn
        let result = adapter
            .execute(serde_json::json!({"action": "test"}))
            .await
            .unwrap();
        assert!(result.success);

        // Now it should be running
        {
            let mut proc = process.lock().await;
            assert!(proc.is_running());
        }
    }

    // 9.1.31 — Adapter end-to-end: real word-count plugin
    #[tokio::test]
    async fn adapter_real_word_count() {
        let Some(plugins_path) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("python3") {
            eprintln!("SKIP: python3 interpreter not available");
            return;
        }

        let binary = plugins_path.join("word-count/word-count.py");
        let process = PluginProcess::new("word-count", binary, 30, 1);
        let process_arc = Arc::new(Mutex::new(process));

        // Spawn first
        {
            let mut proc = process_arc.lock().await;
            proc.spawn().await.unwrap();
        }

        let adapter = PluginToolAdapter::new(
            "word-count".into(),
            "Count words".into(),
            serde_json::json!({}),
            process_arc,
        );

        let result = adapter
            .execute(serde_json::json!({"action": "count", "text": "hello world"}))
            .await
            .unwrap();
        assert!(result.success, "expected success=true, got: {:?}", result);
        assert!(
            result.output.contains("Words: 2"),
            "expected output to contain 'Words: 2', got: {}",
            result.output
        );
    }

    // 9.1.32 — Adapter end-to-end: real json-formatter plugin
    #[tokio::test]
    async fn adapter_real_json_formatter() {
        let Some(plugins_path) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("node") {
            eprintln!("SKIP: node interpreter not available");
            return;
        }

        let binary = plugins_path.join("json-formatter/json-formatter.js");
        let process = PluginProcess::new("json-formatter", binary, 30, 1);
        let process_arc = Arc::new(Mutex::new(process));

        {
            let mut proc = process_arc.lock().await;
            proc.spawn().await.unwrap();
        }

        let adapter = PluginToolAdapter::new(
            "json-formatter".into(),
            "Format JSON".into(),
            serde_json::json!({}),
            process_arc,
        );

        let result = adapter
            .execute(serde_json::json!({"action": "validate", "json": "{\"a\":1}"}))
            .await
            .unwrap();
        assert!(result.success, "expected success=true, got: {:?}", result);
        assert!(
            result.output.to_lowercase().contains("valid"),
            "expected output to contain 'valid', got: {}",
            result.output
        );
    }

    // 9.1.33 — Adapter end-to-end: real uuid-gen plugin
    #[tokio::test]
    async fn adapter_real_uuid_gen() {
        let Some(plugins_path) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("bash") {
            eprintln!("SKIP: bash interpreter not available");
            return;
        }

        let binary = plugins_path.join("uuid-gen/uuid-gen.sh");
        let process = PluginProcess::new("uuid-gen", binary, 30, 1);
        let process_arc = Arc::new(Mutex::new(process));

        {
            let mut proc = process_arc.lock().await;
            proc.spawn().await.unwrap();
        }

        let adapter = PluginToolAdapter::new(
            "uuid-gen".into(),
            "Generate UUIDs".into(),
            serde_json::json!({}),
            process_arc,
        );

        let result = adapter
            .execute(serde_json::json!({"action": "generate", "count": 1}))
            .await
            .unwrap();
        assert!(result.success, "expected success=true, got: {:?}", result);
        assert!(
            result.output.contains('-') && result.output.len() > 10,
            "expected hyphenated UUID string, got: {}",
            result.output
        );
    }

    // 9.1.34 — Adapter end-to-end: real timestamp plugin
    #[tokio::test]
    async fn adapter_real_timestamp() {
        let Some(plugins_path) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("node") {
            eprintln!("SKIP: node interpreter not available");
            return;
        }

        let binary = plugins_path.join("timestamp/timestamp.js");
        let process = PluginProcess::new("timestamp", binary, 30, 1);
        let process_arc = Arc::new(Mutex::new(process));

        {
            let mut proc = process_arc.lock().await;
            proc.spawn().await.unwrap();
        }

        let adapter = PluginToolAdapter::new(
            "timestamp".into(),
            "Get timestamps".into(),
            serde_json::json!({}),
            process_arc,
        );

        let result = adapter
            .execute(serde_json::json!({"action": "now"}))
            .await
            .unwrap();
        assert!(result.success, "expected success=true, got: {:?}", result);
    }

    // 9.1.35 — Adapter end-to-end: lazy-start with real plugin
    #[tokio::test]
    async fn adapter_real_lazy_start() {
        let Some(plugins_path) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        if !has_interpreter("python3") {
            eprintln!("SKIP: python3 interpreter not available");
            return;
        }

        let binary = plugins_path.join("word-count/word-count.py");
        let process = PluginProcess::new("word-count", binary, 30, 1);
        // Do NOT spawn — adapter should lazy-start
        let process_arc = Arc::new(Mutex::new(process));

        let adapter = PluginToolAdapter::new(
            "word-count".into(),
            "Count words".into(),
            serde_json::json!({}),
            process_arc,
        );

        let result = adapter
            .execute(serde_json::json!({"action": "count", "text": "one two three"}))
            .await
            .unwrap();
        assert!(result.success, "expected success=true, got: {:?}", result);
        assert!(
            result.output.contains("Words"),
            "expected output to contain 'Words', got: {}",
            result.output
        );
    }
}
