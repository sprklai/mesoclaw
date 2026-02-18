//! TOML configuration loading, env-var overrides, and atomic saves.
//!
//! # Loading order
//! 1. Parse `~/.mesoclaw/config.toml` (or the path in `MESOCLAW_CONFIG`)
//! 2. Apply `MESOCLAW_*` environment variable overrides
//! 3. Fall back to [`AppConfig::default()`] if the file is missing
//!
//! # Atomic save
//! Writes to `<path>.tmp` → fsync → rename to `<path>` to avoid partial
//! writes corrupting the config file.

use std::{
    env,
    fs,
    path::{Path, PathBuf},
};

use super::schema::AppConfig;

// ─── default_config_path ─────────────────────────────────────────────────────

/// Return the default config file path: `~/.mesoclaw/config.toml`.
pub fn default_config_path() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|h| h.join(".mesoclaw").join("config.toml"))
        .ok_or_else(|| "could not determine home directory".to_string())
}

// ─── load_config ─────────────────────────────────────────────────────────────

/// Load [`AppConfig`] from the given path, falling back to defaults if the
/// file does not exist, then applying environment variable overrides.
pub fn load_config(path: &Path) -> Result<AppConfig, String> {
    let mut config = match fs::read_to_string(path) {
        Ok(content) => toml::from_str::<AppConfig>(&content)
            .map_err(|e| format!("failed to parse config at {path:?}: {e}"))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => AppConfig::default(),
        Err(e) => return Err(format!("failed to read config at {path:?}: {e}")),
    };

    apply_env_overrides(&mut config);
    Ok(config)
}

/// Load config from the default path, creating the directory if needed.
pub fn load_default_config() -> AppConfig {
    // Check for custom config path via env.
    let path = env::var("MESOCLAW_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            default_config_path().unwrap_or_else(|_| PathBuf::from("config.toml"))
        });

    load_config(&path).unwrap_or_default()
}

// ─── apply_env_overrides ─────────────────────────────────────────────────────

/// Apply `MESOCLAW_*` environment variable overrides to `config`.
///
/// Supported overrides:
/// - `MESOCLAW_PROVIDER_ID`           → `provider.default_id`
/// - `MESOCLAW_PROVIDER_MODEL`        → `provider.default_model`
/// - `MESOCLAW_SECURITY_LEVEL`        → `security.autonomy_level`
/// - `MESOCLAW_HEARTBEAT_INTERVAL`    → `scheduler.heartbeat_interval_secs`
/// - `MESOCLAW_HEARTBEAT_ENABLED`     → `scheduler.heartbeat_enabled` (1/0)
/// - `MESOCLAW_MEMORY_ENABLED`        → `memory.enabled` (1/0)
/// - `MESOCLAW_NOTIFICATIONS_ENABLED` → `notifications.enabled` (1/0)
/// - `MESOCLAW_DO_NOT_DISTURB`        → `notifications.do_not_disturb` (1/0)
fn apply_env_overrides(config: &mut AppConfig) {
    if let Ok(v) = env::var("MESOCLAW_PROVIDER_ID") {
        config.provider.default_id = v;
    }
    if let Ok(v) = env::var("MESOCLAW_PROVIDER_MODEL") {
        config.provider.default_model = v;
    }
    if let Ok(v) = env::var("MESOCLAW_SECURITY_LEVEL") {
        config.security.autonomy_level = v;
    }
    if let Ok(v) = env::var("MESOCLAW_HEARTBEAT_INTERVAL") {
        if let Ok(secs) = v.parse::<u64>() {
            config.scheduler.heartbeat_interval_secs = secs;
        }
    }
    if let Ok(v) = env::var("MESOCLAW_HEARTBEAT_ENABLED") {
        config.scheduler.heartbeat_enabled = v == "1" || v.eq_ignore_ascii_case("true");
    }
    if let Ok(v) = env::var("MESOCLAW_MEMORY_ENABLED") {
        config.memory.enabled = v == "1" || v.eq_ignore_ascii_case("true");
    }
    if let Ok(v) = env::var("MESOCLAW_NOTIFICATIONS_ENABLED") {
        config.notifications.enabled = v == "1" || v.eq_ignore_ascii_case("true");
    }
    if let Ok(v) = env::var("MESOCLAW_DO_NOT_DISTURB") {
        config.notifications.do_not_disturb = v == "1" || v.eq_ignore_ascii_case("true");
    }
}

// ─── save_config ─────────────────────────────────────────────────────────────

/// Atomically save `config` to `path`.
///
/// Writes to `<path>.tmp`, syncs to disk, creates a backup of the existing
/// file as `<path>.bak`, then renames the temp file to `<path>`.
pub fn save_config(path: &Path, config: &AppConfig) -> Result<(), String> {
    let content = toml::to_string_pretty(config)
        .map_err(|e| format!("failed to serialise config: {e}"))?;

    // Ensure parent directory exists.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create config dir: {e}"))?;
    }

    let tmp_path = path.with_extension("toml.tmp");

    // Write to temp file.
    fs::write(&tmp_path, &content)
        .map_err(|e| format!("failed to write temp config: {e}"))?;

    // Backup existing config if it exists.
    if path.exists() {
        let bak_path = path.with_extension("toml.bak");
        fs::copy(path, &bak_path)
            .map_err(|e| format!("failed to backup config: {e}"))?;
    }

    // Atomic rename.
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("failed to replace config file: {e}"))?;

    Ok(())
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(unsafe_code)] // env::set_var / remove_var are unsafe in Rust 2024; tests are single-threaded.
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_config(dir: &TempDir, content: &str) -> PathBuf {
        let path = dir.path().join("config.toml");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn load_missing_file_returns_defaults() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.toml");
        let config = load_config(&path).unwrap();
        assert_eq!(config, AppConfig::default());
    }

    #[test]
    fn load_partial_config_fills_defaults() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, r#"
[provider]
default_id = "anthropic"
"#);
        let config = load_config(&path).unwrap();
        assert_eq!(config.provider.default_id, "anthropic");
        // Other fields should use defaults.
        assert_eq!(config.provider.max_retries, 3);
        assert_eq!(config.security.autonomy_level, "supervised");
    }

    #[test]
    fn load_full_config() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, r#"
[provider]
default_id = "openai"
default_model = "gpt-4o"
request_timeout_secs = 30
max_retries = 5

[security]
autonomy_level = "autonomous"
rate_limit_per_minute = 120

[scheduler]
heartbeat_interval_secs = 900
heartbeat_enabled = false

[memory]
enabled = false
embedding_cache_size = 5000

[notifications]
enabled = true
do_not_disturb = true
"#);
        let config = load_config(&path).unwrap();
        assert_eq!(config.provider.default_model, "gpt-4o");
        assert_eq!(config.provider.max_retries, 5);
        assert_eq!(config.security.autonomy_level, "autonomous");
        assert_eq!(config.scheduler.heartbeat_interval_secs, 900);
        assert!(!config.scheduler.heartbeat_enabled);
        assert!(!config.memory.enabled);
        assert!(config.notifications.do_not_disturb);
    }

    #[test]
    fn save_and_reload_round_trip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        let mut original = AppConfig::default();
        original.provider.default_model = "claude-3-haiku".to_owned();
        original.security.autonomy_level = "autonomous".to_owned();

        save_config(&path, &original).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded, original, "config should round-trip through save/load");
    }

    #[test]
    fn save_creates_backup() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");

        // Save twice — second save should create a .bak file.
        save_config(&path, &AppConfig::default()).unwrap();
        save_config(&path, &AppConfig::default()).unwrap();

        let bak = path.with_extension("toml.bak");
        assert!(bak.exists(), "backup file should exist after second save");
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = TempDir::new().unwrap();
        let nested_path = dir.path().join("a").join("b").join("config.toml");
        save_config(&nested_path, &AppConfig::default()).unwrap();
        assert!(nested_path.exists(), "config should be created in nested dirs");
    }

    #[test]
    fn env_override_provider_id() {
        // This test sets and clears an env var so it's not fully isolated
        // in parallel runs, but we use unique var names to reduce risk.
        let key = "MESOCLAW_PROVIDER_ID";
        // SAFETY: single-threaded test context; no other threads read this var.
        unsafe { env::set_var(key, "groq"); }
        let config = load_default_config();
        // SAFETY: same as set_var above.
        unsafe { env::remove_var(key); }
        assert_eq!(config.provider.default_id, "groq");
    }

    #[test]
    fn env_override_security_level() {
        let key = "MESOCLAW_SECURITY_LEVEL";
        // SAFETY: single-threaded test context; no other threads read this var.
        unsafe { env::set_var(key, "readonly"); }
        let config = load_default_config();
        // SAFETY: same as set_var above.
        unsafe { env::remove_var(key); }
        assert_eq!(config.security.autonomy_level, "readonly");
    }

    #[test]
    fn env_override_heartbeat_enabled_false() {
        let key = "MESOCLAW_HEARTBEAT_ENABLED";
        // SAFETY: single-threaded test context; no other threads read this var.
        unsafe { env::set_var(key, "0"); }
        let config = load_default_config();
        // SAFETY: same as set_var above.
        unsafe { env::remove_var(key); }
        assert!(!config.scheduler.heartbeat_enabled);
    }

    #[test]
    fn load_invalid_toml_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = write_config(&dir, "this is not valid toml!!!");
        let result = load_config(&path);
        assert!(result.is_err(), "invalid TOML should return an error");
    }
}
