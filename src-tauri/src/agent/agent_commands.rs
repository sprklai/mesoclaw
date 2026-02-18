//! Tauri IPC commands for agent session management.
//!
//! These commands expose the [`AgentLoop`] to the frontend via the Tauri
//! invoke mechanism.  Each command starts or cancels an agent session.
//!
//! # Integration notes
//! - The LLM provider is resolved from the database configuration on each call.
//! - Tool registry and security policy come from managed app state.
//! - Streaming intermediate results are emitted as Tauri events so the
//!   frontend can update incrementally.
//!
//! ## TODO (Phase 3 follow-up)
//! - Wire up a real LLM provider from app state instead of erroring.
//! - Stream intermediate tool results to the frontend.
//! - Implement session cancellation via a shared `CancellationToken`.

// These commands are stubbed pending full LLM provider state management.
// They compile cleanly and will be wired up in a follow-up.

/// Start an agent session for a user message.
///
/// Returns the agent's final response as a string.
///
/// ## TODO: Implement full agent session wiring
/// - Resolve LLM provider from database config
/// - Get identity system prompt from IdentityLoader
/// - Wire AgentLoop to app state (tool registry, security policy, event bus)
#[tauri::command]
pub async fn start_agent_session_command(
    _message: String,
) -> Result<String, String> {
    // ## TODO: implement proper provider resolution from app state
    Err("Agent sessions not yet fully wired. Planned for Phase 3.".to_string())
}

/// Cancel a running agent session.
///
/// ## TODO: Implement cancellation via CancellationToken
#[tauri::command]
pub async fn cancel_agent_session_command(
    _session_id: String,
) -> Result<(), String> {
    // ## TODO: implement session cancellation
    Err("Session cancellation not yet implemented.".to_string())
}
