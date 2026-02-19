//! Tauri IPC commands for the Channels settings panel.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use crate::channels::ChannelManager;

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
#[tauri::command]
pub async fn test_channel_connection_command(
    name: String,
    mgr: State<'_, Arc<ChannelManager>>,
) -> Result<bool, String> {
    let health = mgr.health_all().await;
    health
        .get(&name)
        .copied()
        .ok_or_else(|| format!("Channel '{name}' not found"))
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
