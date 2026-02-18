use std::{path::PathBuf, sync::Arc};

use serde::Serialize;
use tauri::State;

use crate::event_bus::{AppEvent, EventBus};

/// Called by the frontend approval overlay when the user accepts or rejects a
/// pending high-risk action.
///
/// The response is published to the [`EventBus`] as an [`AppEvent::ApprovalResponse`]
/// so any subscriber (e.g. the agent loop) can resume or abort accordingly.
#[tauri::command]
pub async fn approve_action_command(
    action_id: String,
    approved: bool,
    event_bus: State<'_, Arc<dyn EventBus>>,
) -> Result<(), String> {
    event_bus
        .publish(AppEvent::ApprovalResponse { action_id, approved })
        .map_err(|e| format!("failed to publish approval: {e}"))
}

// ─── Daemon config ────────────────────────────────────────────────────────────

/// The port and bearer token needed to connect to the local gateway daemon.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaemonConfig {
    pub port: u16,
    pub token: String,
}

/// Return the port and bearer token for the running gateway daemon.
///
/// Reads `~/.mesoclaw/daemon.pid` (line 2 = port) and
/// `~/.mesoclaw/daemon.token`.  Returns an error if the daemon is not running
/// or the files are missing.
#[tauri::command]
pub fn get_daemon_config_command() -> Result<DaemonConfig, String> {
    let mesoclaw_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".mesoclaw");
    let pid_path = mesoclaw_dir.join("daemon.pid");
    let token_path = mesoclaw_dir.join("daemon.token");

    let pid_content = std::fs::read_to_string(&pid_path)
        .map_err(|_| "Daemon not running (daemon.pid not found)".to_string())?;

    // daemon.pid format: line 1 = PID, line 2 = port
    let port: u16 = pid_content
        .lines()
        .nth(1)
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(18790);

    let token = std::fs::read_to_string(&token_path)
        .map_err(|_| "Daemon token not found (daemon.token missing)".to_string())?
        .trim()
        .to_string();

    Ok(DaemonConfig { port, token })
}
