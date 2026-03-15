use std::path::{Path, PathBuf};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::info;

use super::manifest::PluginManifest;
use crate::{Result, ZeniiError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub manifest: PluginManifest,
    pub install_path: PathBuf,
    pub enabled: bool,
    pub installed_at: String,
    pub source: PluginSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginSource {
    Git { url: String, commit: Option<String> },
    Local { path: PathBuf },
    Bundled,
}

/// Registry index persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct RegistryIndex {
    plugins: Vec<PluginIndexEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginIndexEntry {
    name: String,
    enabled: bool,
    installed_at: String,
    source: PluginSource,
}

/// DashMap-backed registry of installed plugins.
pub struct PluginRegistry {
    plugins_dir: PathBuf,
    plugins: DashMap<String, InstalledPlugin>,
}

impl PluginRegistry {
    /// Create a new PluginRegistry. Scans plugins_dir for installed plugins.
    pub fn new(plugins_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&plugins_dir)
            .map_err(|e| ZeniiError::Plugin(format!("cannot create plugins dir: {e}")))?;

        let registry = Self {
            plugins_dir: plugins_dir.clone(),
            plugins: DashMap::new(),
        };

        // Load from registry.json index if it exists
        let index_path = plugins_dir.join("registry.json");
        if index_path.exists()
            && let Ok(content) = std::fs::read_to_string(&index_path)
            && let Ok(index) = serde_json::from_str::<RegistryIndex>(&content)
        {
            for entry in index.plugins {
                let plugin_dir = plugins_dir.join(&entry.name);
                let manifest_path = plugin_dir.join("zenii-plugin.toml");
                if let Ok(manifest) = PluginManifest::from_file(&manifest_path) {
                    registry.plugins.insert(
                        entry.name.clone(),
                        InstalledPlugin {
                            manifest,
                            install_path: plugin_dir,
                            enabled: entry.enabled,
                            installed_at: entry.installed_at,
                            source: entry.source,
                        },
                    );
                }
            }
        } else {
            // Scan directory for plugins without index
            registry.scan_directory()?;
        }

        info!(
            "Plugin registry loaded: {} plugins from {}",
            registry.plugins.len(),
            plugins_dir.display()
        );

        Ok(registry)
    }

    /// Scan plugins directory for installed plugins (fallback when no index).
    fn scan_directory(&self) -> Result<()> {
        let entries = std::fs::read_dir(&self.plugins_dir)
            .map_err(|e| ZeniiError::Plugin(format!("cannot read plugins dir: {e}")))?;

        for entry in entries {
            let entry = entry.map_err(|e| ZeniiError::Plugin(format!("dir entry error: {e}")))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("zenii-plugin.toml");
            if !manifest_path.exists() {
                continue;
            }

            match PluginManifest::from_file(&manifest_path) {
                Ok(manifest) => {
                    let name = manifest.plugin.name.clone();
                    self.plugins.insert(
                        name.clone(),
                        InstalledPlugin {
                            manifest,
                            install_path: path,
                            enabled: true,
                            installed_at: chrono::Utc::now().to_rfc3339(),
                            source: PluginSource::Local {
                                path: PathBuf::new(),
                            },
                        },
                    );
                    info!("Discovered plugin: {name}");
                }
                Err(e) => {
                    tracing::warn!("Invalid plugin at {}: {e}", path.display());
                }
            }
        }

        Ok(())
    }

    /// List all installed plugins.
    pub fn list(&self) -> Vec<InstalledPlugin> {
        self.plugins.iter().map(|r| r.value().clone()).collect()
    }

    /// Get a plugin by name.
    pub fn get(&self, name: &str) -> Option<InstalledPlugin> {
        self.plugins.get(name).map(|r| r.value().clone())
    }

    /// Register a newly installed plugin.
    pub fn register(&self, plugin: InstalledPlugin) -> Result<()> {
        let name = plugin.manifest.plugin.name.clone();
        if self.plugins.contains_key(&name) {
            return Err(ZeniiError::Plugin(format!(
                "plugin '{name}' already registered"
            )));
        }
        self.plugins.insert(name, plugin);
        self.save_index()?;
        Ok(())
    }

    /// Unregister a plugin (does not delete files).
    pub fn unregister(&self, name: &str) -> Result<()> {
        if self.plugins.remove(name).is_none() {
            return Err(ZeniiError::PluginNotFound(format!(
                "plugin '{name}' not found"
            )));
        }
        self.save_index()?;
        Ok(())
    }

    /// Enable a disabled plugin.
    pub fn enable(&self, name: &str) -> Result<()> {
        let mut entry = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| ZeniiError::PluginNotFound(format!("plugin '{name}' not found")))?;
        entry.enabled = true;
        drop(entry);
        self.save_index()?;
        Ok(())
    }

    /// Disable a plugin without removing it.
    pub fn disable(&self, name: &str) -> Result<()> {
        let mut entry = self
            .plugins
            .get_mut(name)
            .ok_or_else(|| ZeniiError::PluginNotFound(format!("plugin '{name}' not found")))?;
        entry.enabled = false;
        drop(entry);
        self.save_index()?;
        Ok(())
    }

    /// Get the plugins directory path.
    pub fn plugins_dir(&self) -> &Path {
        &self.plugins_dir
    }

    /// Persist the registry index to disk.
    pub fn save_index(&self) -> Result<()> {
        let entries: Vec<PluginIndexEntry> = self
            .plugins
            .iter()
            .map(|r| {
                let p = r.value();
                PluginIndexEntry {
                    name: p.manifest.plugin.name.clone(),
                    enabled: p.enabled,
                    installed_at: p.installed_at.clone(),
                    source: p.source.clone(),
                }
            })
            .collect();

        let index = RegistryIndex { plugins: entries };
        let content = serde_json::to_string_pretty(&index)
            .map_err(|e| ZeniiError::Plugin(format!("failed to serialize index: {e}")))?;

        let index_path = self.plugins_dir.join("registry.json");
        std::fs::write(&index_path, content)
            .map_err(|e| ZeniiError::Plugin(format!("failed to write index: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_plugin_dir(parent: &Path, name: &str) {
        let dir = parent.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = format!(
            r#"[plugin]
name = "{name}"
version = "1.0.0"
description = "Test plugin {name}"
"#
        );
        std::fs::write(dir.join("zenii-plugin.toml"), manifest).unwrap();
    }

    // 9.0.13 — Registry register/unregister
    #[test]
    fn registry_register_unregister() {
        let dir = TempDir::new().unwrap();
        let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();

        let plugin = InstalledPlugin {
            manifest: PluginManifest::parse(
                r#"[plugin]
name = "test"
version = "1.0.0"
description = "Test""#,
            )
            .unwrap(),
            install_path: dir.path().join("test"),
            enabled: true,
            installed_at: "2026-01-01T00:00:00Z".into(),
            source: PluginSource::Local {
                path: PathBuf::from("/tmp/test"),
            },
        };

        registry.register(plugin).unwrap();
        assert_eq!(registry.list().len(), 1);
        assert!(registry.get("test").is_some());

        registry.unregister("test").unwrap();
        assert_eq!(registry.list().len(), 0);
        assert!(registry.get("test").is_none());
    }

    // 9.0.14 — Registry enable/disable
    #[test]
    fn registry_enable_disable() {
        let dir = TempDir::new().unwrap();
        let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();

        let plugin = InstalledPlugin {
            manifest: PluginManifest::parse(
                r#"[plugin]
name = "toggle"
version = "1.0.0"
description = "Test""#,
            )
            .unwrap(),
            install_path: dir.path().join("toggle"),
            enabled: true,
            installed_at: "2026-01-01T00:00:00Z".into(),
            source: PluginSource::Bundled,
        };

        registry.register(plugin).unwrap();
        assert!(registry.get("toggle").unwrap().enabled);

        registry.disable("toggle").unwrap();
        assert!(!registry.get("toggle").unwrap().enabled);

        registry.enable("toggle").unwrap();
        assert!(registry.get("toggle").unwrap().enabled);
    }

    // 9.0.15 — Registry persist index
    #[test]
    fn registry_persist_index() {
        let dir = TempDir::new().unwrap();

        // Register a plugin
        {
            let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();
            let plugin = InstalledPlugin {
                manifest: PluginManifest::parse(
                    r#"[plugin]
name = "persist"
version = "1.0.0"
description = "Persist test""#,
                )
                .unwrap(),
                install_path: dir.path().join("persist"),
                enabled: true,
                installed_at: "2026-01-01T00:00:00Z".into(),
                source: PluginSource::Git {
                    url: "https://example.com/plugin.git".into(),
                    commit: Some("abc123".into()),
                },
            };
            // Create the plugin dir with manifest for reload
            std::fs::create_dir_all(dir.path().join("persist")).unwrap();
            std::fs::write(
                dir.path().join("persist/zenii-plugin.toml"),
                r#"[plugin]
name = "persist"
version = "1.0.0"
description = "Persist test""#,
            )
            .unwrap();

            registry.register(plugin).unwrap();
        }

        // Verify index file exists
        assert!(dir.path().join("registry.json").exists());

        // Reload from index
        let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();
        assert_eq!(registry.list().len(), 1);
        assert!(registry.get("persist").is_some());
    }

    // 9.0.16 — Registry scan directory
    #[test]
    fn registry_scan_directory() {
        let dir = TempDir::new().unwrap();

        // Create plugin dirs WITHOUT registry.json
        make_plugin_dir(dir.path(), "alpha");
        make_plugin_dir(dir.path(), "beta");

        let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();
        assert_eq!(registry.list().len(), 2);
        assert!(registry.get("alpha").is_some());
        assert!(registry.get("beta").is_some());
    }

    #[test]
    fn register_rejects_duplicate() {
        let dir = TempDir::new().unwrap();
        let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();

        let plugin = InstalledPlugin {
            manifest: PluginManifest::parse(
                r#"[plugin]
name = "dupe"
version = "1.0.0"
description = "Test""#,
            )
            .unwrap(),
            install_path: dir.path().join("dupe"),
            enabled: true,
            installed_at: "2026-01-01T00:00:00Z".into(),
            source: PluginSource::Bundled,
        };

        registry.register(plugin.clone()).unwrap();
        let result = registry.register(plugin);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("already registered")
        );
    }

    #[test]
    fn unregister_nonexistent_fails() {
        let dir = TempDir::new().unwrap();
        let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();
        let result = registry.unregister("nonexistent");
        assert!(result.is_err());
    }

    // --- Phase 9.1: Real plugin registry tests ---

    use crate::plugins::test_helpers::real_plugins_path;

    /// Recursively copy a directory (test-only helper).
    fn copy_dir_recursive(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let dest = dst.join(entry.file_name());
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == ".git") {
                    continue;
                }
                copy_dir_recursive(&path, &dest);
            } else {
                std::fs::copy(&path, &dest).unwrap();
            }
        }
    }

    // 9.1.18 — Registry discovers real plugins via scan
    #[test]
    fn registry_scan_real_plugins() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        let dir = TempDir::new().unwrap();

        // Copy all real plugin dirs into temp dir
        let dirs = [
            "word-count",
            "json-formatter",
            "uuid-gen",
            "timestamp",
            "http-client",
            "hash-tool",
            "base64-tool",
            "regex-tester",
            "csv-analyzer",
            "color-converter",
        ];
        for d in dirs {
            copy_dir_recursive(&plugins.join(d), &dir.path().join(d));
        }

        // Create registry WITHOUT registry.json — forces scan
        let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();
        let all = registry.list();
        assert_eq!(all.len(), 10, "Expected 10 plugins, got {}", all.len());
    }

    // 9.1.19 — Registry persist and reload real plugins
    #[test]
    fn registry_persist_reload_real() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        let dir = TempDir::new().unwrap();

        // Copy plugins and register them
        let plugin_names = ["word-count", "json-formatter", "uuid-gen"];
        {
            let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();
            for name in &plugin_names {
                copy_dir_recursive(&plugins.join(name), &dir.path().join(name));
                let manifest =
                    PluginManifest::from_file(&dir.path().join(name).join("zenii-plugin.toml"))
                        .unwrap();
                registry
                    .register(InstalledPlugin {
                        manifest,
                        install_path: dir.path().join(name),
                        enabled: true,
                        installed_at: "2026-01-01T00:00:00Z".into(),
                        source: PluginSource::Local {
                            path: plugins.join(name),
                        },
                    })
                    .unwrap();
            }
            // save_index is called by register, but let's be explicit
            registry.save_index().unwrap();
        }

        // Reload from same dir
        let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();
        assert_eq!(registry.list().len(), 3);
        for name in &plugin_names {
            let p = registry
                .get(name)
                .expect(&format!("{name} not found after reload"));
            assert_eq!(p.manifest.plugin.name, *name);
        }
    }

    // 9.1.20 — Toggle real plugin enable/disable and persist
    #[test]
    fn registry_toggle_real_plugin() {
        let Some(plugins) = real_plugins_path() else {
            eprintln!("SKIP: real plugins path not available");
            return;
        };
        let dir = TempDir::new().unwrap();

        // Copy word-count into temp dir
        copy_dir_recursive(&plugins.join("word-count"), &dir.path().join("word-count"));

        // Registry::new auto-discovers the plugin via scan (no registry.json)
        let registry = PluginRegistry::new(dir.path().to_path_buf()).unwrap();
        assert!(
            registry.get("word-count").is_some(),
            "word-count should be discovered by scan"
        );

        // Disable
        registry.disable("word-count").unwrap();
        assert!(!registry.get("word-count").unwrap().enabled);

        // Enable
        registry.enable("word-count").unwrap();
        assert!(registry.get("word-count").unwrap().enabled);

        // Persist & reload — state should survive
        registry.save_index().unwrap();
        drop(registry);

        let registry2 = PluginRegistry::new(dir.path().to_path_buf()).unwrap();
        assert!(registry2.get("word-count").unwrap().enabled);
    }
}
