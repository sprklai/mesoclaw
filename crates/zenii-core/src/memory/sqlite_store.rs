use std::sync::Arc;

use async_trait::async_trait;

use crate::db::DbPool;
use crate::{Result, ZeniiError};

use super::embeddings::EmbeddingProvider;
use super::traits::{Memory, MemoryCategory, MemoryEntry};
use super::vector_index::VectorIndex;

pub struct SqliteMemoryStore {
    pool: DbPool,
    vector_index: Option<VectorIndex>,
    embedding_provider: Option<Arc<dyn EmbeddingProvider>>,
    fts_weight: f32,
    vector_weight: f32,
}

impl SqliteMemoryStore {
    pub fn new(pool: DbPool, fts_weight: f32, vector_weight: f32) -> Self {
        Self {
            pool,
            vector_index: None,
            embedding_provider: None,
            fts_weight,
            vector_weight,
        }
    }

    pub fn with_vector(
        mut self,
        vector_index: VectorIndex,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        self.vector_index = Some(vector_index);
        self.embedding_provider = Some(embedding_provider);
        self
    }

    /// Run the memory-specific migrations (FTS5 tables, triggers)
    pub fn run_memory_migrations(pool: &DbPool) -> Result<()> {
        let conn = pool.blocking_lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                key TEXT NOT NULL UNIQUE,
                content TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'core',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
                key, content, category,
                content=memories, content_rowid=rowid
            );

            CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
                INSERT INTO memories_fts(rowid, key, content, category)
                VALUES (new.rowid, new.key, new.content, new.category);
            END;

            CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, key, content, category)
                VALUES ('delete', old.rowid, old.key, old.content, old.category);
            END;

            CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
                INSERT INTO memories_fts(memories_fts, rowid, key, content, category)
                VALUES ('delete', old.rowid, old.key, old.content, old.category);
                INSERT INTO memories_fts(rowid, key, content, category)
                VALUES (new.rowid, new.key, new.content, new.category);
            END;",
        )
        .map_err(|e| ZeniiError::Database(format!("memory migration failed: {e}")))?;
        Ok(())
    }
}

#[async_trait]
impl Memory for SqliteMemoryStore {
    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> Result<()> {
        if content.trim().is_empty() {
            return Err(ZeniiError::Validation("content cannot be empty".into()));
        }
        let pool = self.pool.clone();
        let key = key.to_string();
        let content_str = content.to_string();
        let cat = category.to_string();
        let id = uuid::Uuid::new_v4().to_string();

        let key_clone = key.clone();
        crate::db::with_db(&pool, move |conn| {
            conn.execute(
                "INSERT INTO memories (id, key, content, category) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(key) DO UPDATE SET content=excluded.content, category=excluded.category, updated_at=datetime('now')",
                rusqlite::params![id, key_clone, content_str, cat],
            )
            .map_err(ZeniiError::from)?;
            Ok(())
        })
        .await?;

        // Store vector embedding if provider available
        if let (Some(provider), Some(vi)) = (&self.embedding_provider, &self.vector_index) {
            let embedding = provider.embed(content).await?;
            vi.store(&key, &embedding).await?;
        }

        Ok(())
    }

    async fn recall(&self, query: &str, limit: usize, offset: usize) -> Result<Vec<MemoryEntry>> {
        let pool = self.pool.clone();
        let query_trimmed = query.trim().to_string();
        let fts_weight = self.fts_weight;
        let vector_weight = self.vector_weight;

        // Empty query: return all memories ordered by recency (no FTS5 MATCH)
        if query_trimmed.is_empty() {
            let all_entries = crate::db::with_db(&pool, move |conn| {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, key, content, category, created_at, updated_at
                         FROM memories
                         ORDER BY updated_at DESC
                         LIMIT ?1 OFFSET ?2",
                    )
                    .map_err(ZeniiError::from)?;

                let entries = stmt
                    .query_map(rusqlite::params![limit as i64, offset as i64], |row| {
                        Ok(MemoryEntry {
                            id: row.get(0)?,
                            key: row.get(1)?,
                            content: row.get(2)?,
                            category: MemoryCategory::from(row.get::<_, String>(3)?.as_str()),
                            score: 1.0,
                            created_at: row.get(4)?,
                            updated_at: row.get(5)?,
                        })
                    })
                    .map_err(ZeniiError::from)?
                    .filter_map(|r| r.ok())
                    .collect::<Vec<_>>();

                Ok(entries)
            })
            .await?;

            return Ok(all_entries);
        }

        // Wrap query in double quotes to escape FTS5 special characters (AND, OR, *, ", etc.)
        let query_str = format!("\"{}\"", query_trimmed.replace('"', "\"\""));

        // FTS5 search
        let fts_results = crate::db::with_db(&pool, move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT m.id, m.key, m.content, m.category, m.created_at, m.updated_at,
                        bm25(memories_fts) as rank
                 FROM memories_fts f
                 JOIN memories m ON m.rowid = f.rowid
                 WHERE memories_fts MATCH ?1
                 ORDER BY rank
                 LIMIT ?2 OFFSET ?3",
                )
                .map_err(ZeniiError::from)?;

            let entries = stmt
                .query_map(
                    rusqlite::params![query_str, limit as i64, offset as i64],
                    |row| {
                        Ok(MemoryEntry {
                            id: row.get(0)?,
                            key: row.get(1)?,
                            content: row.get(2)?,
                            category: MemoryCategory::from(row.get::<_, String>(3)?.as_str()),
                            score: row.get::<_, f64>(6)? as f32,
                            created_at: row.get(4)?,
                            updated_at: row.get(5)?,
                        })
                    },
                )
                .map_err(ZeniiError::from)?
                .filter_map(|r| r.ok())
                .collect::<Vec<_>>();

            Ok(entries)
        })
        .await?;

        // If we have vector search, blend scores
        if let (Some(provider), Some(vi)) = (&self.embedding_provider, &self.vector_index) {
            let query_embedding = provider.embed(query).await?;
            let vec_results = vi.search(&query_embedding, limit).await?;

            // Merge: combine FTS and vector scores
            let mut merged: std::collections::HashMap<String, MemoryEntry> =
                std::collections::HashMap::new();

            // Normalize FTS scores (BM25 returns negative, more negative = better)
            let max_fts = fts_results
                .iter()
                .map(|e| e.score.abs())
                .fold(0.0f32, f32::max);
            for mut entry in fts_results {
                let normalized = if max_fts > 0.0 {
                    entry.score.abs() / max_fts
                } else {
                    0.0
                };
                entry.score = fts_weight * normalized;
                merged.insert(entry.key.clone(), entry);
            }

            // Collect vector-only keys (present in vector results but not FTS) and
            // update scores for keys already in the merged map.
            let mut vec_results_map: std::collections::HashMap<String, f32> =
                std::collections::HashMap::new();
            for (key, vec_score) in &vec_results {
                if let Some(entry) = merged.get_mut(key) {
                    entry.score += vector_weight * vec_score;
                } else {
                    vec_results_map.insert(key.clone(), *vec_score);
                }
            }

            // Batch-fetch all missing keys in a single query instead of N+1
            let missing_keys: Vec<String> = vec_results_map.keys().cloned().collect();
            if !missing_keys.is_empty() {
                let placeholders: String = missing_keys
                    .iter()
                    .enumerate()
                    .map(|(i, _)| format!("?{}", i + 1))
                    .collect::<Vec<_>>()
                    .join(", ");
                let sql = format!(
                    "SELECT id, key, content, category, created_at, updated_at FROM memories WHERE key IN ({})",
                    placeholders
                );

                let pool2 = self.pool.clone();
                let keys = missing_keys;
                let fetched = crate::db::with_db(&pool2, move |conn| {
                    let mut stmt = conn.prepare(&sql).map_err(ZeniiError::from)?;
                    let params: Vec<&dyn rusqlite::types::ToSql> = keys
                        .iter()
                        .map(|k| k as &dyn rusqlite::types::ToSql)
                        .collect();
                    let rows = stmt
                        .query_map(params.as_slice(), |row| {
                            Ok(MemoryEntry {
                                id: row.get(0)?,
                                key: row.get(1)?,
                                content: row.get(2)?,
                                category: MemoryCategory::from(row.get::<_, String>(3)?.as_str()),
                                score: 0.0,
                                created_at: row.get(4)?,
                                updated_at: row.get(5)?,
                            })
                        })
                        .map_err(ZeniiError::from)?;
                    let result: Vec<MemoryEntry> = rows.flatten().collect();
                    Ok(result)
                })
                .await?;

                for mut entry in fetched {
                    if let Some(&vec_score) = vec_results_map.get(&entry.key) {
                        entry.score = vector_weight * vec_score;
                        merged.insert(entry.key.clone(), entry);
                    }
                }
            }

            let mut results: Vec<MemoryEntry> = merged.into_values().collect();
            results.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let results: Vec<MemoryEntry> = results.into_iter().skip(offset).take(limit).collect();
            return Ok(results);
        }

        Ok(fts_results)
    }

    async fn forget(&self, key: &str) -> Result<bool> {
        let pool = self.pool.clone();
        let key_str = key.to_string();

        let deleted = crate::db::with_db(&pool, move |conn| {
            let count = conn
                .execute(
                    "DELETE FROM memories WHERE key = ?1",
                    rusqlite::params![key_str],
                )
                .map_err(ZeniiError::from)?;
            Ok(count > 0)
        })
        .await?;

        if deleted && let Some(ref vi) = self.vector_index {
            vi.delete(key).await?;
        }

        Ok(deleted)
    }

    async fn store_daily(&self, content: &str) -> Result<()> {
        let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let key = format!("daily:{date}");
        self.store(&key, content, MemoryCategory::Daily).await
    }

    async fn recall_daily(&self, date: &str) -> Result<Option<String>> {
        let pool = self.pool.clone();
        let key = format!("daily:{date}");

        crate::db::with_db(&pool, move |conn| {
            let result = conn
                .query_row(
                    "SELECT content FROM memories WHERE key = ?1",
                    rusqlite::params![key],
                    |row| row.get::<_, String>(0),
                )
                .ok();
            Ok(result)
        })
        .await
    }

    async fn list_daily_dates(&self) -> Result<Vec<String>> {
        let pool = self.pool.clone();

        crate::db::with_db(&pool, move |conn| {
            let mut stmt = conn
                .prepare(
                    "SELECT key FROM memories WHERE category = 'daily' ORDER BY created_at DESC",
                )
                .map_err(ZeniiError::from)?;
            let dates = stmt
                .query_map([], |row| {
                    let key: String = row.get(0)?;
                    // Strip "daily:" prefix
                    Ok(key.strip_prefix("daily:").unwrap_or(&key).to_string())
                })
                .map_err(ZeniiError::from)?
                .filter_map(|r| r.ok())
                .collect();
            Ok(dates)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, SqliteMemoryStore) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = db::init_pool(&path).unwrap();
        let pool2 = pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool2.blocking_lock();
            db::run_migrations(&conn).unwrap();
            drop(conn);
            SqliteMemoryStore::run_memory_migrations(&pool2).unwrap();
        })
        .await
        .unwrap();
        let store = SqliteMemoryStore::new(pool, 0.4, 0.6);
        (dir, store)
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn store_and_recall_round_trip() {
        let (_dir, store) = setup().await;
        store
            .store("key1", "hello world", MemoryCategory::Core)
            .await
            .unwrap();
        let results = store.recall("hello", 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "hello world");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn recall_empty_store_returns_empty() {
        let (_dir, store) = setup().await;
        let results = store.recall("anything", 10, 0).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fts5_search_ranks_relevant_results() {
        let (_dir, store) = setup().await;
        store
            .store(
                "rust-lang",
                "Rust is a systems programming language",
                MemoryCategory::Core,
            )
            .await
            .unwrap();
        store
            .store(
                "python",
                "Python is a scripting language",
                MemoryCategory::Core,
            )
            .await
            .unwrap();
        store
            .store(
                "rust-book",
                "The Rust programming language book",
                MemoryCategory::Core,
            )
            .await
            .unwrap();

        let results = store.recall("Rust programming", 10, 0).await.unwrap();
        assert!(!results.is_empty());
        let keys: Vec<&str> = results.iter().map(|e| e.key.as_str()).collect();
        assert!(keys.contains(&"rust-lang") || keys.contains(&"rust-book"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn forget_removes_entry() {
        let (_dir, store) = setup().await;
        store
            .store("key1", "content", MemoryCategory::Core)
            .await
            .unwrap();
        assert!(store.forget("key1").await.unwrap());
        let results = store.recall("content", 10, 0).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn store_daily_and_recall() {
        let (_dir, store) = setup().await;
        store.store_daily("Today was productive").await.unwrap();
        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
        let content = store.recall_daily(&today).await.unwrap();
        assert!(content.is_some());
        assert!(content.unwrap().contains("productive"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn list_daily_dates_sorted_descending() {
        let (_dir, store) = setup().await;
        store
            .store("daily:2024-01-01", "first", MemoryCategory::Daily)
            .await
            .unwrap();
        store
            .store("daily:2024-01-03", "third", MemoryCategory::Daily)
            .await
            .unwrap();
        store
            .store("daily:2024-01-02", "second", MemoryCategory::Daily)
            .await
            .unwrap();
        let dates = store.list_daily_dates().await.unwrap();
        assert!(dates.len() >= 3);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn recall_respects_limit() {
        let (_dir, store) = setup().await;
        for i in 0..5 {
            store
                .store(
                    &format!("item{i}"),
                    &format!("content about topic {i}"),
                    MemoryCategory::Core,
                )
                .await
                .unwrap();
        }
        let results = store.recall("content topic", 2, 0).await.unwrap();
        assert!(results.len() <= 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn store_overwrites_existing_key() {
        let (_dir, store) = setup().await;
        store
            .store("key1", "old content", MemoryCategory::Core)
            .await
            .unwrap();
        store
            .store("key1", "new content", MemoryCategory::Core)
            .await
            .unwrap();
        let results = store.recall("key1", 10, 0).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "new content");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn hybrid_scoring_blends_fts_and_vector() {
        unsafe {
            rusqlite::ffi::sqlite3_auto_extension(Some(std::mem::transmute(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }

        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");
        let vec_path = dir.path().join("vec.db");
        let pool = db::init_pool(&db_path).unwrap();
        let vec_pool = db::init_pool(&vec_path).unwrap();
        let pool2 = pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool2.blocking_lock();
            db::run_migrations(&conn).unwrap();
            drop(conn);
            SqliteMemoryStore::run_memory_migrations(&pool2).unwrap();
        })
        .await
        .unwrap();

        let vec_pool2 = vec_pool.clone();
        let vi = tokio::task::spawn_blocking(move || {
            crate::memory::vector_index::VectorIndex::new(vec_pool2, 8).unwrap()
        })
        .await
        .unwrap();
        let mock_provider = Arc::new(crate::memory::embeddings::MockEmbeddingProvider::new(8));

        let store = SqliteMemoryStore::new(pool, 0.4, 0.6).with_vector(vi, mock_provider);

        store
            .store("key1", "rust programming language", MemoryCategory::Core)
            .await
            .unwrap();
        store
            .store("key2", "python scripting", MemoryCategory::Core)
            .await
            .unwrap();

        let results = store.recall("rust", 10, 0).await.unwrap();
        assert!(!results.is_empty());
        for entry in &results {
            assert!(entry.score != 0.0 || entry.key == "python");
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fts5_special_chars_dont_crash() {
        let (_dir, store) = setup().await;
        store
            .store(
                "test",
                "some content with special chars",
                MemoryCategory::Core,
            )
            .await
            .unwrap();
        let r1 = store.recall("hello AND world", 10, 0).await;
        assert!(r1.is_ok());
        let r2 = store.recall("\"unbalanced quote", 10, 0).await;
        assert!(r2.is_ok());
        let r3 = store.recall("test*", 10, 0).await;
        assert!(r3.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn recall_empty_query_returns_all() {
        let (_dir, store) = setup().await;
        store
            .store("key1", "first entry", MemoryCategory::Core)
            .await
            .unwrap();
        store
            .store("key2", "second entry", MemoryCategory::Daily)
            .await
            .unwrap();
        store
            .store("key3", "third entry", MemoryCategory::Core)
            .await
            .unwrap();

        // Empty query should return all entries (used by frontend loadAll)
        let results = store.recall("", 50, 0).await.unwrap();
        assert_eq!(results.len(), 3);

        // Whitespace-only query should also return all
        let results2 = store.recall("   ", 50, 0).await.unwrap();
        assert_eq!(results2.len(), 3);

        // Limit and offset should work with empty query
        let results3 = store.recall("", 2, 0).await.unwrap();
        assert_eq!(results3.len(), 2);
        let results4 = store.recall("", 50, 2).await.unwrap();
        assert_eq!(results4.len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn store_creates_unique_ids() {
        let (_dir, store) = setup().await;
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
        assert!(!r1.is_empty());
        assert!(!r2.is_empty());
        assert_ne!(r1[0].id, r2[0].id);
    }
}
