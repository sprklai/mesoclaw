//! Tauri IPC commands for the memory subsystem.
//!
//! These commands expose the [`Memory`] trait to the frontend via Tauri's
//! invoke mechanism.
//!
//! ## TODO (Phase 3 follow-up)
//! Wire up memory commands to a real managed [`InMemoryStore`] instance.
//! Currently stubs that return an error explaining the status.

/// Store a fact in agent memory.
#[tauri::command]
pub async fn store_memory_command(
    _key: String,
    _content: String,
    _category: Option<String>,
) -> Result<(), String> {
    // ## TODO: Resolve managed InMemoryStore from app state
    Err("Memory commands not yet wired to app state. Planned for Phase 3 follow-up.".to_string())
}

/// Search agent memory by semantic query.
#[tauri::command]
pub async fn search_memory_command(
    _query: String,
    _limit: Option<u32>,
) -> Result<Vec<serde_json::Value>, String> {
    // ## TODO: Resolve managed InMemoryStore from app state
    Err("Memory commands not yet wired to app state. Planned for Phase 3 follow-up.".to_string())
}

/// Remove a memory entry by key.
#[tauri::command]
pub async fn forget_memory_command(_key: String) -> Result<bool, String> {
    // ## TODO: Resolve managed InMemoryStore from app state
    Err("Memory commands not yet wired to app state. Planned for Phase 3 follow-up.".to_string())
}

/// Retrieve the daily memory entry for a given date (`YYYY-MM-DD`).
#[tauri::command]
pub async fn get_daily_memory_command(_date: String) -> Result<Option<String>, String> {
    // ## TODO: Resolve managed InMemoryStore from app state
    Err("Memory commands not yet wired to app state. Planned for Phase 3 follow-up.".to_string())
}
