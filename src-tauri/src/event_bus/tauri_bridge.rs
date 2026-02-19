use std::sync::Arc;

use tauri::{AppHandle, Emitter, async_runtime};
use tokio::sync::broadcast::error::RecvError;

use super::traits::{AppEvent, EventBus};

/// Forwards UI-relevant [`AppEvent`]s from the [`EventBus`] to the frontend
/// via Tauri's built-in event system (`app_handle.emit`).
pub struct TauriBridge {
    event_bus: Arc<dyn EventBus>,
    app_handle: AppHandle,
}

impl TauriBridge {
    pub fn new(event_bus: Arc<dyn EventBus>, app_handle: AppHandle) -> Self {
        Self {
            event_bus,
            app_handle,
        }
    }

    /// Spawn a background task that listens for events and forwards them.
    ///
    /// The task exits cleanly when the broadcast channel is closed (i.e. the
    /// bus is dropped).
    pub fn start(self) {
        let mut receiver = self.event_bus.subscribe();
        let app_handle = self.app_handle;

        async_runtime::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        if is_ui_relevant(&event)
                            && let Err(e) = app_handle.emit("app-event", &event)
                        {
                            log::warn!("TauriBridge: failed to emit event: {e}");
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        log::warn!("TauriBridge: lagged, missed {n} events");
                    }
                    Err(RecvError::Closed) => {
                        break;
                    }
                }
            }
        });
    }
}

/// Returns true for events that the frontend UI needs to react to.
fn is_ui_relevant(event: &AppEvent) -> bool {
    matches!(
        event,
        AppEvent::AgentToolStart { .. }
            | AppEvent::AgentToolResult { .. }
            | AppEvent::AgentStarted { .. }
            | AppEvent::AgentComplete { .. }
            | AppEvent::ApprovalNeeded { .. }
            | AppEvent::HeartbeatTick { .. }
            | AppEvent::ChannelMessage { .. }
            | AppEvent::SystemReady
            | AppEvent::SystemError { .. }
            | AppEvent::ProviderHealthChange { .. }
    )
}
