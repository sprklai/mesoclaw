//! Tauri IPC commands for the Channels settings panel.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::channels::ChannelManager;

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

/// List all registered channels and their connection status.
#[tauri::command]
pub async fn list_channels_command(
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<Vec<ChannelStatusPayload>, String> {
    let names = mgr.channel_names().await;
    let health = mgr.health_all().await;
    Ok(names
        .into_iter()
        .map(|name| {
            let connected = health.get(&name).copied().unwrap_or(false);
            ChannelStatusPayload { name, connected, error: None }
        })
        .collect())
}

/// Test connectivity for the named channel without fully connecting.
///
/// Returns `true` if the channel is registered and healthy, `false` otherwise.
#[tauri::command]
pub async fn test_channel_connection_command(
    name: String,
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<bool, String> {
    let health = mgr.health_all().await;
    Ok(health.get(&name).copied().unwrap_or(false))
}

/// Disconnect the named channel.
#[tauri::command]
pub async fn disconnect_channel_command(
    name: String,
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<(), String> {
    if name == "tauri-ipc" {
        return Err("Desktop IPC channel cannot be disconnected.".to_string());
    }
    mgr.unregister(&name).await;
    log::info!("disconnect_channel_command: '{name}' removed");
    Ok(())
}

/// Attempt to connect the named channel.
///
/// For Telegram: reads the bot token from the OS keyring (saved via `keychain_set`)
/// and registers a new `TelegramChannel` with the `ChannelManager`. If the channel
/// is already registered, returns current health.
///
/// Requires the `channels-telegram` Cargo feature for Telegram support.
#[tauri::command]
pub async fn connect_channel_command(
    name: String,
    _mgr: State<'_, Arc<ChannelManager>>,
) -> Result<ChannelStatusPayload, String> {
    #[cfg(feature = "channels-telegram")]
    if name == "telegram" {
        // Return current status if already registered.
        let health = _mgr.health_all().await;
        if health.contains_key("telegram") {
            return Ok(ChannelStatusPayload {
                name: "telegram".to_string(),
                connected: *health.get("telegram").unwrap_or(&false),
                error: None,
            });
        }
        // Load token from OS keyring.
        let entry = keyring::Entry::new("mesoclaw", "telegram_bot_token")
            .map_err(|e| format!("keyring error: {e}"))?;
        let token = entry.get_password().map_err(|_| {
            "No Telegram bot token saved. Enter your token and click Save first.".to_string()
        })?;
        if token.is_empty() {
            return Err(
                "Telegram bot token is empty. Enter your token and click Save first.".to_string(),
            );
        }
        let config = crate::channels::TelegramConfig::new(token);
        let channel = Arc::new(crate::channels::TelegramChannel::new(config));
        _mgr.register(channel).await?;
        return Ok(ChannelStatusPayload {
            name: "telegram".to_string(),
            connected: true,
            error: None,
        });
    }

    Err(format!(
        "Channel '{name}' is not available in this build. Enable the corresponding Cargo feature."
    ))
}
