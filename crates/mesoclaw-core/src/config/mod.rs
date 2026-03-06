mod schema;

pub use schema::AppConfig;

use crate::Result;
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

/// Reverse-domain identifier: com.sprklai.mesoclaw
/// Produces platform-correct paths:
///   Linux:   ~/.config/mesoclaw/          ~/.local/share/mesoclaw/
///   macOS:   ~/Library/Application Support/com.sprklai.mesoclaw/
///   Windows: %APPDATA%\sprklai\mesoclaw\
fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "sprklai", "mesoclaw")
}

pub fn default_config_path() -> PathBuf {
    project_dirs()
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.toml")
}

pub fn default_data_dir() -> PathBuf {
    project_dirs()
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn load_config(path: &Path) -> Result<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let content = std::fs::read_to_string(path)?;
    let config: AppConfig = toml::from_str(&content)?;
    Ok(config)
}

pub fn save_config(path: &Path, config: &AppConfig) -> Result<()> {
    let content = toml::to_string_pretty(config)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, content)?;
    Ok(())
}

pub fn load_or_create_config(path: &Path) -> Result<AppConfig> {
    if path.exists() {
        load_config(path)
    } else {
        let config = AppConfig::default();
        save_config(path, &config)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_default_when_missing() {
        let path = Path::new("/tmp/nonexistent_mesoclaw_test.toml");
        let config = load_config(path).unwrap();
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let config = AppConfig {
            gateway_port: 9999,
            ..Default::default()
        };

        save_config(&path, &config).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.gateway_port, 9999);
    }

    #[test]
    fn load_partial_config_fills_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "gateway_port = 4000\n").unwrap();

        let config = load_config(&path).unwrap();
        assert_eq!(config.gateway_port, 4000);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn load_or_create_writes_default_when_missing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("sub").join("config.toml");
        assert!(!path.exists());

        let config = load_or_create_config(&path).unwrap();
        assert_eq!(config, AppConfig::default());
        assert!(path.exists());
    }

    #[test]
    fn load_or_create_reads_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "gateway_port = 7777\n").unwrap();

        let config = load_or_create_config(&path).unwrap();
        assert_eq!(config.gateway_port, 7777);
    }

    #[test]
    fn default_config_path_is_valid() {
        let path = default_config_path();
        assert!(
            path.ends_with("config.toml"),
            "Expected config path to end with config.toml, got: {path:?}"
        );
    }
}
