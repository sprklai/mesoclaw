//! Tauri IPC commands for the Channels settings panel.
//!
//! ## TODO (7.2): These are stubs. Real connect/disconnect/test logic for
//! Telegram and other channels will be wired in Phase 7 follow-up work.

use serde::Serialize;

/// Status payload returned to the frontend after a connect or health-check.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStatusPayload {
    /// Channel name (e.g. `"telegram"`, `"tauri-ipc"`).
    pub name: String,
    /// Whether the channel is currently connected/healthy.
    pub connected: bool,
    /// Optional human-readable error message.
    pub error: Option<String>,
}

/// Attempt to connect the named channel.
///
/// ## TODO (7.2): Wire real Telegram connect logic once the `channels-telegram`
/// feature is enabled and the ChannelManager is exposed via Tauri state.
#[tauri::command]
pub async fn connect_channel_command(name: String) -> Result<ChannelStatusPayload, String> {
    // ## MOCK: returns an error indicating the channel is not yet implemented.
    Err(format!(
        "Channel '{name}' connect is not yet implemented. Configure the bot token and try again."
    ))
}

/// Disconnect the named channel.
///
/// ## TODO (7.2): Wire real disconnect logic.
#[tauri::command]
pub async fn disconnect_channel_command(name: String) -> Result<(), String> {
    // ## MOCK: no-op stub — channel manager not yet wired to Tauri state.
    log::info!("disconnect_channel_command called for '{name}' (stub)");
    Ok(())
}

/// Test connectivity for the named channel without fully connecting.
///
/// Returns `true` if the health check passes, `false` otherwise.
///
/// ## TODO (7.2): Wire real health-check logic.
#[tauri::command]
pub async fn test_channel_connection_command(name: String) -> Result<bool, String> {
    // ## MOCK: always returns false until real implementation is wired.
    log::info!("test_channel_connection_command called for '{name}' (stub)");
    Ok(false)
}

/// List all registered channels and their connection status.
///
/// ## TODO (7.2): Return real status from ChannelManager state.
#[tauri::command]
pub async fn list_channels_command() -> Result<Vec<ChannelStatusPayload>, String> {
    // ## MOCK: return the two default channels in a hardcoded disconnected state.
    Ok(vec![
        ChannelStatusPayload {
            name: "tauri-ipc".to_string(),
            connected: true,
            error: None,
        },
        ChannelStatusPayload {
            name: "telegram".to_string(),
            connected: false,
            error: None,
        },
    ])
}
