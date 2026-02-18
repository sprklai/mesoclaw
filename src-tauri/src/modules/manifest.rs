//! TOML manifest parser for sidecar modules.
//!
//! Each module lives in `~/.mesoclaw/modules/<name>/` and contains a
//! `manifest.toml` file.  Example:
//!
//! ```toml
//! [module]
//! id = "my-tool"
//! name = "My Tool"
//! version = "0.1.0"
//! description = "Does something useful"
//! type = "tool"  # tool | service | mcp
//!
//! [runtime]
//! type = "native"  # native | docker | podman
//! command = "my-tool-binary"
//! args = ["--stdio"]
//! timeout_secs = 30
//!
//! [security]
//! allow_network = false
//! allow_filesystem = false
//! max_memory_mb = 256
//!
//! [parameters]  # optional
//! schema = '{"type":"object","properties":{"input":{"type":"string"}}}'
//! ```

use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ─── Module types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ModuleType {
    /// Spawned on demand, returns a result, then exits.
    Tool,
    /// Long-running background process; managed by ModuleRegistry.
    Service,
    /// MCP (Model Context Protocol) server — discovers tools on `initialize`.
    Mcp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    /// Native OS process spawned via `tokio::process::Command`.
    #[default]
    Native,
    /// Docker container (requires `containers` feature).
    Docker,
    /// Podman container (requires `containers` feature).
    Podman,
}

// ─── Manifest sections ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(rename = "type")]
    pub module_type: ModuleType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    #[serde(rename = "type", default)]
    pub runtime_type: RuntimeType,
    /// Binary name or absolute path.
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Extra environment variables passed to the process.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// Per-invocation wall-clock timeout.  Defaults to 30 seconds.
    pub timeout_secs: Option<u64>,
}

fn default_max_memory_mb() -> u64 {
    512
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Whether the process is allowed to make outbound network connections.
    #[serde(default)]
    pub allow_network: bool,
    /// Whether the process is allowed to read/write arbitrary filesystem paths.
    #[serde(default)]
    pub allow_filesystem: bool,
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u64,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            allow_network: false,
            allow_filesystem: false,
            max_memory_mb: default_max_memory_mb(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParametersConfig {
    /// JSON Schema string describing accepted parameters.
    pub schema: Option<String>,
}

// ─── Top-level manifest ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleManifest {
    pub module: ModuleInfo,
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub parameters: ParametersConfig,
}

impl ModuleManifest {
    /// JSON Schema for this module's tool parameters.
    ///
    /// Returns the schema from `[parameters]` if provided, otherwise a generic
    /// passthrough schema.
    pub fn parameters_schema(&self) -> Value {
        if let Some(schema_str) = &self.parameters.schema {
            serde_json::from_str(schema_str).unwrap_or_else(|_| {
                serde_json::json!({"type": "object"})
            })
        } else {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string",
                        "description": "Method to invoke on the sidecar"
                    },
                    "params": {
                        "type": "object",
                        "description": "Parameters passed to the method"
                    }
                }
            })
        }
    }
}

// ─── Parsing ───────────────────────────────────────────────────────────────────

/// Parse a `ModuleManifest` from a TOML string.
pub fn parse_manifest(toml_str: &str) -> Result<ModuleManifest, String> {
    toml::from_str::<ModuleManifest>(toml_str)
        .map_err(|e| format!("manifest parse error: {e}"))
}

/// Load and parse a `manifest.toml` file from disk.
pub fn load_manifest(path: &Path) -> Result<ModuleManifest, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {:?}: {e}", path))?;
    parse_manifest(&content)
}

// ─── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const MINIMAL_TOML: &str = r#"
[module]
id = "hello"
name = "Hello"
version = "1.0.0"
description = "Says hello"
type = "tool"

[runtime]
command = "hello-bin"
"#;

    const FULL_TOML: &str = r#"
[module]
id = "full-module"
name = "Full Module"
version = "2.3.4"
description = "A fully specified module"
type = "service"

[runtime]
type = "native"
command = "/usr/local/bin/my-daemon"
args = ["--port", "9000"]
timeout_secs = 60

[security]
allow_network = true
allow_filesystem = false
max_memory_mb = 1024

[parameters]
schema = '{"type":"object","properties":{"query":{"type":"string"}}}'
"#;

    #[test]
    fn parse_minimal_manifest() {
        let m = parse_manifest(MINIMAL_TOML).unwrap();
        assert_eq!(m.module.id, "hello");
        assert_eq!(m.module.module_type, ModuleType::Tool);
        assert_eq!(m.runtime.command, "hello-bin");
        assert!(m.runtime.args.is_empty());
        assert_eq!(m.runtime.runtime_type, RuntimeType::Native);
        assert!(!m.security.allow_network);
        assert_eq!(m.security.max_memory_mb, 512);
    }

    #[test]
    fn parse_full_manifest() {
        let m = parse_manifest(FULL_TOML).unwrap();
        assert_eq!(m.module.module_type, ModuleType::Service);
        assert_eq!(m.runtime.runtime_type, RuntimeType::Native);
        assert_eq!(m.runtime.args, vec!["--port", "9000"]);
        assert_eq!(m.runtime.timeout_secs, Some(60));
        assert!(m.security.allow_network);
        assert_eq!(m.security.max_memory_mb, 1024);
        assert!(m.parameters.schema.is_some());
    }

    #[test]
    fn parse_mcp_type() {
        let toml = r#"
[module]
id = "my-mcp"
name = "MCP Server"
version = "1.0.0"
description = "MCP server"
type = "mcp"

[runtime]
command = "mcp-server"
"#;
        let m = parse_manifest(toml).unwrap();
        assert_eq!(m.module.module_type, ModuleType::Mcp);
    }

    #[test]
    fn missing_required_field_fails() {
        // Missing `id` in [module]
        let toml = r#"
[module]
name = "No ID"
version = "1.0.0"
description = "Missing id"
type = "tool"

[runtime]
command = "binary"
"#;
        assert!(parse_manifest(toml).is_err());
    }

    #[test]
    fn missing_command_fails() {
        let toml = r#"
[module]
id = "no-command"
name = "No Command"
version = "1.0.0"
description = "Missing command"
type = "tool"

[runtime]
type = "native"
"#;
        assert!(parse_manifest(toml).is_err());
    }

    #[test]
    fn parameters_schema_returns_json() {
        let m = parse_manifest(FULL_TOML).unwrap();
        let schema = m.parameters_schema();
        assert!(schema.is_object());
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["query"].is_object());
    }

    #[test]
    fn parameters_schema_default_when_absent() {
        let m = parse_manifest(MINIMAL_TOML).unwrap();
        let schema = m.parameters_schema();
        assert!(schema.is_object());
        assert!(schema["properties"]["method"].is_object());
    }

    #[test]
    fn load_manifest_from_disk() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("manifest.toml");
        std::fs::write(&path, MINIMAL_TOML).unwrap();
        let m = load_manifest(&path).unwrap();
        assert_eq!(m.module.id, "hello");
    }

    #[test]
    fn load_manifest_missing_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does-not-exist.toml");
        assert!(load_manifest(&path).is_err());
    }
}
