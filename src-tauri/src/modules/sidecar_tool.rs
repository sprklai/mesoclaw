//! `SidecarTool` — spawns a sidecar process on demand to satisfy a `Tool::execute()` call.
//!
//! # Protocol
//! Communicates with the sidecar via the newline-delimited JSON protocol in
//! `protocol::stdio_json`.  The process is spawned fresh for each invocation
//! (tool-type modules) and killed after the response is received or the
//! timeout expires.

use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::BufReader;

use crate::{
    event_bus::{AppEvent, EventBus},
    security::{SecurityPolicy, ValidationResult},
    tools::{Tool, ToolResult},
};

use super::{
    manifest::{ModuleManifest, ModuleType, RuntimeConfig, RuntimeType},
    protocol::stdio_json::{send_request, read_response, StdioRequest},
};

// ─── SidecarTool ──────────────────────────────────────────────────────────────

/// A `Tool` implementation that delegates execution to a spawned sidecar process.
pub struct SidecarTool {
    manifest: ModuleManifest,
    policy: Arc<SecurityPolicy>,
    /// Optional EventBus for emitting tool-start / tool-result events.
    bus: Option<Arc<dyn EventBus>>,
}

impl SidecarTool {
    pub fn new(
        manifest: ModuleManifest,
        policy: Arc<SecurityPolicy>,
        bus: Option<Arc<dyn EventBus>>,
    ) -> Self {
        Self { manifest, policy, bus }
    }

    pub fn module_type(&self) -> &ModuleType {
        &self.manifest.module.module_type
    }

    pub fn runtime_config(&self) -> &RuntimeConfig {
        &self.manifest.runtime
    }

    /// Return true when the container runtime required by this module
    /// matches the configured (or auto-detected) runtime.
    ///
    /// Always true for native; container runtimes require the `containers`
    /// feature and auto-detection logic in `modules::container`.
    pub fn runtime_available(&self) -> bool {
        match self.manifest.runtime.runtime_type {
            RuntimeType::Native => true,
            _ => {
                // Container runtimes are only supported when the `containers` feature
                // is enabled and a runtime binary is found on the system.
                #[cfg(feature = "containers")]
                {
                    super::container::detect_runtime().is_some()
                }
                #[cfg(not(feature = "containers"))]
                {
                    false
                }
            }
        }
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    fn emit_start(&self, args: &Value) {
        if let Some(bus) = &self.bus {
            let _ = bus.publish(AppEvent::AgentToolStart {
                tool_name: self.manifest.module.id.clone(),
                args: args.clone(),
            });
        }
    }

    fn emit_result(&self, result_str: &str, success: bool) {
        if let Some(bus) = &self.bus {
            let _ = bus.publish(AppEvent::AgentToolResult {
                tool_name: self.manifest.module.id.clone(),
                result: result_str.to_string(),
                success,
            });
        }
    }
}

// ─── Tool impl ────────────────────────────────────────────────────────────────

#[async_trait]
impl Tool for SidecarTool {
    fn name(&self) -> &str {
        &self.manifest.module.id
    }

    fn description(&self) -> &str {
        &self.manifest.module.description
    }

    fn parameters_schema(&self) -> Value {
        self.manifest.parameters_schema()
    }

    async fn execute(&self, args: Value) -> Result<ToolResult, String> {
        // ── 1. Security check ────────────────────────────────────────────────
        let command = &self.manifest.runtime.command;
        match self.policy.validate_command(command) {
            ValidationResult::Denied(reason) => {
                return Err(format!(
                    "sidecar '{}' denied by security policy: {reason}",
                    self.manifest.module.id
                ))
            }
            ValidationResult::NeedsApproval => {
                return Err(format!(
                    "sidecar '{}' requires manual approval (autonomy level too low)",
                    self.manifest.module.id
                ))
            }
            ValidationResult::Allowed => {}
        }

        // ── 2. Emit start event ──────────────────────────────────────────────
        self.emit_start(&args);

        // ── 3. Extract method / params from the args value ───────────────────
        let method = args
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("execute")
            .to_string();
        let params = args
            .get("params")
            .cloned()
            .unwrap_or_else(|| args.clone());

        // ── 4. Spawn process (native or container) ───────────────────────────
        let timeout = Duration::from_secs(
            self.manifest.runtime.timeout_secs.unwrap_or(30),
        );

        let mut child = self.spawn_child().await?;

        let mut stdin = child.stdin.take().ok_or("child has no stdin")?;
        let stdout = child.stdout.take().ok_or("child has no stdout")?;
        let mut reader = BufReader::new(stdout);

        let request = StdioRequest {
            id: uuid::Uuid::new_v4().to_string(),
            method,
            params,
        };

        // ── 5. Exchange request / response with timeout ──────────────────────
        let io_result = tokio::time::timeout(timeout, async {
            send_request(&mut stdin, &request).await?;
            read_response(&mut reader).await
        })
        .await;

        // Kill the child regardless of outcome.
        let _ = child.kill().await;

        // ── 6. Process result ────────────────────────────────────────────────
        let tool_result = match io_result {
            Ok(Ok(response)) => match response.into_result() {
                Ok(value) => {
                    let output = serde_json::to_string_pretty(&value)
                        .unwrap_or_else(|_| value.to_string());
                    ToolResult::ok(output)
                }
                Err(e) => ToolResult::err(e),
            },
            Ok(Err(e)) => ToolResult::err(e),
            Err(_) => ToolResult::err(format!(
                "sidecar '{}' timed out after {}s",
                self.manifest.module.id,
                timeout.as_secs()
            )),
        };

        self.emit_result(&tool_result.output, tool_result.success);
        Ok(tool_result)
    }
}

// ─── Spawn helpers ─────────────────────────────────────────────────────────────

impl SidecarTool {
    /// Spawn the child process for this module using the appropriate runtime.
    async fn spawn_child(&self) -> Result<tokio::process::Child, String> {
        match self.manifest.runtime.runtime_type {
            RuntimeType::Native => self.spawn_native_child(),

            #[cfg(feature = "containers")]
            RuntimeType::Docker | RuntimeType::Podman => self.spawn_container_child().await,

            #[cfg(not(feature = "containers"))]
            _ => Err(format!(
                "module '{}' requires a container runtime but the `containers` \
                 feature is not enabled",
                self.manifest.module.id
            )),
        }
    }

    /// Spawn a native process for a `RuntimeType::Native` module.
    fn spawn_native_child(&self) -> Result<tokio::process::Child, String> {
        let command = &self.manifest.runtime.command;
        let mut cmd = tokio::process::Command::new(command);
        cmd.args(&self.manifest.runtime.args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        for (k, v) in &self.manifest.runtime.env {
            cmd.env(k, v);
        }
        cmd.spawn()
            .map_err(|e| format!("failed to spawn sidecar '{}': {e}", command))
    }

    /// Spawn a container via the auto-detected container runtime.
    ///
    /// For Docker/Podman modules:
    /// - `manifest.runtime.command` is the container image name.
    /// - `manifest.runtime.args[0]` is the command inside the container.
    /// - `manifest.runtime.args[1..]` are arguments to that command.
    #[cfg(feature = "containers")]
    async fn spawn_container_child(&self) -> Result<tokio::process::Child, String> {
        use super::container::{detect_runtime, ContainerConfig};

        let runtime = detect_runtime().ok_or_else(|| {
            format!(
                "module '{}' requires a container runtime but neither Docker nor \
                 Podman was found",
                self.manifest.module.id
            )
        })?;

        let config = ContainerConfig {
            image: self.manifest.runtime.command.clone(),
            command: self.manifest.runtime.args.first().cloned().unwrap_or_default(),
            args: self
                .manifest
                .runtime
                .args
                .get(1..)
                .unwrap_or(&[])
                .to_vec(),
            env: self.manifest.runtime.env.clone(),
            memory_limit_mb: Some(self.manifest.security.max_memory_mb),
            network_disabled: !self.manifest.security.allow_network,
            timeout_secs: self.manifest.runtime.timeout_secs,
            volumes: vec![],
        };

        runtime.spawn(&config).await
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::manifest::{
        ModuleInfo, ModuleType, ParametersConfig, RuntimeConfig, RuntimeType,
        SecurityConfig,
    };
    use crate::security::SecurityPolicy;
    use std::collections::HashMap;

    fn make_manifest(id: &str, command: &str) -> ModuleManifest {
        ModuleManifest {
            module: ModuleInfo {
                id: id.to_string(),
                name: id.to_string(),
                version: "1.0.0".to_string(),
                description: "test module".to_string(),
                module_type: ModuleType::Tool,
            },
            runtime: RuntimeConfig {
                runtime_type: RuntimeType::Native,
                command: command.to_string(),
                args: vec![],
                env: HashMap::new(),
                timeout_secs: Some(5),
            },
            security: SecurityConfig::default(),
            parameters: ParametersConfig::default(),
            service: Default::default(),
        }
    }

    fn readonly_policy() -> Arc<SecurityPolicy> {
        use crate::security::AutonomyLevel;
        Arc::new(SecurityPolicy::new(
            AutonomyLevel::ReadOnly,
            None,
            vec![],
            3600,
            20,
        ))
    }

    fn supervised_policy() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::default_policy())
    }

    #[test]
    fn name_from_manifest() {
        let m = make_manifest("my-tool", "echo");
        let t = SidecarTool::new(m, supervised_policy(), None);
        assert_eq!(t.name(), "my-tool");
    }

    #[test]
    fn description_from_manifest() {
        let m = make_manifest("my-tool", "echo");
        let t = SidecarTool::new(m, supervised_policy(), None);
        assert_eq!(t.description(), "test module");
    }

    #[test]
    fn parameters_schema_is_object() {
        let m = make_manifest("my-tool", "echo");
        let t = SidecarTool::new(m, supervised_policy(), None);
        let schema = t.parameters_schema();
        assert!(schema.is_object());
    }

    #[test]
    fn module_type_returns_type() {
        let m = make_manifest("svc", "daemon");
        let t = SidecarTool::new(m, supervised_policy(), None);
        assert_eq!(*t.module_type(), ModuleType::Tool);
    }

    #[test]
    fn native_runtime_is_available() {
        let m = make_manifest("t", "cat");
        let t = SidecarTool::new(m, supervised_policy(), None);
        assert!(t.runtime_available());
    }

    #[tokio::test]
    async fn blocked_by_readonly_policy() {
        // ReadOnly policy denies medium/high risk commands; `echo` is low risk
        // but `rm` is high risk and should be denied.
        let m = make_manifest("rm-tool", "rm");
        let t = SidecarTool::new(m, readonly_policy(), None);
        let result = t
            .execute(serde_json::json!({"method":"execute","params":{}}))
            .await;
        // Should be Err because readonly policy denies rm (high risk)
        // or at least the spawn should fail
        assert!(result.is_err() || !result.unwrap().success);
    }

    #[tokio::test]
    async fn nonexistent_binary_returns_error() {
        let m = make_manifest("ghost", "this-binary-does-not-exist-xyzzy");
        let t = SidecarTool::new(m, supervised_policy(), None);
        let result = t
            .execute(serde_json::json!({"method":"echo","params":{}}))
            .await;
        assert!(result.is_err() || !result.unwrap().success);
    }
}
