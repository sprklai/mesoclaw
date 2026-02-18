use std::sync::Arc;

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
