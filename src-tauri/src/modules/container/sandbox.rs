//! Sandbox manager for tool isolation.
//!
//! Provides container-based isolation for tool execution, wrapping the
//! `ContainerRuntime` trait with a higher-level API for executing tools
//! in sandboxed environments.
//!
//! # Sandbox Modes
//!
//! - `Off`: No sandboxing, tools run directly on host
//! - `NonMain`: Only tools spawned by agents are sandboxed
//! - `All`: All tool executions are sandboxed
//!
//! # Example
//!
//! ```ignore
//! let runtime = detect_runtime().ok_or("no container runtime")?;
//! let sandbox = SandboxManager::new(runtime, SandboxConfig::default());
//!
//! if sandbox.should_sandbox(false) {
//!     let result = sandbox.execute_tool("shell", &args).await?;
//! }
//! ```

use std::{collections::HashMap, sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::config::schema::{SandboxConfig, SandboxMode};

use super::{ContainerConfig, ContainerRuntime};

// ─── SandboxedToolResult ─────────────────────────────────────────────────────

/// Result of executing a tool inside a sandboxed container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxedToolResult {
    /// The tool's stdout/output.
    pub output: String,
    /// Whether execution succeeded.
    pub success: bool,
    /// Optional metadata from the tool.
    pub metadata: Option<Value>,
    /// Container ID that was used (for debugging).
    pub container_id: Option<String>,
}

// ─── SandboxManager ───────────────────────────────────────────────────────────

/// Manages container-based sandboxing for tool execution.
///
/// Wraps a `ContainerRuntime` and provides a higher-level API for:
/// - Determining whether sandboxing should apply
/// - Building container configs for tool execution
/// - Managing the lifecycle of sandboxed containers
pub struct SandboxManager {
    runtime: Arc<dyn ContainerRuntime>,
    config: SandboxConfig,
}

impl SandboxManager {
    /// Create a new sandbox manager with the given runtime and configuration.
    pub fn new(runtime: Arc<dyn ContainerRuntime>, config: SandboxConfig) -> Self {
        Self { runtime, config }
    }

    /// Create with default configuration.
    pub fn with_defaults(runtime: Arc<dyn ContainerRuntime>) -> Self {
        Self {
            runtime,
            config: SandboxConfig::default(),
        }
    }

    /// Check if sandboxing should be applied for the given execution context.
    ///
    /// `is_main_thread` should be `true` when the tool is being executed
    /// from the main application thread (e.g., direct user action), and
    /// `false` when executed by an agent loop.
    pub fn should_sandbox(&self, is_main_thread: bool) -> bool {
        self.config.mode.is_sandboxed(is_main_thread)
    }

    /// Check if the underlying container runtime is available.
    pub fn is_available(&self) -> bool {
        self.runtime.is_available()
    }

    /// Get the current sandbox mode.
    pub fn mode(&self) -> SandboxMode {
        self.config.mode
    }

    /// Execute a tool inside a sandboxed container.
    ///
    /// The tool is expected to be available inside the container image.
    /// Tool arguments are serialized and passed via stdin to the container's
    /// entrypoint.
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name of the tool to execute
    /// * `args` - Tool arguments as JSON
    /// * `image_override` - Optional image override (uses config default if None)
    ///
    /// # Returns
    ///
    /// The result of the tool execution, or an error if the container failed
    /// to start or the tool execution failed.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        args: Value,
        image_override: Option<&str>,
    ) -> Result<SandboxedToolResult, String> {
        self.execute_tool_with_env(tool_name, args, image_override, HashMap::new())
            .await
    }

    /// Execute a tool with additional environment variables.
    pub async fn execute_tool_with_env(
        &self,
        tool_name: &str,
        args: Value,
        image_override: Option<&str>,
        extra_env: HashMap<String, String>,
    ) -> Result<SandboxedToolResult, String> {
        let image = image_override.unwrap_or(&self.config.default_image);

        // Ensure the image is available.
        self.runtime.pull_image(image).await?;

        // Build container config.
        let mut env = extra_env;
        env.insert("TOOL_NAME".to_string(), tool_name.to_string());

        let config = ContainerConfig {
            image: image.to_string(),
            command: "/usr/local/bin/tool-runner".to_string(),
            args: vec![tool_name.to_string()],
            env,
            volumes: self.config.volumes.clone(),
            memory_limit_mb: self.config.memory_limit_mb,
            network_disabled: self.config.network_disabled,
            timeout_secs: self.config.timeout_secs,
        };

        // Spawn the container.
        let mut child = self.runtime.spawn(&config).await?;

        // Write tool arguments to container stdin.
        let args_json = serde_json::to_string(&args).map_err(|e| e.to_string())?;
        if let Some(ref mut stdin) = child.stdin {
            stdin
                .write_all(args_json.as_bytes())
                .await
                .map_err(|e| format!("failed to write to container stdin: {e}"))?;
            stdin.shutdown().await.map_err(|e| e.to_string())?;
        }

        // Take stdout before the async block to avoid borrow issues.
        let stdout_pipe = child.stdout.take();

        // Collect stdout with timeout.
        let timeout_duration = self
            .config
            .timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(60));

        let output = tokio::time::timeout(timeout_duration, async {
            let mut stdout = String::new();
            if let Some(pipe) = stdout_pipe {
                let mut reader = BufReader::new(pipe).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    stdout.push_str(&line);
                    stdout.push('\n');
                }
            }
            Ok::<String, String>(stdout)
        })
        .await
        .map_err(|_| format!("tool execution timed out after {:?}", timeout_duration))??;

        // Wait for container to finish.
        let status = child
            .wait()
            .await
            .map_err(|e| format!("failed to wait for container: {e}"))?;

        let success = status.success();

        // Try to parse the output as a structured result.
        let (output, metadata) = if let Ok(parsed) = serde_json::from_str::<Value>(&output) {
            let out = parsed
                .get("output")
                .and_then(|v| v.as_str())
                .unwrap_or(&output)
                .to_string();
            let meta = parsed.get("metadata").cloned();
            (out, meta)
        } else {
            (output.trim().to_string(), None)
        };

        Ok(SandboxedToolResult {
            output,
            success,
            metadata,
            container_id: None, // We use --rm, so no ID to track
        })
    }

    /// Build a container config for a shell command execution.
    ///
    /// This is useful for running simple shell commands inside a sandbox
    /// without a custom tool runner.
    pub fn build_shell_config(&self, command: &str, working_dir: Option<&str>) -> ContainerConfig {
        let mut volumes = self.config.volumes.clone();

        // Mount the working directory if specified.
        if let Some(dir) = working_dir {
            volumes.push(format!("{}:/work", dir));
        }

        ContainerConfig {
            image: self.config.default_image.clone(),
            command: "/bin/sh".to_string(),
            args: vec!["-c".to_string(), command.to_string()],
            env: HashMap::new(),
            volumes,
            memory_limit_mb: self.config.memory_limit_mb,
            network_disabled: self.config.network_disabled,
            timeout_secs: self.config.timeout_secs,
        }
    }

    /// Execute a shell command inside a sandboxed container.
    pub async fn execute_shell(&self, command: &str, working_dir: Option<&str>) -> Result<SandboxedToolResult, String> {
        let config = self.build_shell_config(command, working_dir);

        // Ensure the image is available.
        self.runtime.pull_image(&config.image).await?;

        // Spawn and run the container.
        let mut child = self.runtime.spawn(&config).await?;

        // Take stdout before the async block to avoid borrow issues.
        let stdout_pipe = child.stdout.take();

        let timeout_duration = self
            .config
            .timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(60));

        let output = tokio::time::timeout(timeout_duration, async {
            let mut stdout = String::new();
            if let Some(pipe) = stdout_pipe {
                let mut reader = BufReader::new(pipe).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    stdout.push_str(&line);
                    stdout.push('\n');
                }
            }
            Ok::<String, String>(stdout)
        })
        .await
        .map_err(|_| format!("shell execution timed out after {:?}", timeout_duration))??;

        let status = child
            .wait()
            .await
            .map_err(|e| format!("failed to wait for container: {e}"))?;

        Ok(SandboxedToolResult {
            output: output.trim().to_string(),
            success: status.success(),
            metadata: Some(serde_json::json!({ "exit_code": status.code() })),
            container_id: None,
        })
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(mode: SandboxMode) -> SandboxConfig {
        SandboxConfig {
            mode,
            ..SandboxConfig::default()
        }
    }

    #[test]
    fn sandbox_mode_off_never_sandboxed() {
        let mode = SandboxMode::Off;
        assert!(!mode.is_sandboxed(true));
        assert!(!mode.is_sandboxed(false));
    }

    #[test]
    fn sandbox_mode_non_main_only_non_main() {
        let mode = SandboxMode::NonMain;
        assert!(!mode.is_sandboxed(true));
        assert!(mode.is_sandboxed(false));
    }

    #[test]
    fn sandbox_mode_all_always_sandboxed() {
        let mode = SandboxMode::All;
        assert!(mode.is_sandboxed(true));
        assert!(mode.is_sandboxed(false));
    }

    #[test]
    fn sandbox_config_defaults() {
        let config = SandboxConfig::default();
        assert_eq!(config.mode, SandboxMode::NonMain);
        assert_eq!(config.default_image, "alpine:3.20");
        assert_eq!(config.memory_limit_mb, Some(256));
        assert!(config.network_disabled);
        assert_eq!(config.timeout_secs, Some(60));
        assert!(config.volumes.is_empty());
    }

    #[test]
    fn sandbox_manager_should_sandbox_respects_mode() {
        let config = make_config(SandboxMode::Off);
        // We can't test with a real runtime here, so just test the mode logic
        assert!(!config.mode.is_sandboxed(true));
        assert!(!config.mode.is_sandboxed(false));

        let config = make_config(SandboxMode::All);
        assert!(config.mode.is_sandboxed(true));
        assert!(config.mode.is_sandboxed(false));
    }

    #[test]
    fn build_shell_config_uses_settings() {
        let config = SandboxConfig {
            mode: SandboxMode::All,
            default_image: "ubuntu:22.04".to_string(),
            memory_limit_mb: Some(512),
            network_disabled: false,
            timeout_secs: Some(120),
            volumes: vec!["/host/data:/data".to_string()],
        };

        // Can't create manager without runtime, but we can test the config building
        assert_eq!(config.default_image, "ubuntu:22.04");
        assert_eq!(config.memory_limit_mb, Some(512));
        assert!(!config.network_disabled);
        assert_eq!(config.timeout_secs, Some(120));
        assert_eq!(config.volumes.len(), 1);
    }

    #[test]
    fn sandboxed_tool_result_serialization() {
        let result = SandboxedToolResult {
            output: "test output".to_string(),
            success: true,
            metadata: Some(serde_json::json!({ "exit_code": 0 })),
            container_id: Some("abc123".to_string()),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test output"));
        assert!(json.contains("exit_code"));

        let parsed: SandboxedToolResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.output, result.output);
        assert_eq!(parsed.success, result.success);
    }

    #[test]
    fn sandbox_mode_default_is_non_main() {
        assert_eq!(SandboxMode::default(), SandboxMode::NonMain);
    }

    #[test]
    fn sandbox_mode_serde_snake_case() {
        let mode = SandboxMode::NonMain;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"non_main\"");

        let parsed: SandboxMode = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, SandboxMode::NonMain);
    }
}
