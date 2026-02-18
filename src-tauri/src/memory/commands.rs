//! Tauri IPC commands for the memory subsystem.

use std::sync::Arc;

use tauri::State;

use crate::memory::{
    store::InMemoryStore,
    traits::{Memory as _, MemoryCategory, MemoryEntry},
};

/// Store a fact in agent memory.
#[tauri::command]
pub async fn store_memory_command(
    key: String,
    content: String,
    category: Option<String>,
    store: State<'_, Arc<InMemoryStore>>,
) -> Result<(), String> {
    let cat = match category.as_deref() {
        Some("daily") => MemoryCategory::Daily,
        Some("conversation") => MemoryCategory::Conversation,
        Some(other) => MemoryCategory::Custom(other.to_string()),
        None => MemoryCategory::Core,
    };
    store.store(&key, &content, cat).await
}

/// Search agent memory by semantic/keyword query.
#[tauri::command]
pub async fn search_memory_command(
    query: String,
    limit: Option<u32>,
    store: State<'_, Arc<InMemoryStore>>,
) -> Result<Vec<MemoryEntry>, String> {
    store.recall(&query, limit.unwrap_or(10) as usize).await
}

/// Remove a memory entry by key.
#[tauri::command]
pub async fn forget_memory_command(
    key: String,
    store: State<'_, Arc<InMemoryStore>>,
) -> Result<bool, String> {
    store.forget(&key).await
}

/// Retrieve the daily memory entry for a given date (`YYYY-MM-DD`).
#[tauri::command]
pub async fn get_daily_memory_command(
    date: String,
    store: State<'_, Arc<InMemoryStore>>,
) -> Result<Option<String>, String> {
    store.recall_daily(&date).await
}
