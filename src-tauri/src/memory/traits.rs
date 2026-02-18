//! Core types and the `Memory` trait for the memory subsystem.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ─── MemoryCategory ───────────────────────────────────────────────────────────

/// Semantic classification of a memory entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryCategory {
    /// Core facts about the agent's persona or the user.
    Core,
    /// Daily diary entries.
    Daily,
    /// Snippets extracted from conversation history.
    Conversation,
    /// User-defined custom category.
    Custom(String),
}

impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryCategory::Core => write!(f, "core"),
            MemoryCategory::Daily => write!(f, "daily"),
            MemoryCategory::Conversation => write!(f, "conversation"),
            MemoryCategory::Custom(s) => write!(f, "custom:{s}"),
        }
    }
}

// ─── MemoryEntry ──────────────────────────────────────────────────────────────

/// A single memory record returned by [`Memory::recall()`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier.
    pub id: String,
    /// Namespaced lookup key (e.g. `"user:name"`, `"project:goal"`).
    pub key: String,
    /// The text content of this memory.
    pub content: String,
    /// Semantic category.
    pub category: MemoryCategory,
    /// Relevance score in [0, 1] — higher is more relevant.
    pub score: f32,
    /// ISO 8601 creation timestamp.
    pub created_at: String,
    /// ISO 8601 last-updated timestamp.
    pub updated_at: String,
}

// ─── Memory trait ─────────────────────────────────────────────────────────────

/// Abstraction over the agent's memory store.
///
/// Implementations may back the store with an in-process `HashMap`, an SQLite
/// database, or a remote vector database.  All retrieval operations must return
/// results sorted by descending `score`.
#[async_trait]
pub trait Memory: Send + Sync {
    /// Store or overwrite a memory entry identified by `key`.
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
    ) -> Result<(), String>;

    /// Search for entries that match `query`.
    ///
    /// Returns at most `limit` entries sorted by descending relevance.
    async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>, String>;

    /// Remove an entry by `key`.  Returns `true` if it was found and removed.
    async fn forget(&self, key: &str) -> Result<bool, String>;

    /// Append an entry to today's daily diary (`MemoryCategory::Daily`).
    async fn store_daily(&self, content: &str) -> Result<(), String>;

    /// Retrieve the diary content for `date` (format `YYYY-MM-DD`).
    /// Returns `None` if no entry exists for that date.
    async fn recall_daily(&self, date: &str) -> Result<Option<String>, String>;
}
