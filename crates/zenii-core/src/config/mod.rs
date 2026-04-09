mod schema;

pub use schema::AppConfig;

use crate::Result;
use directories::ProjectDirs;
use std::path::{Path, PathBuf};

/// Reverse-domain identifier: com.sprklai.zenii
/// Produces platform-correct paths:
///   Linux:   ~/.config/zenii/          ~/.local/share/zenii/
///   macOS:   ~/Library/Application Support/com.sprklai.zenii/
///   Windows: %APPDATA%\sprklai\zenii\
fn project_dirs() -> Option<ProjectDirs> {
    ProjectDirs::from("com", "sprklai", "zenii")
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
    let mut config: AppConfig = toml::from_str(&content)?;
    config.validate();
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
        let path = std::env::temp_dir().join("nonexistent_zenii_test.toml");
        let config = load_config(&path).unwrap();
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

    #[test]
    fn phase3_config_defaults() {
        let config = AppConfig::default();
        assert!(config.gateway_auth_token.is_none());
        assert_eq!(config.ws_max_connections, 32);
        assert_eq!(config.agent_max_turns, 8);
        assert_eq!(config.agent_max_tokens, 4096);
        assert!(config.agent_system_prompt.is_none());
    }

    #[test]
    fn provider_config_deserializes() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
provider_name = "anthropic"
provider_type = "anthropic"
provider_model_id = "claude-sonnet-4-20250514"
provider_api_key_env = "ANTHROPIC_API_KEY"
"#,
        )
        .unwrap();

        let config = load_config(&path).unwrap();
        assert_eq!(config.provider_name, "anthropic");
        assert_eq!(config.provider_type, "anthropic");
        assert_eq!(config.provider_model_id, "claude-sonnet-4-20250514");
        assert_eq!(
            config.provider_api_key_env,
            Some("ANTHROPIC_API_KEY".into())
        );
    }

    #[test]
    fn auth_token_optional() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        std::fs::write(&path, "gateway_port = 18981\n").unwrap();
        let config = load_config(&path).unwrap();
        assert!(config.gateway_auth_token.is_none());

        std::fs::write(
            &path,
            r#"gateway_auth_token = "secret123"
"#,
        )
        .unwrap();
        let config = load_config(&path).unwrap();
        assert_eq!(config.gateway_auth_token, Some("secret123".into()));
    }

    #[test]
    fn phase4_config_defaults() {
        let config = AppConfig::default();
        assert!(config.identity_dir.is_none());
        assert!(config.skills_dir.is_none());
        assert_eq!(config.skill_max_content_size, 100_000);
        assert!(config.learning_enabled);
        assert!(config.learning_denied_categories.is_empty());
        assert_eq!(config.learning_max_observations, 10_000);
        assert_eq!(config.learning_observation_ttl_days, 365);
        assert_eq!(config.learning_min_confidence, 0.5);
    }

    #[test]
    fn phase4_config_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let config = AppConfig {
            learning_enabled: false,
            learning_denied_categories: vec!["personal".into()],
            skill_max_content_size: 50_000,
            ..Default::default()
        };

        save_config(&path, &config).unwrap();
        let loaded = load_config(&path).unwrap();
        assert!(!loaded.learning_enabled);
        assert_eq!(loaded.learning_denied_categories, vec!["personal"]);
        assert_eq!(loaded.skill_max_content_size, 50_000);
    }

    #[test]
    fn channel_config_defaults() {
        let config = AppConfig::default();
        assert!(config.channels_enabled.is_empty());
        assert_eq!(config.telegram_polling_timeout_secs, 30);
        assert_eq!(config.telegram_dm_policy, "allowlist");
        assert_eq!(config.telegram_retry_min_ms, 1000);
        assert_eq!(config.telegram_retry_max_ms, 60_000);
        assert!(config.telegram_require_group_mention);
    }

    #[test]
    fn backwards_compat_aliases() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
default_provider = "my-prov"
default_model = "my-model"
"#,
        )
        .unwrap();

        let config = load_config(&path).unwrap();
        assert_eq!(config.provider_name, "my-prov");
        assert_eq!(config.provider_model_id, "my-model");
    }
}

mod wiki_config_tests;
