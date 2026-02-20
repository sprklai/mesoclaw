//! SQLite-backed implementation of the [`Memory`] trait.
//!
//! [`SqliteMemoryStore`] persists memory entries to a SQLite database and uses
//! an FTS5 virtual table for full-text search recall.  It is a drop-in
//! replacement for [`super::store::InMemoryStore`] behind the same [`Memory`]
//! trait.
//!
//! # Schema
//! The store expects the `memories` table and `memories_fts` FTS5 virtual table
//! to already exist (created by the migration
//! `2026-02-19-230000_create_memories`).
//!
//! # Connection
//! A [`rusqlite::Connection`] wrapped in an `Arc<Mutex<…>>` is held internally,
//! making the store `Send + Sync` while keeping SQLite's single-writer
//! requirement.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::{Connection, params};
use uuid::Uuid;

use super::traits::{Memory, MemoryCategory, MemoryEntry};

// ─── Schema setup ─────────────────────────────────────────────────────────────

/// DDL executed when the store is opened with [`SqliteMemoryStore::open`].
/// This mirrors the migration SQL so that in-process tests work without
/// running Diesel migrations.
const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS memories (
    id TEXT PRIMARY KEY NOT NULL,
    key TEXT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'core',
    embedding BLOB,
    score REAL NOT NULL DEFAULT 0.5,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_memories_category ON memories (category);
CREATE INDEX IF NOT EXISTS idx_memories_key ON memories (key);

CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts USING fts5(
    content,
    content='memories',
    content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER IF NOT EXISTS memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;
CREATE TRIGGER IF NOT EXISTS memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    INSERT INTO memories_fts(rowid, content) VALUES (new.rowid, new.content);
END;
"#;

// ─── MemoryCategory conversion ────────────────────────────────────────────────

/// Serialise a [`MemoryCategory`] to the TEXT stored in the database.
fn category_to_str(cat: &MemoryCategory) -> String {
    match cat {
        MemoryCategory::Core => "core".to_owned(),
        MemoryCategory::Daily => "daily".to_owned(),
        MemoryCategory::Conversation => "conversation".to_owned(),
        MemoryCategory::Custom(s) => format!("custom:{s}"),
    }
}

/// Deserialise a TEXT column back to a [`MemoryCategory`].
fn str_to_category(s: &str) -> MemoryCategory {
    match s {
        "core" => MemoryCategory::Core,
        "daily" => MemoryCategory::Daily,
        "conversation" => MemoryCategory::Conversation,
        other => {
            if let Some(tag) = other.strip_prefix("custom:") {
                MemoryCategory::Custom(tag.to_owned())
            } else {
                MemoryCategory::Custom(other.to_owned())
            }
        }
    }
}

// ─── SqliteMemoryStore ────────────────────────────────────────────────────────

/// SQLite-backed, thread-safe memory store with FTS5 full-text search.
pub struct SqliteMemoryStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteMemoryStore {
    /// Open (or create) a SQLite database at `path` and initialise the schema.
    ///
    /// This is the primary constructor for production use.
    pub fn open(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| format!("SQLite open error: {e}"))?;
        // Enable WAL for better concurrent read performance.  `PRAGMA
        // journal_mode` returns a result row, so we must use query_row rather
        // than execute_batch.
        conn.query_row("PRAGMA journal_mode=WAL", [], |_| Ok(()))
            .map_err(|e| format!("SQLite PRAGMA error: {e}"))?;
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| format!("SQLite schema error: {e}"))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory SQLite store — useful for tests.
    pub fn in_memory() -> Result<Self, String> {
        let conn =
            Connection::open_in_memory().map_err(|e| format!("SQLite in-memory error: {e}"))?;
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| format!("SQLite schema error: {e}"))?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Acquire the connection lock, mapping the poison-error to a `String`.
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.conn
            .lock()
            .map_err(|e| format!("SQLite lock error: {e}"))
    }
}

// ─── Memory implementation ────────────────────────────────────────────────────

#[async_trait]
impl Memory for SqliteMemoryStore {
    /// Store or overwrite a memory entry identified by `key`.
    ///
    /// Uses `INSERT OR REPLACE` so that the row is updated atomically when the
    /// key already exists.  The `id` and `created_at` of an existing entry are
    /// preserved by fetching them first.
    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let category_str = category_to_str(&category);
        let conn = self.lock()?;

        // Check whether this key already exists so we can preserve id/created_at.
        let existing: Option<(String, String)> = conn
            .query_row(
                "SELECT id, created_at FROM memories WHERE key = ?1",
                params![key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        let (id, created_at) =
            existing.unwrap_or_else(|| (Uuid::new_v4().to_string(), now.clone()));

        conn.execute(
            r#"
            INSERT INTO memories (id, key, content, category, score, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, 0.5, ?5, ?6)
            ON CONFLICT(key) DO UPDATE SET
                content    = excluded.content,
                category   = excluded.category,
                score      = excluded.score,
                updated_at = excluded.updated_at
            "#,
            params![id, key, content, category_str, created_at, now],
        )
        .map_err(|e| format!("SQLite insert error: {e}"))?;

        Ok(())
    }

    /// Search for entries that match `query` using FTS5.
    ///
    /// Returns at most `limit` entries.  Each entry's `score` field reflects
    /// the FTS5 `bm25()` rank (negated so higher = better, clamped to [0, 1]).
    async fn recall(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>, String> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let conn = self.lock()?;

        // When the query is empty or blank, fall back to a full table scan
        // ordered by updated_at so the most recent entries are returned.
        let entries: Vec<MemoryEntry> = if query.trim().is_empty() {
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT id, key, content, category, score, created_at, updated_at
                    FROM memories
                    ORDER BY updated_at DESC
                    LIMIT ?1
                    "#,
                )
                .map_err(|e| format!("SQLite prepare error: {e}"))?;

            let rows = stmt
                .query_map(params![limit as i64], |row| {
                    Ok(MemoryEntry {
                        id: row.get(0)?,
                        key: row.get(1)?,
                        content: row.get(2)?,
                        category: str_to_category(&row.get::<_, String>(3)?),
                        score: row.get(4)?,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                })
                .map_err(|e| format!("SQLite query error: {e}"))?;

            rows.filter_map(|r| r.ok()).collect()
        } else {
            // Escape FTS5 special characters in the query so user input cannot
            // inject FTS5 operators.  Wrap each whitespace-delimited word as a
            // quoted phrase.
            let fts_query = sanitise_fts_query(query);

            // FTS5 bm25() returns negative values: more-negative = better match.
            // We negate and clamp to produce a score in (0, 1].
            let mut stmt = conn
                .prepare(
                    r#"
                    SELECT m.id, m.key, m.content, m.category, m.score,
                           m.created_at, m.updated_at,
                           -bm25(memories_fts) AS rank
                    FROM memories_fts
                    JOIN memories m ON m.rowid = memories_fts.rowid
                    WHERE memories_fts MATCH ?1
                    ORDER BY rank DESC
                    LIMIT ?2
                    "#,
                )
                .map_err(|e| format!("SQLite prepare error: {e}"))?;

            let rows = stmt
                .query_map(params![fts_query, limit as i64], |row| {
                    let raw_rank: f64 = row.get(7)?;
                    // Normalise rank to (0, 1].  bm25 values are unbounded so
                    // we use a simple sigmoid-style clamp.
                    let score = (raw_rank / (1.0 + raw_rank)).clamp(0.0, 1.0) as f32;
                    Ok(MemoryEntry {
                        id: row.get(0)?,
                        key: row.get(1)?,
                        content: row.get(2)?,
                        category: str_to_category(&row.get::<_, String>(3)?),
                        score,
                        created_at: row.get(5)?,
                        updated_at: row.get(6)?,
                    })
                })
                .map_err(|e| format!("SQLite query error: {e}"))?;

            let mut results: Vec<MemoryEntry> = rows.filter_map(|r| r.ok()).collect();

            // If FTS matched nothing (e.g. very short query or no matches),
            // fall back to a keyword LIKE scan so `recall` never returns empty
            // when entries exist.
            if results.is_empty() {
                let like_pat = format!("%{}%", query.replace('%', "\\%").replace('_', "\\_"));
                let mut fallback = conn
                    .prepare(
                        r#"
                        SELECT id, key, content, category, score, created_at, updated_at
                        FROM memories
                        WHERE content LIKE ?1 ESCAPE '\'
                        ORDER BY updated_at DESC
                        LIMIT ?2
                        "#,
                    )
                    .map_err(|e| format!("SQLite prepare error: {e}"))?;

                let fb_rows = fallback
                    .query_map(params![like_pat, limit as i64], |row| {
                        Ok(MemoryEntry {
                            id: row.get(0)?,
                            key: row.get(1)?,
                            content: row.get(2)?,
                            category: str_to_category(&row.get::<_, String>(3)?),
                            score: 0.1,
                            created_at: row.get(5)?,
                            updated_at: row.get(6)?,
                        })
                    })
                    .map_err(|e| format!("SQLite query error: {e}"))?;

                results = fb_rows.filter_map(|r| r.ok()).collect();
            }

            results
        };

        Ok(entries)
    }

    /// Remove an entry by `key`.  Returns `true` if the entry was found and
    /// deleted, `false` if the key did not exist.
    async fn forget(&self, key: &str) -> Result<bool, String> {
        let conn = self.lock()?;
        let rows_affected = conn
            .execute("DELETE FROM memories WHERE key = ?1", params![key])
            .map_err(|e| format!("SQLite delete error: {e}"))?;
        Ok(rows_affected > 0)
    }

    /// Append `content` to today's daily diary entry.
    ///
    /// The key is `daily:YYYY-MM-DD`.  If an entry already exists its content
    /// is extended with a blank-line separator.
    async fn store_daily(&self, content: &str) -> Result<(), String> {
        let date = Utc::now().format("%Y-%m-%d").to_string();
        let key = format!("daily:{date}");

        // Read existing content while holding no lock across awaits.
        let existing = {
            let conn = self.lock()?;
            conn.query_row(
                "SELECT content FROM memories WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .ok()
        };

        let full_content = match existing {
            Some(prev) if !prev.is_empty() => format!("{prev}\n\n{content}"),
            _ => content.to_owned(),
        };

        self.store(&key, &full_content, MemoryCategory::Daily).await
    }

    /// Retrieve the diary content for `date` (format `YYYY-MM-DD`).
    /// Returns `None` if no entry exists for that date.
    async fn recall_daily(&self, date: &str) -> Result<Option<String>, String> {
        let key = format!("daily:{date}");
        let conn = self.lock()?;
        let result = conn
            .query_row(
                "SELECT content FROM memories WHERE key = ?1",
                params![key],
                |row| row.get::<_, String>(0),
            )
            .ok();
        Ok(result)
    }
}

// ─── FTS query sanitisation ───────────────────────────────────────────────────

/// Convert a free-text query into a safe FTS5 MATCH expression.
///
/// Each whitespace-separated token is double-quoted so FTS5 treats it as a
/// literal phrase rather than an operator.  Double-quote characters inside
/// tokens are escaped by doubling them.
fn sanitise_fts_query(query: &str) -> String {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|tok| {
            let escaped = tok.replace('"', "\"\"");
            format!("\"{escaped}\"")
        })
        .collect();
    tokens.join(" ")
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::traits::MemoryCategory;

    fn make_store() -> SqliteMemoryStore {
        SqliteMemoryStore::in_memory().expect("in-memory SQLite store")
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
        let entry = results.iter().find(|e| e.key == "user:name");
        assert!(entry.is_some(), "key user:name should appear in recall");
        assert_eq!(entry.unwrap().content, "Alice");
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
        let results = store.recall("v", 5).await.unwrap();
        let keys: Vec<&str> = results.iter().map(|e| e.key.as_str()).collect();
        assert!(
            !keys.contains(&"k"),
            "forgotten key should not appear in recall"
        );
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
        store
            .store_daily("Today I worked on the memory system.")
            .await
            .unwrap();

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
                .store(
                    &format!("key:{i}"),
                    &format!("content number {i}"),
                    MemoryCategory::Core,
                )
                .await
                .unwrap();
        }
        let results = store.recall("content", 3).await.unwrap();
        assert!(
            results.len() <= 3,
            "recall should respect limit=3, got {}",
            results.len()
        );
    }

    #[tokio::test]
    async fn recall_limit_zero_returns_empty() {
        let store = make_store();
        store.store("k", "v", MemoryCategory::Core).await.unwrap();
        let results = store.recall("v", 0).await.unwrap();
        assert!(results.is_empty(), "limit=0 → empty results");
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

        // Verify the key still exists with updated content.
        let conn = store.lock().unwrap();
        let content: String = conn
            .query_row(
                "SELECT content FROM memories WHERE key = 'key'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(content, "updated content");
    }

    #[tokio::test]
    async fn store_preserves_category() {
        let store = make_store();
        store
            .store("k", "value", MemoryCategory::Custom("my-tag".to_owned()))
            .await
            .unwrap();
        let conn = store.lock().unwrap();
        let cat: String = conn
            .query_row("SELECT category FROM memories WHERE key = 'k'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(cat, "custom:my-tag", "category should be preserved");
    }

    #[tokio::test]
    async fn store_daily_appends_multiple_entries() {
        let store = make_store();
        store.store_daily("First entry.").await.unwrap();
        store.store_daily("Second entry.").await.unwrap();

        let date = Utc::now().format("%Y-%m-%d").to_string();
        let content = store.recall_daily(&date).await.unwrap().unwrap();
        assert!(
            content.contains("First entry."),
            "daily content should contain first entry"
        );
        assert!(
            content.contains("Second entry."),
            "daily content should contain second entry"
        );
    }

    #[tokio::test]
    async fn store_creates_unique_ids() {
        let store = make_store();
        store
            .store("a", "alpha", MemoryCategory::Core)
            .await
            .unwrap();
        store
            .store("b", "beta", MemoryCategory::Core)
            .await
            .unwrap();

        let conn = store.lock().unwrap();
        let id_a: String = conn
            .query_row("SELECT id FROM memories WHERE key = 'a'", [], |row| {
                row.get(0)
            })
            .unwrap();
        let id_b: String = conn
            .query_row("SELECT id FROM memories WHERE key = 'b'", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_ne!(id_a, id_b, "each entry should have a unique id");
    }

    #[tokio::test]
    async fn store_preserves_id_on_overwrite() {
        let store = make_store();
        store
            .store("key", "v1", MemoryCategory::Core)
            .await
            .unwrap();
        let id_before: String = {
            let conn = store.lock().unwrap();
            conn.query_row("SELECT id FROM memories WHERE key = 'key'", [], |row| {
                row.get(0)
            })
            .unwrap()
        };

        store
            .store("key", "v2", MemoryCategory::Core)
            .await
            .unwrap();
        let id_after: String = {
            let conn = store.lock().unwrap();
            conn.query_row("SELECT id FROM memories WHERE key = 'key'", [], |row| {
                row.get(0)
            })
            .unwrap()
        };

        assert_eq!(
            id_before, id_after,
            "id should be preserved across overwrites"
        );
    }

    #[tokio::test]
    async fn recall_scores_are_non_negative() {
        let store = make_store();
        store
            .store("k", "some content", MemoryCategory::Core)
            .await
            .unwrap();
        let results = store.recall("some content", 5).await.unwrap();
        for entry in &results {
            assert!(
                entry.score >= 0.0,
                "score should be non-negative, got {}",
                entry.score
            );
        }
    }

    #[tokio::test]
    async fn open_with_tempfile() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test_memory.db");
        let store = SqliteMemoryStore::open(path.to_str().unwrap()).expect("open store on disk");
        store
            .store("persistent:key", "hello world", MemoryCategory::Core)
            .await
            .unwrap();
        let results = store.recall("hello world", 5).await.unwrap();
        assert!(!results.is_empty(), "on-disk store should recall entries");
    }
}
