//! Memory subsystem for the MesoClaw agent.
//!
//! # Architecture
//! ```text
//! Memory (trait)
//!   └── InMemoryStore          ← HashMap-backed, thread-safe
//!         ├── EmbeddingProvider (trait)
//!         │     └── MockEmbeddingProvider  (deterministic hash-based)
//!         │     └── LruEmbeddingCache      (caching wrapper)
//!         └── keyword_score  (BM25-style term-frequency scoring)
//!
//! Hybrid recall score = 0.7 * cosine_similarity + 0.3 * keyword_score
//! ```
//!
//! # Agent tools
//! - [`tools::MemoryStoreTool`]  — `memory_store`
//! - [`tools::MemoryRecallTool`] — `memory_recall`
//! - [`tools::MemoryForgetTool`] — `memory_forget`
//!
//! # IPC commands
//! - [`commands::store_memory_command`]
//! - [`commands::search_memory_command`]
//! - [`commands::forget_memory_command`]
//! - [`commands::get_daily_memory_command`]

pub mod chunker;
pub mod commands;
pub mod daily;
pub mod embeddings;
pub mod store;
pub mod tools;
pub mod traits;

use std::sync::Arc;

use crate::tools::ToolRegistry;

pub use chunker::{Chunk, ChunkConfig, split_into_chunks};
pub use commands::{
    forget_memory_command, get_daily_memory_command, search_memory_command, store_memory_command,
};
pub use daily::DailyMemory;
pub use embeddings::{
    EmbeddingProvider, LruEmbeddingCache, MockEmbeddingProvider, cosine_similarity,
};
pub use store::InMemoryStore;
pub use tools::{MemoryForgetTool, MemoryRecallTool, MemoryStoreTool};
pub use traits::{Memory, MemoryCategory, MemoryEntry};

/// Register the three memory agent tools into `registry`.
///
/// Typically called from `lib.rs` after managing an `Arc<InMemoryStore>`.
pub fn register_memory_tools(registry: &mut ToolRegistry, memory: Arc<dyn Memory>) {
    registry.register(Arc::new(MemoryStoreTool::new(memory.clone())));
    registry.register(Arc::new(MemoryRecallTool::new(memory.clone())));
    registry.register(Arc::new(MemoryForgetTool::new(memory)));
}
