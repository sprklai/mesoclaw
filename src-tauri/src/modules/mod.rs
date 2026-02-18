//! Sidecar Module System — dynamic extension modules for MesoClaw.
//!
//! Modules extend the agent's capabilities by providing additional tools.
//! Each module lives in `~/.mesoclaw/modules/<name>/` and is described by a
//! `manifest.toml` file.  On startup, `ModuleRegistry::discover()` scans the
//! directory and registers discovered modules in the shared `ToolRegistry`.
//!
//! # Module types
//!
//! | Type      | Lifecycle           | Protocol              |
//! |-----------|---------------------|-----------------------|
//! | `tool`    | spawned on demand   | newline-delimited JSON |
//! | `service` | started at boot     | newline-delimited JSON |
//! | `mcp`     | started at boot     | JSON-RPC (MCP spec)   |
//!
//! # Bundled sidecar binaries (Tauri shell plugin)
//! ## TODO: configure bundled sidecar binaries in tauri.conf.json
//! ## `bundle.externalBin` and add tauri-plugin-shell once built-in helpers exist.
//! ## See: https://v2.tauri.app/develop/sidecar/
//! Dynamic user-installed modules under `~/.mesoclaw/modules/` continue to use
//! `tokio::process::Command` directly and are **not** managed by the Tauri
//! shell plugin.

pub mod manifest;
pub mod protocol;
pub mod sidecar_service;
pub mod sidecar_tool;

#[cfg(feature = "containers")]
pub mod container;

#[cfg(feature = "mcp-client")]
pub mod mcp_client;

pub use manifest::{
    load_manifest, parse_manifest, ModuleManifest, ModuleType, ParametersConfig,
    RuntimeConfig, RuntimeType, SecurityConfig,
};
pub use sidecar_tool::SidecarTool;

#[cfg(feature = "mcp-client")]
pub use mcp_client::{McpClient, McpTool, McpToolProxy};

use std::{collections::HashMap, path::Path, sync::Arc};

use async_trait::async_trait;

use crate::{
    event_bus::EventBus,
    security::SecurityPolicy,
    tools::{Tool, ToolRegistry},
};

// ─── SidecarModule trait ──────────────────────────────────────────────────────

/// Extension of `Tool` with lifecycle-management methods for long-running
/// modules (service / mcp types).
#[async_trait]
pub trait SidecarModule: Tool {
    /// Classify the module's operational mode.
    fn module_type(&self) -> &ModuleType;

    /// Runtime configuration from the manifest.
    fn runtime_config(&self) -> &RuntimeConfig;

    /// Perform a lightweight health probe.  Returns `true` if healthy.
    async fn health_check(&self) -> Result<bool, String>;

    /// Start a service-type module's background process.
    /// No-op for `tool` modules (they are spawned per-call).
    async fn start(&self) -> Result<(), String>;

    /// Stop a running service-type module.
    /// No-op for `tool` modules.
    async fn stop(&self) -> Result<(), String>;
}

// ─── Blanket SidecarModule impl for SidecarTool ───────────────────────────────

#[async_trait]
impl SidecarModule for SidecarTool {
    fn module_type(&self) -> &ModuleType {
        SidecarTool::module_type(self)
    }

    fn runtime_config(&self) -> &RuntimeConfig {
        SidecarTool::runtime_config(self)
    }

    async fn health_check(&self) -> Result<bool, String> {
        // For tool-type modules: the module is considered healthy if the binary
        // exists on PATH.  Service modules would do an actual health probe.
        Ok(which::which(&self.runtime_config().command).is_ok())
    }

    async fn start(&self) -> Result<(), String> {
        // Tool modules are spawned on demand — no persistent start needed.
        Ok(())
    }

    async fn stop(&self) -> Result<(), String> {
        // Tool modules have no persistent process to stop.
        Ok(())
    }
}

// ─── ModuleRegistry ───────────────────────────────────────────────────────────

/// Manages discovered sidecar modules and their registration in the
/// application's `ToolRegistry`.
pub struct ModuleRegistry {
    modules: HashMap<String, Arc<SidecarTool>>,
}

impl ModuleRegistry {
    /// Scan `modules_dir` for `<name>/manifest.toml` files, parse them, and
    /// register each valid module in `tool_registry`.
    ///
    /// Invalid or unparseable manifests are logged and skipped.
    pub fn discover(
        modules_dir: &Path,
        tool_registry: &mut ToolRegistry,
        policy: Arc<SecurityPolicy>,
        bus: Option<Arc<dyn EventBus>>,
    ) -> Self {
        let mut modules = HashMap::new();

        if !modules_dir.is_dir() {
            log::debug!(
                "ModuleRegistry: modules directory {:?} does not exist, skipping discovery",
                modules_dir
            );
            return Self { modules };
        }

        let entries = match std::fs::read_dir(modules_dir) {
            Ok(e) => e,
            Err(e) => {
                log::warn!(
                    "ModuleRegistry: failed to scan {:?}: {e}",
                    modules_dir
                );
                return Self { modules };
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Only consider sub-directories.
            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("manifest.toml");
            if !manifest_path.exists() {
                continue;
            }

            match load_manifest(&manifest_path) {
                Ok(manifest) => {
                    let id = manifest.module.id.clone();
                    let tool = Arc::new(SidecarTool::new(
                        manifest,
                        policy.clone(),
                        bus.clone(),
                    ));

                    // Register as a generic Tool in the shared registry.
                    tool_registry
                        .register(Arc::clone(&tool) as Arc<dyn Tool>);

                    log::info!("ModuleRegistry: registered module '{id}'");
                    modules.insert(id, tool);
                }
                Err(e) => {
                    log::warn!(
                        "ModuleRegistry: skipping {:?}: {e}",
                        manifest_path
                    );
                }
            }
        }

        Self { modules }
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn get(&self, id: &str) -> Option<&Arc<SidecarTool>> {
        self.modules.get(id)
    }

    /// Ids of all registered modules.
    pub fn ids(&self) -> Vec<&str> {
        self.modules.keys().map(String::as_str).collect()
    }
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn policy() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy::default_policy())
    }

    const VALID_MANIFEST: &str = r#"
[module]
id = "test-tool"
name = "Test Tool"
version = "1.0.0"
description = "A test tool"
type = "tool"

[runtime]
command = "echo"
"#;

    #[test]
    fn empty_modules_dir_returns_empty_registry() {
        let dir = TempDir::new().unwrap();
        let mut tr = ToolRegistry::new();
        let reg = ModuleRegistry::discover(dir.path(), &mut tr, policy(), None);
        assert!(reg.is_empty());
        assert_eq!(tr.len(), 0);
    }

    #[test]
    fn nonexistent_dir_returns_empty_registry() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("does-not-exist");
        let mut tr = ToolRegistry::new();
        let reg = ModuleRegistry::discover(&missing, &mut tr, policy(), None);
        assert!(reg.is_empty());
    }

    #[test]
    fn discovers_valid_module() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("test-tool");
        std::fs::create_dir_all(&mod_dir).unwrap();
        std::fs::write(mod_dir.join("manifest.toml"), VALID_MANIFEST).unwrap();

        let mut tr = ToolRegistry::new();
        let reg = ModuleRegistry::discover(dir.path(), &mut tr, policy(), None);

        assert_eq!(reg.len(), 1);
        assert!(reg.get("test-tool").is_some());
        assert!(tr.get("test-tool").is_some());
    }

    #[test]
    fn skips_files_at_top_level() {
        let dir = TempDir::new().unwrap();
        // A file, not a directory — should be skipped.
        std::fs::write(dir.path().join("manifest.toml"), VALID_MANIFEST).unwrap();

        let mut tr = ToolRegistry::new();
        let reg = ModuleRegistry::discover(dir.path(), &mut tr, policy(), None);
        assert!(reg.is_empty());
    }

    #[test]
    fn skips_subdir_without_manifest() {
        let dir = TempDir::new().unwrap();
        // Sub-directory with no manifest.toml.
        std::fs::create_dir_all(dir.path().join("empty-module")).unwrap();

        let mut tr = ToolRegistry::new();
        let reg = ModuleRegistry::discover(dir.path(), &mut tr, policy(), None);
        assert!(reg.is_empty());
    }

    #[test]
    fn skips_invalid_manifest() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("bad-module");
        std::fs::create_dir_all(&mod_dir).unwrap();
        // Invalid TOML — will be skipped.
        std::fs::write(mod_dir.join("manifest.toml"), "not valid toml !!##").unwrap();

        let mut tr = ToolRegistry::new();
        let reg = ModuleRegistry::discover(dir.path(), &mut tr, policy(), None);
        assert!(reg.is_empty());
    }

    #[test]
    fn discovers_multiple_modules() {
        let dir = TempDir::new().unwrap();

        let make_mod = |name: &str, dir_path: &Path| {
            let mod_dir = dir_path.join(name);
            std::fs::create_dir_all(&mod_dir).unwrap();
            let manifest = format!(
                r#"
[module]
id = "{name}"
name = "{name}"
version = "1.0.0"
description = "Module {name}"
type = "tool"

[runtime]
command = "echo"
"#
            );
            std::fs::write(mod_dir.join("manifest.toml"), manifest).unwrap();
        };

        make_mod("alpha", dir.path());
        make_mod("beta", dir.path());
        // One invalid module — should be skipped.
        let bad = dir.path().join("gamma");
        std::fs::create_dir_all(&bad).unwrap();
        std::fs::write(bad.join("manifest.toml"), "broken").unwrap();

        let mut tr = ToolRegistry::new();
        let reg = ModuleRegistry::discover(dir.path(), &mut tr, policy(), None);

        assert_eq!(reg.len(), 2);
        assert!(reg.get("alpha").is_some());
        assert!(reg.get("beta").is_some());
        assert_eq!(tr.len(), 2);
    }

    #[test]
    fn ids_returns_all_module_ids() {
        let dir = TempDir::new().unwrap();
        let mod_dir = dir.path().join("my-module");
        std::fs::create_dir_all(&mod_dir).unwrap();
        std::fs::write(mod_dir.join("manifest.toml"), VALID_MANIFEST).unwrap();

        let mut tr = ToolRegistry::new();
        let reg = ModuleRegistry::discover(dir.path(), &mut tr, policy(), None);
        let ids = reg.ids();
        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&"test-tool"));
    }
}
