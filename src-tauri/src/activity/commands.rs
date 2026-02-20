//! Tauri IPC commands for activity tracking.

use std::sync::Arc;

use tauri::State;

use super::ActivityBuffer;

/// Default time window: 1 hour in milliseconds.
const DEFAULT_WITHIN_MS: u64 = 3_600_000;

/// Get recent activities within a time window.
///
/// `within_ms` is optional; defaults to 1 hour (3,600,000 ms).
#[tauri::command]
#[tracing::instrument(name = "command.activity.get_recent", skip(buffer), fields(within_ms = %within_ms.unwrap_or(DEFAULT_WITHIN_MS)))]
pub async fn get_recent_activity_command(
    within_ms: Option<u64>,
    buffer: State<'_, Arc<ActivityBuffer>>,
) -> Result<Vec<super::Activity>, String> {
    let window = within_ms.unwrap_or(DEFAULT_WITHIN_MS);
    Ok(buffer.get_recent(window).await)
}
