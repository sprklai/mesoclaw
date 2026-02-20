use serde::Serialize;

/// A parsed log entry returned to the frontend.
#[derive(Debug, Serialize, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// Resolve the log directory (same logic as `plugins::logging`).
fn resolve_log_dir() -> std::path::PathBuf {
    #[cfg(target_os = "macos")]
    let base = dirs::home_dir().map(|h| h.join("Library").join("Logs"));

    #[cfg(not(target_os = "macos"))]
    let base = dirs::data_local_dir();

    base.map(|d| d.join("com.sprklai.mesoclaw"))
        .unwrap_or_else(std::env::temp_dir)
}

/// Parse a single log line produced by `tracing_subscriber::fmt`.
///
/// Expected format (no ANSI colours):
/// ```
/// 2025-01-01T12:00:00.123456Z  INFO crate::module: some message
/// ```
fn parse_line(line: &str) -> Option<LogEntry> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Split off the timestamp token (ends at the first whitespace run).
    let mut parts = line.splitn(2, "  ");
    let timestamp = parts.next()?.trim().to_string();
    let rest = parts.next()?.trim();

    // Next token is the log level.
    let mut rest_parts = rest.splitn(2, ' ');
    let level = rest_parts.next()?.trim().to_string();
    let rest2 = rest_parts.next().unwrap_or("").trim();

    // Optional target before the colon; fall back to empty string.
    let (target, message) = if let Some(colon) = rest2.find(": ") {
        (
            rest2[..colon].trim().to_string(),
            rest2[colon + 2..].trim().to_string(),
        )
    } else {
        (String::new(), rest2.to_string())
    };

    Some(LogEntry {
        timestamp,
        level,
        target,
        message,
    })
}

/// Return log entries from recent log files (today + yesterday), newest first.
///
/// `max_lines` caps how many raw lines are scanned (default 5 000).
#[tauri::command]
pub fn get_logs_command(max_lines: Option<usize>) -> Result<Vec<LogEntry>, String> {
    let limit = max_lines.unwrap_or(5_000);
    let log_dir = resolve_log_dir();

    // Collect all `mesoclaw.log.*` files, sorted by name descending (newest date first).
    let mut log_files: Vec<std::path::PathBuf> = std::fs::read_dir(&log_dir)
        .map_err(|e| format!("cannot read log dir {}: {e}", log_dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("mesoclaw.log"))
                .unwrap_or(false)
        })
        .collect();

    log_files.sort_by(|a, b| b.cmp(a)); // newest first

    let mut entries: Vec<LogEntry> = Vec::new();
    let mut lines_scanned = 0usize;

    'files: for path in &log_files {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;

        // Collect lines from this file (reversed so we get newest first).
        let file_lines: Vec<&str> = content.lines().collect();
        for line in file_lines.iter().rev() {
            if let Some(entry) = parse_line(line) {
                entries.push(entry);
            }
            lines_scanned += 1;
            if lines_scanned >= limit {
                break 'files;
            }
        }
    }

    Ok(entries)
}
