use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;

use crate::Result;

use super::traits::{Memory, MemoryCategory, MemoryEntry};

pub struct InMemoryStore {
    memories: tokio::sync::Mutex<HashMap<String, MemoryEntry>>,
    daily: tokio::sync::Mutex<HashMap<String, Vec<String>>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self {
            memories: tokio::sync::Mutex::new(HashMap::new()),
            daily: tokio::sync::Mutex::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Memory for InMemoryStore {
    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> Result<()> {
        if content.trim().is_empty() {
            return Err(crate::MesoError::Validation(
                "content cannot be empty".into(),
            ));
        }
        let mut memories = self.memories.lock().await;
        let now = Utc::now().to_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        let entry = MemoryEntry {
            id,
            key: key.to_string(),
            content: content.to_string(),
            category,
            score: 1.0,
            created_at: now.clone(),
            updated_at: now,
        };
        memories.insert(key.to_string(), entry);
        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize, offset: usize) -> Result<Vec<MemoryEntry>> {
        let memories = self.memories.lock().await;
        let query_lower = query.to_lowercase();
        let results: Vec<MemoryEntry> = memories
            .values()
            .filter(|e| {
                e.key.to_lowercase().contains(&query_lower)
                    || e.content.to_lowercase().contains(&query_lower)
            })
            .skip(offset)
            .take(limit)
            .cloned()
            .collect();
        Ok(results)
    }

    async fn forget(&self, key: &str) -> Result<bool> {
        let mut memories = self.memories.lock().await;
        Ok(memories.remove(key).is_some())
    }

    async fn store_daily(&self, content: &str) -> Result<()> {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let mut daily = self.daily.lock().await;
        daily.entry(date).or_default().push(content.to_string());
        Ok(())
    }

    async fn recall_daily(&self, date: &str) -> Result<Option<String>> {
        let daily = self.daily.lock().await;
        Ok(daily.get(date).map(|entries| entries.join("\n")))
    }

    async fn list_daily_dates(&self) -> Result<Vec<String>> {
        let daily = self.daily.lock().await;
        let mut dates: Vec<String> = daily.keys().cloned().collect();
        dates.sort_by(|a, b| b.cmp(a)); // descending
        Ok(dates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_and_recall_round_trip() {
        let store = InMemoryStore::new();
        store
            .store("key1", "hello world", MemoryCategory::Core)
            .await
            .unwrap();
        let results = store.recall("hello", 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello world");
    }

    #[tokio::test]
    async fn recall_empty_store_returns_empty() {
        let store = InMemoryStore::new();
        let results = store.recall("anything", 10, 0).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn forget_existing_key_returns_true() {
        let store = InMemoryStore::new();
        store
            .store("key1", "content", MemoryCategory::Core)
            .await
            .unwrap();
        assert!(store.forget("key1").await.unwrap());
    }

    #[tokio::test]
    async fn forget_nonexistent_key_returns_false() {
        let store = InMemoryStore::new();
        assert!(!store.forget("nope").await.unwrap());
    }

    #[tokio::test]
    async fn store_daily_and_recall_daily_round_trip() {
        let store = InMemoryStore::new();
        store.store_daily("Today was good").await.unwrap();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let content = store.recall_daily(&today).await.unwrap();
        assert!(content.is_some());
        assert!(content.unwrap().contains("Today was good"));
    }

    #[tokio::test]
    async fn recall_daily_nonexistent_date_returns_none() {
        let store = InMemoryStore::new();
        let result = store.recall_daily("1999-01-01").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn recall_respects_limit() {
        let store = InMemoryStore::new();
        for i in 0..5 {
            store
                .store(
                    &format!("key{i}"),
                    &format!("content {i}"),
                    MemoryCategory::Core,
                )
                .await
                .unwrap();
        }
        let results = store.recall("content", 2, 0).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn store_overwrites_existing_key() {
        let store = InMemoryStore::new();
        store
            .store("key1", "old", MemoryCategory::Core)
            .await
            .unwrap();
        store
            .store("key1", "new", MemoryCategory::Core)
            .await
            .unwrap();
        let results = store.recall("key1", 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "new");
    }

    #[tokio::test]
    async fn store_preserves_category() {
        let store = InMemoryStore::new();
        store
            .store("key1", "content", MemoryCategory::Conversation)
            .await
            .unwrap();
        let results = store.recall("content", 10, 0).await.unwrap();
        assert_eq!(results[0].category, MemoryCategory::Conversation);
    }

    #[tokio::test]
    async fn store_daily_appends_multiple_entries() {
        let store = InMemoryStore::new();
        store.store_daily("Entry 1").await.unwrap();
        store.store_daily("Entry 2").await.unwrap();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let content = store.recall_daily(&today).await.unwrap().unwrap();
        assert!(content.contains("Entry 1"));
        assert!(content.contains("Entry 2"));
    }

    #[tokio::test]
    async fn store_empty_content_returns_error() {
        let store = InMemoryStore::new();
        let result = store.store("key", "", MemoryCategory::Core).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn store_whitespace_only_returns_error() {
        let store = InMemoryStore::new();
        let result = store.store("key", "   \n\t  ", MemoryCategory::Core).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn recall_with_offset_skips_entries() {
        let store = InMemoryStore::new();
        for i in 0..5 {
            store
                .store(
                    &format!("key{i}"),
                    &format!("rust topic {i}"),
                    MemoryCategory::Core,
                )
                .await
                .unwrap();
        }
        let all = store.recall("rust", 10, 0).await.unwrap();
        let offset = store.recall("rust", 10, 2).await.unwrap();
        assert_eq!(all.len(), 5);
        assert_eq!(offset.len(), 3);
    }

    #[tokio::test]
    async fn recall_offset_beyond_results_returns_empty() {
        let store = InMemoryStore::new();
        store
            .store("key1", "rust content", MemoryCategory::Core)
            .await
            .unwrap();
        let results = store.recall("rust", 10, 100).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn store_creates_unique_ids() {
        let store = InMemoryStore::new();
        store
            .store("key1", "content1", MemoryCategory::Core)
            .await
            .unwrap();
        store
            .store("key2", "content2", MemoryCategory::Core)
            .await
            .unwrap();
        let r1 = store.recall("key1", 1, 0).await.unwrap();
        let r2 = store.recall("key2", 1, 0).await.unwrap();
        assert_ne!(r1[0].id, r2[0].id);
    }
}
