use std::path::Path;
use std::sync::Arc;

use tokio::sync::Mutex;
use tracing::info;

use super::adapter::PluginToolAdapter;
use super::manifest::PluginManifest;
use super::process::PluginProcess;
use super::registry::{InstalledPlugin, PluginRegistry, PluginSource};
use crate::skills::SkillRegistry;
use crate::tools::ToolRegistry;
use crate::{Result, ZeniiError};

/// Handles plugin install/update/remove operations.
pub struct PluginInstaller {
    registry: Arc<PluginRegistry>,
    tool_registry: Arc<ToolRegistry>,
    skill_registry: Arc<SkillRegistry>,
    execute_timeout_secs: u64,
    max_restart_attempts: u32,
}

impl PluginInstaller {
    pub fn new(
        registry: Arc<PluginRegistry>,
        tool_registry: Arc<ToolRegistry>,
        skill_registry: Arc<SkillRegistry>,
        execute_timeout_secs: u64,
        max_restart_attempts: u32,
    ) -> Self {
        Self {
            registry,
            tool_registry,
            skill_registry,
            execute_timeout_secs,
            max_restart_attempts,
        }
    }

    /// Install a plugin from a git URL.
    ///
    /// Supports monorepo subdirectories via URL fragment:
    /// `https://github.com/org/plugins#plugins/weather` installs only the
    /// `plugins/weather` subdirectory. Without a fragment, the entire repo
    /// is treated as one plugin.
    pub async fn install_from_git(&self, url: &str) -> Result<InstalledPlugin> {
        let plugins_dir = self.registry.plugins_dir();

        // Parse optional subdirectory from URL fragment
        let (git_url, subdir) = match url.rsplit_once('#') {
            Some((base, path)) if !path.is_empty() => (base, Some(path)),
            _ => (url, None),
        };

        // Clone to temp dir first
        let temp_path = std::env::temp_dir().join(format!(
            "zenii-plugin-install-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&temp_path)
            .map_err(|e| ZeniiError::Plugin(format!("temp dir failed: {e}")))?;

        let output = tokio::process::Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                git_url,
                temp_path.to_str().unwrap_or("."),
            ])
            .output()
            .await
            .map_err(|e| ZeniiError::Plugin(format!("git clone failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let _ = std::fs::remove_dir_all(&temp_path);
            return Err(ZeniiError::Plugin(format!("git clone failed: {stderr}")));
        }

        // Resolve plugin root (subdirectory or repo root)
        let plugin_root = match &subdir {
            Some(path) => {
                let sub = temp_path.join(path);
                if !sub.exists() {
                    let _ = std::fs::remove_dir_all(&temp_path);
                    return Err(ZeniiError::Plugin(format!(
                        "subdirectory '{path}' not found in repository"
                    )));
                }
                sub
            }
            None => temp_path.clone(),
        };

        // Parse manifest
        let manifest = PluginManifest::from_file(&plugin_root.join("zenii-plugin.toml"))?;
        let name = manifest.plugin.name.clone();

        // Check not already installed
        if self.registry.get(&name).is_some() {
            let _ = std::fs::remove_dir_all(&temp_path);
            return Err(ZeniiError::Plugin(format!(
                "plugin '{name}' is already installed"
            )));
        }

        // Get commit hash
        let commit_output = tokio::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&temp_path)
            .output()
            .await
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        // Move to plugins dir
        let dest = plugins_dir.join(&name);
        if dest.exists() {
            std::fs::remove_dir_all(&dest)
                .map_err(|e| ZeniiError::Plugin(format!("remove old dir failed: {e}")))?;
        }
        Self::copy_dir_recursive(&plugin_root, &dest)?;
        let _ = std::fs::remove_dir_all(&temp_path);

        let installed = InstalledPlugin {
            manifest,
            install_path: dest,
            enabled: true,
            installed_at: chrono::Utc::now().to_rfc3339(),
            source: PluginSource::Git {
                url: url.to_string(),
                commit: commit_output,
            },
        };

        // Register plugin, tools, and skills
        self.register_plugin_assets(&installed).await?;
        self.registry.register(installed.clone())?;

        info!("Installed plugin '{}' from git", name);
        Ok(installed)
    }

    /// Install a plugin from a local directory.
    pub async fn install_from_local(&self, path: &Path) -> Result<InstalledPlugin> {
        let plugins_dir = self.registry.plugins_dir();

        // Parse manifest from source
        let manifest = PluginManifest::from_file(&path.join("zenii-plugin.toml"))?;
        let name = manifest.plugin.name.clone();

        // Check not already installed
        if self.registry.get(&name).is_some() {
            return Err(ZeniiError::Plugin(format!(
                "plugin '{name}' is already installed"
            )));
        }

        // Copy to plugins dir
        let dest = plugins_dir.join(&name);
        if dest.exists() {
            std::fs::remove_dir_all(&dest)
                .map_err(|e| ZeniiError::Plugin(format!("remove old dir failed: {e}")))?;
        }
        Self::copy_dir_recursive(path, &dest)?;

        let installed = InstalledPlugin {
            manifest,
            install_path: dest,
            enabled: true,
            installed_at: chrono::Utc::now().to_rfc3339(),
            source: PluginSource::Local {
                path: path.to_path_buf(),
            },
        };

        self.register_plugin_assets(&installed).await?;
        self.registry.register(installed.clone())?;

        info!("Installed plugin '{}' from local path", name);
        Ok(installed)
    }

    /// Install all plugins found in a local directory.
    ///
    /// Scans immediate subdirectories for `zenii-plugin.toml` manifests and
    /// installs each one. If the directory itself contains a manifest, it is
    /// installed as a single plugin instead.
    ///
    /// Returns the list of successfully installed plugins. Errors for
    /// individual plugins are logged but do not abort the batch.
    pub async fn install_all_from_local(&self, path: &Path) -> Result<Vec<InstalledPlugin>> {
        // If the directory itself is a plugin, install it directly
        if path.join("zenii-plugin.toml").exists() {
            let installed = self.install_from_local(path).await?;
            return Ok(vec![installed]);
        }

        // Scan subdirectories for plugins
        let entries = std::fs::read_dir(path)
            .map_err(|e| ZeniiError::Plugin(format!("read dir failed: {e}")))?;

        let mut installed = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| ZeniiError::Plugin(format!("dir entry error: {e}")))?;
            let sub = entry.path();
            if sub.is_dir() && sub.join("zenii-plugin.toml").exists() {
                match self.install_from_local(&sub).await {
                    Ok(plugin) => installed.push(plugin),
                    Err(e) => {
                        tracing::warn!("Skipping plugin in '{}': {e}", sub.display());
                    }
                }
            }
        }

        if installed.is_empty() {
            return Err(ZeniiError::Plugin(format!(
                "no plugins found in '{}'",
                path.display()
            )));
        }

        Ok(installed)
    }

    /// Update a plugin to the latest version (git only).
    pub async fn update(&self, name: &str) -> Result<InstalledPlugin> {
        let plugin = self
            .registry
            .get(name)
            .ok_or_else(|| ZeniiError::PluginNotFound(format!("plugin '{name}' not found")))?;

        let url = match &plugin.source {
            PluginSource::Git { url, .. } => url.clone(),
            _ => {
                return Err(ZeniiError::Plugin(format!(
                    "plugin '{name}' was not installed from git, cannot update"
                )));
            }
        };

        // Remove old, install new
        self.remove(name).await?;
        self.install_from_git(&url).await
    }

    /// Remove an installed plugin.
    pub async fn remove(&self, name: &str) -> Result<()> {
        let plugin = self
            .registry
            .get(name)
            .ok_or_else(|| ZeniiError::PluginNotFound(format!("plugin '{name}' not found")))?;

        // Remove plugin directory
        if plugin.install_path.exists() {
            std::fs::remove_dir_all(&plugin.install_path)
                .map_err(|e| ZeniiError::Plugin(format!("remove dir failed: {e}")))?;
        }

        self.registry.unregister(name)?;
        info!("Removed plugin '{}'", name);
        Ok(())
    }

    /// Register plugin tools and skills into their respective registries.
    async fn register_plugin_assets(&self, plugin: &InstalledPlugin) -> Result<()> {
        // Register tools
        for tool_def in &plugin.manifest.tools {
            let binary = plugin.install_path.join(&tool_def.binary);

            // Fetch real schema from the plugin's info() JSON-RPC method
            let schema = super::fetch_plugin_schema(
                &binary,
                &tool_def.name,
                self.execute_timeout_secs,
                self.max_restart_attempts,
            )
            .await;

            // Create a fresh process for the adapter (the one used for schema fetch is consumed)
            let process = PluginProcess::new(
                &tool_def.name,
                binary,
                self.execute_timeout_secs,
                self.max_restart_attempts,
            );
            let adapter = PluginToolAdapter::new(
                tool_def.name.clone(),
                tool_def.description.clone(),
                schema,
                Arc::new(Mutex::new(process)),
            );
            self.tool_registry
                .register(Arc::new(adapter))
                .unwrap_or_else(|e| {
                    tracing::warn!(
                        "Failed to register tool '{}' from plugin '{}': {e}",
                        tool_def.name,
                        plugin.manifest.plugin.name
                    );
                });
        }

        // Register skills
        for skill_def in &plugin.manifest.skills {
            let skill_path = plugin.install_path.join(&skill_def.file);
            if let Ok(content) = std::fs::read_to_string(&skill_path) {
                if let Err(e) = self
                    .skill_registry
                    .register_external(&skill_def.name, content)
                    .await
                {
                    tracing::warn!("Failed to register skill '{}': {e}", skill_def.name);
                }
            }
        }

        Ok(())
    }

    /// Recursively copy a directory.
    fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
        std::fs::create_dir_all(dst)
            .map_err(|e| ZeniiError::Plugin(format!("create dir failed: {e}")))?;

        for entry in std::fs::read_dir(src)
            .map_err(|e| ZeniiError::Plugin(format!("read dir failed: {e}")))?
        {
            let entry = entry.map_err(|e| ZeniiError::Plugin(format!("dir entry error: {e}")))?;
            let path = entry.path();
            let dest = dst.join(entry.file_name());

            if path.is_dir() {
                // Skip .git directory
                if path.file_name().is_some_and(|n| n == ".git") {
                    continue;
                }
                Self::copy_dir_recursive(&path, &dest)?;
            } else {
                std::fs::copy(&path, &dest)
                    .map_err(|e| ZeniiError::Plugin(format!("copy file failed: {e}")))?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use tempfile::TempDir;

    fn setup_test_env() -> (
        TempDir,
        TempDir,
        Arc<PluginRegistry>,
        Arc<ToolRegistry>,
        Arc<SkillRegistry>,
    ) {
        let plugins_dir = TempDir::new().unwrap();
        let skills_dir = TempDir::new().unwrap();
        let registry = Arc::new(PluginRegistry::new(plugins_dir.path().to_path_buf()).unwrap());
        let tool_registry = Arc::new(ToolRegistry::new());
        let skill_registry = Arc::new(SkillRegistry::new(skills_dir.path(), 100_000).unwrap());
        (
            plugins_dir,
            skills_dir,
            registry,
            tool_registry,
            skill_registry,
        )
    }

    fn create_local_plugin(dir: &TempDir, name: &str) -> PathBuf {
        let plugin_dir = dir.path().join(format!("source-{name}"));
        std::fs::create_dir_all(&plugin_dir).unwrap();
        let manifest = format!(
            r#"[plugin]
name = "{name}"
version = "1.0.0"
description = "Test plugin {name}"

[[tools]]
name = "{name}-tool"
description = "Test tool"
binary = "{name}-tool"
"#
        );
        std::fs::write(plugin_dir.join("zenii-plugin.toml"), manifest).unwrap();

        // Create a dummy binary
        std::fs::write(plugin_dir.join(format!("{name}-tool")), "#!/bin/bash\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                plugin_dir.join(format!("{name}-tool")),
                std::fs::Permissions::from_mode(0o755),
            )
            .unwrap();
        }

        plugin_dir
    }

    // 9.0.17 — Install from local path
    #[tokio::test]
    async fn install_from_local_path() {
        let (plugins_dir, _skills_dir, registry, tool_registry, skill_registry) = setup_test_env();
        let source_dir = TempDir::new().unwrap();
        let plugin_path = create_local_plugin(&source_dir, "local-test");

        let installer = PluginInstaller::new(
            registry.clone(),
            tool_registry.clone(),
            skill_registry,
            60,
            3,
        );

        let installed = installer.install_from_local(&plugin_path).await.unwrap();
        assert_eq!(installed.manifest.plugin.name, "local-test");
        assert!(installed.enabled);

        // Verify it's in the registry
        assert!(registry.get("local-test").is_some());

        // Verify files were copied
        assert!(
            plugins_dir
                .path()
                .join("local-test/zenii-plugin.toml")
                .exists()
        );
    }

    // 9.0.18 — Install validates manifest
    #[tokio::test]
    async fn install_validates_manifest() {
        let (_plugins_dir, _skills_dir, registry, tool_registry, skill_registry) = setup_test_env();
        let source_dir = TempDir::new().unwrap();

        // Create a plugin with an invalid manifest (missing name)
        let bad_dir = source_dir.path().join("bad-plugin");
        std::fs::create_dir_all(&bad_dir).unwrap();
        std::fs::write(
            bad_dir.join("zenii-plugin.toml"),
            r#"[plugin]
name = ""
version = "1.0.0"
description = "Bad"
"#,
        )
        .unwrap();

        let installer = PluginInstaller::new(registry, tool_registry, skill_registry, 60, 3);

        let result = installer.install_from_local(&bad_dir).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name is required"));
    }

    // 9.0.19 — Remove cleans up files
    #[tokio::test]
    async fn remove_cleans_up_files() {
        let (plugins_dir, _skills_dir, registry, tool_registry, skill_registry) = setup_test_env();
        let source_dir = TempDir::new().unwrap();
        let plugin_path = create_local_plugin(&source_dir, "removable");

        let installer =
            PluginInstaller::new(registry.clone(), tool_registry, skill_registry, 60, 3);

        installer.install_from_local(&plugin_path).await.unwrap();
        assert!(plugins_dir.path().join("removable").exists());

        installer.remove("removable").await.unwrap();
        assert!(!plugins_dir.path().join("removable").exists());
        assert!(registry.get("removable").is_none());
    }

    #[tokio::test]
    async fn install_rejects_duplicate() {
        let (_plugins_dir, _skills_dir, registry, tool_registry, skill_registry) = setup_test_env();
        let source_dir = TempDir::new().unwrap();
        let plugin_path = create_local_plugin(&source_dir, "dupe-install");

        let installer = PluginInstaller::new(registry, tool_registry, skill_registry, 60, 3);

        installer.install_from_local(&plugin_path).await.unwrap();
        let result = installer.install_from_local(&plugin_path).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("already installed")
        );
    }

    // --- Phase 9.1: Real plugin installer tests ---

    use crate::plugins::test_helpers::real_plugins_path;

    // 9.1.13 — Install real word-count plugin
    #[tokio::test]
    async fn install_real_word_count() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        let (plugins_dir, _skills_dir, registry, tool_registry, skill_registry) = setup_test_env();
        let installer = PluginInstaller::new(
            registry.clone(),
            tool_registry.clone(),
            skill_registry,
            60,
            3,
        );

        let installed = installer
            .install_from_local(&plugins.join("word-count"))
            .await
            .unwrap();
        assert_eq!(installed.manifest.plugin.name, "word-count");
        assert!(installed.enabled);

        // Verify manifest copied
        assert!(
            plugins_dir
                .path()
                .join("word-count/zenii-plugin.toml")
                .exists()
        );
        // Verify binary copied
        assert!(plugins_dir.path().join("word-count/word-count.py").exists());
        // Verify skill file copied
        assert!(
            plugins_dir
                .path()
                .join("word-count/skills/writing-tips.md")
                .exists()
        );
        // Verify registry has entry
        assert!(registry.get("word-count").is_some());
        // Verify tool_registry has the tool
        assert!(tool_registry.get("word-count").is_some());
    }

    // 9.1.14 — Install real json-formatter plugin
    #[tokio::test]
    async fn install_real_json_formatter() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        let (plugins_dir, _skills_dir, registry, tool_registry, skill_registry) = setup_test_env();
        let installer = PluginInstaller::new(
            registry.clone(),
            tool_registry.clone(),
            skill_registry,
            60,
            3,
        );

        let installed = installer
            .install_from_local(&plugins.join("json-formatter"))
            .await
            .unwrap();
        assert_eq!(installed.manifest.plugin.name, "json-formatter");
        assert!(
            plugins_dir
                .path()
                .join("json-formatter/json-formatter.js")
                .exists()
        );
        assert!(registry.get("json-formatter").is_some());
        assert!(tool_registry.get("json-formatter").is_some());
    }

    // 9.1.15 — Install all real plugins at once
    #[tokio::test]
    async fn install_all_real_plugins() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        let (_plugins_dir, _skills_dir, registry, tool_registry, skill_registry) = setup_test_env();
        let installer =
            PluginInstaller::new(registry.clone(), tool_registry, skill_registry, 60, 3);

        let installed = installer.install_all_from_local(&plugins).await.unwrap();
        assert_eq!(installed.len(), 10);

        // Verify registry has all 10
        let all = registry.list();
        assert_eq!(all.len(), 10);

        // Verify each has correct name
        let names: std::collections::HashSet<String> =
            all.iter().map(|p| p.manifest.plugin.name.clone()).collect();
        assert!(names.contains("word-count"));
        assert!(names.contains("json-formatter"));
        assert!(names.contains("uuid-gen"));
        assert!(names.contains("timestamp"));
        assert!(names.contains("http-client"));
        assert!(names.contains("hash-tool"));
        assert!(names.contains("base64-tool"));
        assert!(names.contains("regex-tester"));
        assert!(names.contains("csv-analyzer"));
        assert!(names.contains("color-converter"));
    }

    // 9.1.16 — Install preserves permissions metadata
    #[tokio::test]
    async fn install_real_plugin_preserves_permissions() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        let (_plugins_dir, _skills_dir, registry, tool_registry, skill_registry) = setup_test_env();
        let installer = PluginInstaller::new(registry, tool_registry, skill_registry, 60, 3);

        let installed = installer
            .install_from_local(&plugins.join("csv-analyzer"))
            .await
            .unwrap();
        assert_eq!(
            installed.manifest.tools[0].permissions.filesystem,
            vec!["*"]
        );
    }

    // 9.1.17 — Install preserves config metadata
    #[tokio::test]
    async fn install_real_plugin_preserves_config() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        let (_plugins_dir, _skills_dir, registry, tool_registry, skill_registry) = setup_test_env();
        let installer = PluginInstaller::new(registry, tool_registry, skill_registry, 60, 3);

        let installed = installer
            .install_from_local(&plugins.join("regex-tester"))
            .await
            .unwrap();
        let cfg = installed
            .manifest
            .config
            .get("default_timeout_ms")
            .expect("config field missing");
        assert_eq!(cfg.field_type, "int");
        assert_eq!(cfg.default, Some(toml::Value::Integer(5000)));
    }
}
