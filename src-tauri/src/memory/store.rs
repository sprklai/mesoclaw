//! In-memory implementation of the [`Memory`] trait.
//!
//! [`InMemoryStore`] keeps all entries in a `RwLock<HashMap>` and performs
//! hybrid retrieval: 70 % vector similarity + 30 % keyword frequency.
//!
//! This implementation is suitable for development, testing, and lightweight
//! production use.  For large-scale persistent storage, a future
//! `SqliteMemory` implementation can replace this behind the same trait.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use super::{
    embeddings::{EmbeddingProvider, MockEmbeddingProvider, cosine_similarity},
    traits::{Memory, MemoryCategory, MemoryEntry},
};

// ─── InternalEntry ───────────────────────────────────────────────────────────

/// Storage unit: a [`MemoryEntry`] plus its pre-computed embedding.
struct InternalEntry {
    entry: MemoryEntry,
    embedding: Vec<f32>,
}

// ─── InMemoryStore ───────────────────────────────────────────────────────────

/// HashMap-backed, thread-safe memory store.
pub struct InMemoryStore {
    entries: RwLock<HashMap<String, InternalEntry>>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
}

impl InMemoryStore {
    /// Create a store backed by the given embedding provider.
    pub fn new(embedding_provider: Arc<dyn EmbeddingProvider>) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            embedding_provider,
        }
    }

    /// Convenience constructor using the [`MockEmbeddingProvider`] (no external API calls).
    pub fn new_mock() -> Self {
        Self::new(Arc::new(MockEmbeddingProvider::new()))
    }
}

// ─── Memory implementation ────────────────────────────────────────────────────

#[async_trait]
impl Memory for InMemoryStore {
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
    ) -> Result<(), String> {
        let embedding = self.embedding_provider.embed(content).await?;
        let now = Utc::now().to_rfc3339();

        let mut entries = self.entries.write().map_err(|e| e.to_string())?;

        let entry = if let Some(existing) = entries.get(key) {
            // Overwrite — preserve id and created_at.
            MemoryEntry {
                id: existing.entry.id.clone(),
                key: key.to_owned(),
                content: content.to_owned(),
                category,
                score: 0.0,
                created_at: existing.entry.created_at.clone(),
                updated_at: now,
            }
        } else {
            MemoryEntry {
                id: Uuid::new_v4().to_string(),
                key: key.to_owned(),
                content: content.to_owned(),
                category,
                score: 0.0,
                created_at: now.clone(),
                updated_at: now,
            }
        };

        entries.insert(
            key.to_owned(),
            InternalEntry { entry, embedding },
        );
        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>, String> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let query_embedding = self.embedding_provider.embed(query).await?;
        let query_words: Vec<String> = query
            .split_whitespace()
            .map(|w| w.to_lowercase())
            .collect();

        let entries = self.entries.read().map_err(|e| e.to_string())?;

        let mut scored: Vec<MemoryEntry> = entries
            .values()
            .map(|ie| {
                let vector_score = cosine_similarity(&query_embedding, &ie.embedding);
                let bm25_score = keyword_score(query_words.as_slice(), &ie.entry.content);
                let final_score = 0.7 * vector_score + 0.3 * bm25_score;
                let mut e = ie.entry.clone();
                e.score = final_score;
                e
            })
            .collect();

        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        Ok(scored)
    }

    async fn forget(&self, key: &str) -> Result<bool, String> {
        let mut entries = self.entries.write().map_err(|e| e.to_string())?;
        Ok(entries.remove(key).is_some())
    }

    async fn store_daily(&self, content: &str) -> Result<(), String> {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let key = format!("daily:{date}");

        let existing = {
            let entries = self.entries.read().map_err(|e| e.to_string())?;
            entries.get(&key).map(|ie| ie.entry.content.clone())
        };

        // Append to existing daily entry (separated by newlines).
        let full_content = match existing {
            Some(prev) if !prev.is_empty() => format!("{prev}\n\n{content}"),
            _ => content.to_owned(),
        };

        self.store(&key, &full_content, MemoryCategory::Daily).await
    }

    async fn recall_daily(&self, date: &str) -> Result<Option<String>, String> {
        let key = format!("daily:{date}");
        let entries = self.entries.read().map_err(|e| e.to_string())?;
        Ok(entries.get(&key).map(|ie| ie.entry.content.clone()))
    }
}

// ─── keyword_score ───────────────────────────────────────────────────────────

/// Simple normalised term-frequency score: fraction of query words that appear
/// in `content` (case-insensitive).  Returns a value in `[0, 1]`.
fn keyword_score(query_words: &[String], content: &str) -> f32 {
    if query_words.is_empty() {
        return 0.0;
    }
    let content_lower = content.to_lowercase();
    let matches = query_words
        .iter()
        .filter(|w| content_lower.contains(w.as_str()))
        .count();
    matches as f32 / query_words.len() as f32
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::traits::MemoryCategory;

    fn make_store() -> InMemoryStore {
        InMemoryStore::new_mock()
    }

    #[tokio::test]
    async fn store_and_recall_round_trip() {
        let store = make_store();
        store
            .store("user:name", "Alice", MemoryCategory::Core)
            .await
            .unwrap();

        let results = store.recall("Alice", 5).await.unwrap();
        assert!(!results.is_empty(), "stored entry should be recalled");
        assert_eq!(results[0].key, "user:name");
        assert_eq!(results[0].content, "Alice");
    }

    #[tokio::test]
    async fn recall_empty_store_returns_empty() {
        let store = make_store();
        let results = store.recall("anything", 5).await.unwrap();
        assert!(results.is_empty(), "empty store → empty results");
    }

    #[tokio::test]
    async fn forget_existing_key_returns_true() {
        let store = make_store();
        store
            .store("k", "v", MemoryCategory::Conversation)
            .await
            .unwrap();
        let found = store.forget("k").await.unwrap();
        assert!(found, "forget existing key → true");

        // Key is gone.
        let results = store.recall("v", 1).await.unwrap();
        let keys: Vec<&str> = results.iter().map(|e| e.key.as_str()).collect();
        assert!(!keys.contains(&"k"), "forgotten key should not appear in recall");
    }

    #[tokio::test]
    async fn forget_nonexistent_key_returns_false() {
        let store = make_store();
        let found = store.forget("does_not_exist").await.unwrap();
        assert!(!found, "forget missing key → false");
    }

    #[tokio::test]
    async fn store_daily_and_recall_daily_round_trip() {
        let store = make_store();
        store.store_daily("Today I worked on the memory system.").await.unwrap();

        let date = Utc::now().format("%Y-%m-%d").to_string();
        let content = store.recall_daily(&date).await.unwrap();
        assert!(content.is_some(), "daily entry should exist for today");
        assert!(content.unwrap().contains("memory system"));
    }

    #[tokio::test]
    async fn recall_daily_nonexistent_date_returns_none() {
        let store = make_store();
        let result = store.recall_daily("1970-01-01").await.unwrap();
        assert!(result.is_none(), "no entry for ancient date");
    }

    #[tokio::test]
    async fn recall_respects_limit() {
        let store = make_store();
        for i in 0..10 {
            store
                .store(&format!("key:{i}"), &format!("content number {i}"), MemoryCategory::Core)
                .await
                .unwrap();
        }
        let results = store.recall("content", 3).await.unwrap();
        assert_eq!(results.len(), 3, "recall should respect limit=3");
    }

    #[tokio::test]
    async fn recall_limit_zero_returns_empty() {
        let store = make_store();
        store.store("k", "v", MemoryCategory::Core).await.unwrap();
        let results = store.recall("v", 0).await.unwrap();
        assert!(results.is_empty(), "limit=0 → empty results");
    }

    #[tokio::test]
    async fn recall_scores_are_non_negative() {
        let store = make_store();
        store
            .store("k", "some content", MemoryCategory::Core)
            .await
            .unwrap();
        let results = store.recall("some query", 5).await.unwrap();
        for entry in &results {
            assert!(entry.score >= 0.0, "score should be non-negative, got {}", entry.score);
        }
    }

    #[tokio::test]
    async fn store_overwrites_existing_key() {
        let store = make_store();
        store
            .store("key", "original content", MemoryCategory::Core)
            .await
            .unwrap();
        store
            .store("key", "updated content", MemoryCategory::Core)
            .await
            .unwrap();

        let results = store.recall("updated content", 5).await.unwrap();
        let entry = results.iter().find(|e| e.key == "key");
        assert!(entry.is_some(), "key should still exist after overwrite");
        assert_eq!(entry.unwrap().content, "updated content");
    }

    #[tokio::test]
    async fn store_preserves_category() {
        let store = make_store();
        store
            .store("k", "value", MemoryCategory::Custom("my-tag".to_owned()))
            .await
            .unwrap();
        let results = store.recall("value", 5).await.unwrap();
        let entry = results.iter().find(|e| e.key == "k").unwrap();
        assert_eq!(
            entry.category,
            MemoryCategory::Custom("my-tag".to_owned()),
            "category should be preserved"
        );
    }

    #[tokio::test]
    async fn store_daily_appends_multiple_entries() {
        let store = make_store();
        store.store_daily("First entry.").await.unwrap();
        store.store_daily("Second entry.").await.unwrap();

        let date = Utc::now().format("%Y-%m-%d").to_string();
        let content = store.recall_daily(&date).await.unwrap().unwrap();
        assert!(content.contains("First entry."), "daily content should contain first entry");
        assert!(content.contains("Second entry."), "daily content should contain second entry");
    }

    #[tokio::test]
    async fn store_creates_unique_ids() {
        let store = make_store();
        store.store("a", "alpha", MemoryCategory::Core).await.unwrap();
        store.store("b", "beta", MemoryCategory::Core).await.unwrap();

        let ra = store.recall("alpha", 5).await.unwrap();
        let rb = store.recall("beta", 5).await.unwrap();

        let id_a = ra.iter().find(|e| e.key == "a").unwrap().id.clone();
        let id_b = rb.iter().find(|e| e.key == "b").unwrap().id.clone();
        assert_ne!(id_a, id_b, "each entry should have a unique id");
    }
}
