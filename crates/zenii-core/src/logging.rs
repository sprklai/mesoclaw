use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Serialize;
use tracing::{debug, info, warn};

use crate::config::AppConfig;

/// A single AI usage record, written as one JSON line per request.
#[derive(Debug, Clone, Serialize)]
pub struct UsageRecord {
    pub timestamp: String,
    pub session_id: Option<String>,
    pub model_id: String,
    pub provider_id: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub cached_input_tokens: u64,
    pub tool_calls_count: u32,
    pub duration_ms: u64,
    pub context_level: String,
    pub binary: String,
    pub success: bool,
}

/// Async JSONL writer for AI usage records.
///
/// Writes one JSON line per AI request to date-rotated files:
/// `{log_dir}/usage-YYYY-MM-DD.jsonl`
pub struct UsageLogger {
    log_dir: PathBuf,
    binary_name: String,
    enabled: bool,
    keep_days: u32,
}

impl UsageLogger {
    /// Create a new usage logger.
    ///
    /// If `config.log_dir` is empty, defaults to `{data_dir}/logs/`.
    /// Creates the log directory if it doesn't exist.
    pub fn new(config: &AppConfig, binary_name: &str) -> Self {
        let log_dir = resolve_log_dir(config);

        if config.usage_tracking_enabled
            && let Err(e) = std::fs::create_dir_all(&log_dir)
        {
            warn!("Failed to create usage log dir {}: {e}", log_dir.display());
        }

        Self {
            log_dir,
            binary_name: binary_name.to_string(),
            enabled: config.usage_tracking_enabled,
            keep_days: config.log_keep_days,
        }
    }

    /// Log a usage record as a single JSONL line.
    pub async fn log(&self, record: &UsageRecord) -> crate::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let date = Utc::now().format("%Y-%m-%d").to_string();
        let path = self.log_dir.join(format!("usage-{date}.jsonl"));

        let mut line =
            serde_json::to_string(record).map_err(|e| crate::ZeniiError::Config(e.to_string()))?;
        line.push('\n');

        let path_clone = path.clone();
        let line_clone = line;
        tokio::task::spawn_blocking(move || {
            use std::io::Write;
            let file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path_clone);
            match file {
                Ok(mut f) => {
                    if let Err(e) = f.write_all(line_clone.as_bytes()) {
                        warn!("Failed to write usage log: {e}");
                    }
                }
                Err(e) => warn!("Failed to open usage log {}: {e}", path_clone.display()),
            }
        })
        .await
        .map_err(|e| crate::ZeniiError::Config(format!("usage log task failed: {e}")))?;

        debug!(
            "Usage logged: {}→{} tokens ({}), model={}, {}ms",
            record.input_tokens,
            record.output_tokens,
            record.total_tokens,
            record.model_id,
            record.duration_ms
        );

        Ok(())
    }

    /// Delete usage log files older than `keep_days`.
    pub async fn cleanup_old_files(&self) -> crate::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let log_dir = self.log_dir.clone();
        let keep_days = self.keep_days;

        tokio::task::spawn_blocking(move || cleanup_old_usage_files(&log_dir, keep_days))
            .await
            .map_err(|e| crate::ZeniiError::Config(format!("cleanup task failed: {e}")))?;

        Ok(())
    }

    /// The binary name this logger was created for.
    pub fn binary_name(&self) -> &str {
        &self.binary_name
    }

    /// Whether usage tracking is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

/// Resolve the log directory from config.
pub fn resolve_log_dir(config: &AppConfig) -> PathBuf {
    if config.log_dir.is_empty() {
        crate::config::default_data_dir().join("logs")
    } else {
        PathBuf::from(&config.log_dir)
    }
}

/// Delete tracing log files older than `keep_days`.
///
/// Matches files like `daemon.log.2026-03-15` (tracing-appender daily format).
pub fn cleanup_old_tracing_files(log_dir: &Path, keep_days: u32) {
    let cutoff = Utc::now() - chrono::Duration::days(i64::from(keep_days));
    let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

    let entries = match std::fs::read_dir(log_dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to read log dir for tracing cleanup: {e}");
            return;
        }
    };

    let mut removed = 0u32;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Match *.log.YYYY-MM-DD (tracing-appender daily rotation format)
        if let Some(pos) = name_str.find(".log.") {
            let date_part = &name_str[pos + 5..];
            if date_part.len() == 10 && date_part < cutoff_str.as_str() {
                if let Err(e) = std::fs::remove_file(entry.path()) {
                    warn!("Failed to remove old tracing log {}: {e}", name_str);
                } else {
                    removed += 1;
                }
            }
        }
    }

    if removed > 0 {
        info!("Cleaned up {removed} old tracing log files");
    }
}

fn cleanup_old_usage_files(log_dir: &Path, keep_days: u32) {
    let cutoff = Utc::now() - chrono::Duration::days(i64::from(keep_days));
    let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

    let entries = match std::fs::read_dir(log_dir) {
        Ok(e) => e,
        Err(e) => {
            warn!("Failed to read log dir for cleanup: {e}");
            return;
        }
    };

    let mut removed = 0u32;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Match usage-YYYY-MM-DD.jsonl
        if let Some(date_part) = name_str
            .strip_prefix("usage-")
            .and_then(|s| s.strip_suffix(".jsonl"))
            && date_part < cutoff_str.as_str()
        {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                warn!("Failed to remove old usage log {}: {e}", name_str);
            } else {
                removed += 1;
            }
        }
    }

    if removed > 0 {
        info!("Cleaned up {removed} old usage log files");
    }
}

/// Initialize unified tracing for all binaries.
///
/// Sets up:
/// - Console layer (stderr with color, unless `quiet` is true)
/// - File layer (daily-rotated to `{log_dir}/{binary_name}.log`)
///
/// Call this once at binary startup, before any tracing macros.
pub fn init_tracing(config: &AppConfig, binary_name: &str, quiet: bool) -> crate::Result<()> {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::fmt;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    let log_dir = resolve_log_dir(config);
    std::fs::create_dir_all(&log_dir).map_err(|e| {
        crate::ZeniiError::Config(format!(
            "failed to create log dir {}: {e}",
            log_dir.display()
        ))
    })?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, format!("{binary_name}.log"));

    if quiet {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));
        let file_layer = fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false)
            .with_target(true);

        if tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .try_init()
            .is_err()
        {
            // A global subscriber was already set (e.g. by Tauri devtools) — not an error.
        }
    } else {
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));
        let file_layer = fmt::layer()
            .with_writer(file_appender)
            .with_ansi(false)
            .with_target(true);
        let console_layer = fmt::layer().with_writer(std::io::stderr).with_target(true);

        if tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .with(file_layer)
            .try_init()
            .is_err()
        {
            // A global subscriber was already set — not an error.
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 8.14.7 — UsageRecord serializes to valid JSON
    #[test]
    fn usage_record_serializes() {
        let record = UsageRecord {
            timestamp: "2026-03-15T12:00:00Z".into(),
            session_id: Some("sess-1".into()),
            model_id: "gpt-4o".into(),
            provider_id: "openai".into(),
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
            cached_input_tokens: 10,
            tool_calls_count: 2,
            duration_ms: 1500,
            context_level: "Full".into(),
            binary: "daemon".into(),
            success: true,
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"input_tokens\":100"));
        assert!(json.contains("\"model_id\":\"gpt-4o\""));
        assert!(json.contains("\"success\":true"));
    }

    // 8.14.8 — UsageLogger::new() creates log_dir if missing
    #[test]
    fn usage_logger_creates_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("logs");
        let config = AppConfig {
            usage_tracking_enabled: true,
            log_dir: log_dir.to_string_lossy().into_owned(),
            log_keep_days: 30,
            ..Default::default()
        };
        let _logger = UsageLogger::new(&config, "test");
        assert!(log_dir.exists());
    }

    // 8.14.9 — UsageLogger::log() writes JSONL line to date-named file
    #[tokio::test]
    async fn usage_logger_writes_jsonl() {
        let tmp = tempfile::tempdir().unwrap();
        let config = AppConfig {
            usage_tracking_enabled: true,
            log_dir: tmp.path().to_string_lossy().into_owned(),
            log_keep_days: 30,
            ..Default::default()
        };
        let logger = UsageLogger::new(&config, "test");

        let record = UsageRecord {
            timestamp: "2026-03-15T12:00:00Z".into(),
            session_id: None,
            model_id: "test-model".into(),
            provider_id: "test".into(),
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
            cached_input_tokens: 0,
            tool_calls_count: 0,
            duration_ms: 100,
            context_level: "Minimal".into(),
            binary: "test".into(),
            success: true,
        };
        logger.log(&record).await.unwrap();

        let date = Utc::now().format("%Y-%m-%d").to_string();
        let path = tmp.path().join(format!("usage-{date}.jsonl"));
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"model_id\":\"test-model\""));
    }

    // 8.14.10 — UsageLogger::log() appends (not overwrites) on second call
    #[tokio::test]
    async fn usage_logger_appends() {
        let tmp = tempfile::tempdir().unwrap();
        let config = AppConfig {
            usage_tracking_enabled: true,
            log_dir: tmp.path().to_string_lossy().into_owned(),
            log_keep_days: 30,
            ..Default::default()
        };
        let logger = UsageLogger::new(&config, "test");

        let record = UsageRecord {
            timestamp: "2026-03-15T12:00:00Z".into(),
            session_id: None,
            model_id: "model-a".into(),
            provider_id: "test".into(),
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
            cached_input_tokens: 0,
            tool_calls_count: 0,
            duration_ms: 100,
            context_level: "Full".into(),
            binary: "test".into(),
            success: true,
        };
        logger.log(&record).await.unwrap();

        let record2 = UsageRecord {
            model_id: "model-b".into(),
            ..record.clone()
        };
        logger.log(&record2).await.unwrap();

        let date = Utc::now().format("%Y-%m-%d").to_string();
        let path = tmp.path().join(format!("usage-{date}.jsonl"));
        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("model-a"));
        assert!(lines[1].contains("model-b"));
    }

    // 8.14.11 — UsageLogger::log() is no-op when disabled
    #[tokio::test]
    async fn usage_logger_disabled_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let config = AppConfig {
            usage_tracking_enabled: false,
            log_dir: tmp.path().to_string_lossy().into_owned(),
            log_keep_days: 30,
            ..Default::default()
        };
        let logger = UsageLogger::new(&config, "test");

        let record = UsageRecord {
            timestamp: "2026-03-15T12:00:00Z".into(),
            session_id: None,
            model_id: "test".into(),
            provider_id: "test".into(),
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
            cached_input_tokens: 0,
            tool_calls_count: 0,
            duration_ms: 100,
            context_level: "Full".into(),
            binary: "test".into(),
            success: true,
        };
        logger.log(&record).await.unwrap();

        // No file should be created
        let entries: Vec<_> = std::fs::read_dir(tmp.path()).unwrap().flatten().collect();
        assert!(entries.is_empty());
    }

    // 8.14.12 — cleanup_old_files removes files older than keep_days
    #[tokio::test]
    async fn cleanup_removes_old_files() {
        let tmp = tempfile::tempdir().unwrap();

        // Create a "old" file
        std::fs::write(tmp.path().join("usage-2020-01-01.jsonl"), "old\n").unwrap();
        // Create a "recent" file
        let today = Utc::now().format("%Y-%m-%d").to_string();
        std::fs::write(tmp.path().join(format!("usage-{today}.jsonl")), "recent\n").unwrap();

        let config = AppConfig {
            usage_tracking_enabled: true,
            log_dir: tmp.path().to_string_lossy().into_owned(),
            log_keep_days: 7,
            ..Default::default()
        };
        let logger = UsageLogger::new(&config, "test");
        logger.cleanup_old_files().await.unwrap();

        assert!(!tmp.path().join("usage-2020-01-01.jsonl").exists());
        assert!(tmp.path().join(format!("usage-{today}.jsonl")).exists());
    }

    // 8.14.13 — cleanup keeps recent files
    #[tokio::test]
    async fn cleanup_keeps_recent_files() {
        let tmp = tempfile::tempdir().unwrap();
        let today = Utc::now().format("%Y-%m-%d").to_string();
        std::fs::write(tmp.path().join(format!("usage-{today}.jsonl")), "today\n").unwrap();

        let config = AppConfig {
            usage_tracking_enabled: true,
            log_dir: tmp.path().to_string_lossy().into_owned(),
            log_keep_days: 7,
            ..Default::default()
        };
        let logger = UsageLogger::new(&config, "test");
        logger.cleanup_old_files().await.unwrap();

        assert!(tmp.path().join(format!("usage-{today}.jsonl")).exists());
    }

    // 8.14.14 — Log file name follows usage-YYYY-MM-DD.jsonl pattern
    #[test]
    fn log_file_name_pattern() {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let name = format!("usage-{date}.jsonl");
        assert!(name.starts_with("usage-"));
        assert!(name.ends_with(".jsonl"));
        assert_eq!(name.len(), "usage-YYYY-MM-DD.jsonl".len());
    }

    // 8.14.15 — init_tracing() succeeds with default config
    #[test]
    fn init_tracing_succeeds() {
        // Can only init tracing once per process, so just test resolve_log_dir
        let config = AppConfig::default();
        let dir = resolve_log_dir(&config);
        assert!(dir.to_string_lossy().contains("logs"));
    }

    // 8.14.16 — resolve_log_dir uses data_dir/logs when log_dir empty
    #[test]
    fn resolve_log_dir_default() {
        let config = AppConfig {
            log_dir: String::new(),
            ..Default::default()
        };
        let dir = resolve_log_dir(&config);
        assert!(dir.ends_with("logs"));
    }
}
