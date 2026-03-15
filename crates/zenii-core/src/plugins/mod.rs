use std::path::Path;

pub mod adapter;
pub mod installer;
pub mod manifest;
pub mod process;
pub mod registry;

pub use manifest::PluginManifest;
pub use registry::{InstalledPlugin, PluginRegistry};

/// Fetch the real `parameters_schema` from a plugin by spawning it and calling `info()`.
///
/// Falls back to `{}` on any failure (missing interpreter, binary not built, etc.).
pub async fn fetch_plugin_schema(
    binary_path: &Path,
    tool_name: &str,
    timeout_secs: u64,
    max_restart: u32,
) -> serde_json::Value {
    let mut proc = process::PluginProcess::new(
        tool_name,
        binary_path.to_path_buf(),
        timeout_secs,
        max_restart,
    );

    let schema = match proc.spawn().await {
        Ok(()) => match proc.info().await {
            Ok(info) => info.parameters_schema,
            Err(e) => {
                tracing::debug!("Could not fetch schema for plugin '{tool_name}': {e}");
                serde_json::json!({})
            }
        },
        Err(e) => {
            tracing::debug!("Could not spawn plugin '{tool_name}' for schema fetch: {e}");
            serde_json::json!({})
        }
    };

    let _ = proc.shutdown().await;
    schema
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use std::path::PathBuf;

    /// Returns the real zenii-plugins path, or None if not available.
    pub fn real_plugins_path() -> Option<PathBuf> {
        let path = std::env::var("ZENII_PLUGINS_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from("/home/rakesh/RD/NSRTech/Tauri/zenii-plugins/plugins")
            });
        if path.exists() { Some(path) } else { None }
    }

    /// Check if an interpreter is available on PATH.
    pub fn has_interpreter(name: &str) -> bool {
        std::process::Command::new("which")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}
