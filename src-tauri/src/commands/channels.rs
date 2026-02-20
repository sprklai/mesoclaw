//! Tauri IPC commands for the Channels settings panel.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::channels::{Channel, ChannelManager};

// ─── ChannelStatusPayload ─────────────────────────────────────────────────────

/// Status payload returned to the frontend.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStatusPayload {
    /// Channel name (e.g. `"telegram"`, `"tauri-ipc"`).
    pub name: String,
    /// Whether the channel is currently healthy.
    pub connected: bool,
    /// Optional human-readable error message.
    pub error: Option<String>,
}

// ─── Commands ─────────────────────────────────────────────────────────────────

/// Read credentials for `name` from the OS keyring, create the channel
/// instance, and register it in the manager so it starts receiving messages.
///
/// If the channel is already registered it is first unregistered (allowing a
/// re-connect with fresh credentials).  Returns an error if no credentials are
/// saved for the requested channel.
#[tauri::command]
pub async fn start_channel_command(
    name: String,
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<ChannelStatusPayload, String> {
    use crate::config::app_identity::KEYCHAIN_SERVICE;

    // Unregister the existing instance so we can re-register fresh.
    mgr.unregister(&name).await;

    #[cfg(feature = "channels-discord")]
    if name == "discord" {
        let token = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:discord:token")
            .map_err(|e| e.to_string())?
            .get_password()
            .map_err(|_| "discord: no bot token saved — configure and save first".to_string())?;
        if token.is_empty() {
            return Err("discord: bot token is empty — save your credentials first".to_string());
        }
        let allowed_guild_ids: Vec<u64> =
            keyring::Entry::new(KEYCHAIN_SERVICE, "channel:discord:allowed_guild_ids")
                .ok()
                .and_then(|e| e.get_password().ok())
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
        let allowed_channel_ids: Vec<u64> =
            keyring::Entry::new(KEYCHAIN_SERVICE, "channel:discord:allowed_channel_ids")
                .ok()
                .and_then(|e| e.get_password().ok())
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
        let config = crate::channels::DiscordConfig::with_allowlists(
            token,
            allowed_guild_ids,
            allowed_channel_ids,
        );
        let ch = Arc::new(crate::channels::DiscordChannel::new(config));
        let healthy = ch.health_check().await;
        mgr.register(ch).await?;
        return Ok(ChannelStatusPayload {
            name,
            connected: healthy,
            error: None,
        });
    }

    #[cfg(feature = "channels-slack")]
    if name == "slack" {
        let bot_token = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:slack:bot_token")
            .map_err(|e| e.to_string())?
            .get_password()
            .map_err(|_| "slack: no bot token saved — configure and save first".to_string())?;
        if bot_token.is_empty() {
            return Err("slack: bot token is empty — save your credentials first".to_string());
        }
        let app_token = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:slack:app_token")
            .ok()
            .and_then(|e| e.get_password().ok())
            .unwrap_or_default();
        let allowed_channel_ids: Vec<String> =
            keyring::Entry::new(KEYCHAIN_SERVICE, "channel:slack:allowed_channel_ids")
                .ok()
                .and_then(|e| e.get_password().ok())
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
        let config = crate::channels::SlackConfig::with_allowed_channels(
            bot_token,
            app_token,
            allowed_channel_ids,
        );
        let ch = Arc::new(crate::channels::SlackChannel::new(config));
        let healthy = ch.health_check().await;
        mgr.register(ch).await?;
        return Ok(ChannelStatusPayload {
            name,
            connected: healthy,
            error: None,
        });
    }

    #[cfg(feature = "channels-matrix")]
    if name == "matrix" {
        let access_token = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:matrix:access_token")
            .map_err(|e| e.to_string())?
            .get_password()
            .map_err(|_| "matrix: no access token saved — configure and save first".to_string())?;
        if access_token.is_empty() {
            return Err("matrix: access token is empty — save your credentials first".to_string());
        }
        let homeserver = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:matrix:homeserver_url")
            .ok()
            .and_then(|e| e.get_password().ok())
            .unwrap_or_default();
        let username = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:matrix:username")
            .ok()
            .and_then(|e| e.get_password().ok())
            .unwrap_or_default();
        let allowed_room_ids: Vec<String> =
            keyring::Entry::new(KEYCHAIN_SERVICE, "channel:matrix:allowed_room_ids")
                .ok()
                .and_then(|e| e.get_password().ok())
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
        let config = crate::channels::MatrixConfig::with_allowed_rooms(
            homeserver,
            username,
            access_token,
            allowed_room_ids,
        );
        let ch = Arc::new(crate::channels::MatrixChannel::new(config));
        let healthy = ch.health_check().await;
        mgr.register(ch).await?;
        return Ok(ChannelStatusPayload {
            name,
            connected: healthy,
            error: None,
        });
    }

    #[cfg(feature = "channels-telegram")]
    if name == "telegram" {
        let token = keyring::Entry::new(KEYCHAIN_SERVICE, "channel:telegram:token")
            .map_err(|e| e.to_string())?
            .get_password()
            .map_err(|_| "telegram: no bot token saved — configure and save first".to_string())?;
        if token.is_empty() {
            return Err("telegram: bot token is empty — save your credentials first".to_string());
        }
        let allowed_ids: Vec<i64> =
            keyring::Entry::new(KEYCHAIN_SERVICE, "channel:telegram:allowed_chat_ids")
                .ok()
                .and_then(|e| e.get_password().ok())
                .unwrap_or_default()
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
        let timeout: u32 =
            keyring::Entry::new(KEYCHAIN_SERVICE, "channel:telegram:polling_timeout_secs")
                .ok()
                .and_then(|e| e.get_password().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(30);
        let mut config = crate::channels::TelegramConfig::with_allowed_ids(token, allowed_ids);
        config.polling_timeout_secs = timeout;
        let ch = Arc::new(crate::channels::TelegramChannel::new(config));
        let healthy = ch.health_check().await;
        mgr.register(ch).await?;
        return Ok(ChannelStatusPayload {
            name,
            connected: healthy,
            error: None,
        });
    }

    Err(format!("Channel '{name}' is not supported"))
}

/// Return the current health status for the named channel.
///
/// This is a health/probe command, not a full connect flow.  For a full
/// Telegram connect flow (load token → register → start listener),
/// see Phase 7.1.6 (token management UI).
#[tauri::command]
pub async fn channel_health_command(
    name: String,
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<ChannelStatusPayload, String> {
    let health = mgr.health_all().await;
    let connected = health
        .get(&name)
        .copied()
        .ok_or_else(|| format!("Channel '{name}' not found"))?;
    Ok(ChannelStatusPayload {
        name,
        connected,
        error: None,
    })
}

/// Unregister the named channel from the channel manager.
#[tauri::command]
pub async fn disconnect_channel_command(
    name: String,
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<(), String> {
    if mgr.unregister(&name).await {
        Ok(())
    } else {
        Err(format!("Channel '{name}' not found"))
    }
}

/// Return `true` if the named channel's health check passes.
///
/// If the channel is already registered in the manager its `health_check()` is
/// used.  Otherwise an ad-hoc probe is performed using the supplied credentials
/// (`token` + optional `extra`) so the UI can verify settings before the
/// channel has ever been started.
///
/// | Channel  | `token`       | `extra`            |
/// |----------|---------------|--------------------|
/// | discord  | bot token     | —                  |
/// | matrix   | access token  | homeserver URL     |
/// | slack    | bot token     | —                  |
/// | telegram | bot token     | —                  |
#[tauri::command]
pub async fn test_channel_connection_command(
    name: String,
    token: Option<String>,
    extra: Option<String>,
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<bool, String> {
    // Fast path: channel is already running in the manager.
    let health = mgr.health_all().await;
    if let Some(&healthy) = health.get(&name) {
        return Ok(healthy);
    }

    // Slow path: ad-hoc probe from supplied credentials.
    // Suppress unused-variable warnings when no channel features are compiled.
    let _ = (&token, &extra);

    #[cfg(feature = "channels-discord")]
    if name == "discord" {
        let tok = token
            .filter(|t| !t.is_empty())
            .ok_or("discord probe: bot token is required")?;
        let ch = crate::channels::DiscordChannel::new(crate::channels::DiscordConfig::new(tok));
        return Ok(ch.health_check().await);
    }

    #[cfg(feature = "channels-matrix")]
    if name == "matrix" {
        let access_token = token
            .filter(|t| !t.is_empty())
            .ok_or("matrix probe: access token is required")?;
        let homeserver = extra
            .filter(|u| !u.is_empty())
            .ok_or("matrix probe: homeserver URL is required")?;
        let ch = crate::channels::MatrixChannel::new(crate::channels::MatrixConfig::new(
            homeserver,
            "",
            access_token,
        ));
        return Ok(ch.health_check().await);
    }

    #[cfg(feature = "channels-slack")]
    if name == "slack" {
        let tok = token
            .filter(|t| !t.is_empty())
            .ok_or("slack probe: bot token is required")?;
        let ch = crate::channels::SlackChannel::new(crate::channels::SlackConfig::new(tok, ""));
        return Ok(ch.health_check().await);
    }

    #[cfg(feature = "channels-telegram")]
    if name == "telegram" {
        let tok = token
            .filter(|t| !t.is_empty())
            .ok_or("telegram probe: bot token is required")?;
        let ch = crate::channels::TelegramChannel::new(crate::channels::TelegramConfig::new(tok));
        return Ok(ch.health_check().await);
    }

    Err(format!(
        "Channel '{name}' is not registered and no ad-hoc probe is available for it"
    ))
}

/// List all registered channels with their connection status.
#[tauri::command]
pub async fn list_channels_command(
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<Vec<ChannelStatusPayload>, String> {
    let health = mgr.health_all().await;
    let mut names = mgr.channel_names().await;
    names.sort(); // deterministic order for UI
    Ok(names
        .into_iter()
        .map(|name| {
            let connected = health.get(&name).copied().unwrap_or(false);
            ChannelStatusPayload {
                name,
                connected,
                error: None,
            }
        })
        .collect())
}

/// Send a message through a named channel to a specific recipient.
///
/// `channel` — registered channel name (e.g. `"telegram"`).
/// `recipient` — channel-specific address; for Telegram this is the chat ID as a string.
///              Pass `None` to broadcast to all peers (channel-dependent).
#[tauri::command]
pub async fn send_channel_message_command(
    channel: String,
    message: String,
    recipient: Option<String>,
    channel_manager: State<'_, Arc<ChannelManager>>,
) -> Result<(), String> {
    channel_manager
        .send(&channel, &message, recipient.as_deref())
        .await
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channels::{Channel, ChannelManager, ChannelMessage};
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    struct FakeChannel {
        id: String,
        healthy: bool,
    }

    #[async_trait]
    impl Channel for FakeChannel {
        fn name(&self) -> &str {
            &self.id
        }
        async fn send(&self, _: &str, _: Option<&str>) -> Result<(), String> {
            Ok(())
        }
        async fn listen(&self, _: mpsc::Sender<ChannelMessage>) -> Result<(), String> {
            Ok(())
        }
        async fn health_check(&self) -> bool {
            self.healthy
        }
    }

    fn healthy(name: &str) -> Arc<dyn Channel> {
        Arc::new(FakeChannel {
            id: name.to_string(),
            healthy: true,
        })
    }

    fn unhealthy(name: &str) -> Arc<dyn Channel> {
        Arc::new(FakeChannel {
            id: name.to_string(),
            healthy: false,
        })
    }

    #[tokio::test]
    async fn list_channels_returns_all_with_health() {
        let mgr = Arc::new(ChannelManager::new());
        mgr.register(healthy("tauri-ipc")).await.unwrap();
        mgr.register(unhealthy("telegram")).await.unwrap();

        let health = mgr.health_all().await;
        let mut names = mgr.channel_names().await;
        names.sort();
        let result: Vec<ChannelStatusPayload> = names
            .into_iter()
            .map(|name| {
                let connected = health.get(&name).copied().unwrap_or(false);
                ChannelStatusPayload {
                    name,
                    connected,
                    error: None,
                }
            })
            .collect();

        assert_eq!(result.len(), 2);
        let ipc = result.iter().find(|p| p.name == "tauri-ipc").unwrap();
        assert!(ipc.connected);
        let tg = result.iter().find(|p| p.name == "telegram").unwrap();
        assert!(!tg.connected);
    }

    #[tokio::test]
    async fn health_check_returns_true_for_healthy_channel() {
        let mgr = Arc::new(ChannelManager::new());
        mgr.register(healthy("tauri-ipc")).await.unwrap();
        let health = mgr.health_all().await;
        let result = health
            .get("tauri-ipc")
            .copied()
            .ok_or_else(|| "Channel 'tauri-ipc' not found".to_string());
        assert_eq!(result, Ok(true));
    }

    #[tokio::test]
    async fn health_check_returns_err_for_unknown_channel() {
        let mgr = Arc::new(ChannelManager::new());
        let health = mgr.health_all().await;
        let result: Result<bool, String> = health
            .get("ghost")
            .copied()
            .ok_or_else(|| "Channel 'ghost' not found".to_string());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn disconnect_unregisters_channel() {
        let mgr = Arc::new(ChannelManager::new());
        mgr.register(healthy("tauri-ipc")).await.unwrap();
        assert_eq!(mgr.len().await, 1);
        assert!(mgr.unregister("tauri-ipc").await);
        assert!(mgr.is_empty().await);
    }

    #[tokio::test]
    async fn disconnect_returns_false_for_unknown_channel() {
        let mgr = Arc::new(ChannelManager::new());
        assert!(!mgr.unregister("ghost").await);
    }
}

#[cfg(test)]
mod send_cmd_tests {
    // Compile-time check: function exists with correct signature.
    // Real integration test requires a live ChannelManager + registered channel.
    use super::*;
    #[test]
    fn send_channel_message_command_compiles() {
        // fn(String, String, Option<String>, State<Arc<ChannelManager>>) -> impl Future
        // Verified by the compiler — if this test file compiles, the command exists.
        let _ = send_channel_message_command as fn(_, _, _, _) -> _;
    }
}
