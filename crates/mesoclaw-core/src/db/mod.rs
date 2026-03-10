use crate::{MesoError, Result};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type DbPool = Arc<Mutex<Connection>>;

pub fn init_pool(path: &Path) -> Result<DbPool> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;",
    )?;
    Ok(Arc::new(Mutex::new(conn)))
}

pub fn init_memory_pool(path: &Path) -> Result<DbPool> {
    let pool = init_pool(path)?;
    Ok(pool)
}

pub async fn with_db<F, T>(pool: &DbPool, f: F) -> Result<T>
where
    F: FnOnce(&Connection) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.blocking_lock();
        f(&conn)
    })
    .await
    .map_err(|e| MesoError::Database(format!("spawn_blocking join error: {e}")))?
}

pub fn run_migrations(conn: &Connection) -> Result<()> {
    let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;

    if version < 1 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT 'Untitled',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
                role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant', 'tool')),
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created_at);

            CREATE TABLE IF NOT EXISTS providers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                provider_type TEXT NOT NULL,
                base_url TEXT,
                default_model TEXT,
                is_enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS schedule_jobs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                cron_expr TEXT NOT NULL,
                prompt TEXT NOT NULL,
                is_enabled INTEGER NOT NULL DEFAULT 1,
                last_run TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            PRAGMA user_version = 1;",
        )?;
    }

    if version < 2 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS user_observations (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                key TEXT NOT NULL UNIQUE,
                value TEXT NOT NULL,
                confidence REAL NOT NULL DEFAULT 0.5,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_observations_category
                ON user_observations(category);
            CREATE INDEX IF NOT EXISTS idx_observations_confidence
                ON user_observations(confidence DESC);

            PRAGMA user_version = 2;",
        )?;
    }

    if version < 3 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS ai_providers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                base_url TEXT NOT NULL,
                requires_api_key INTEGER NOT NULL DEFAULT 1,
                is_active INTEGER NOT NULL DEFAULT 1,
                is_user_defined INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS ai_models (
                id TEXT PRIMARY KEY,
                provider_id TEXT NOT NULL REFERENCES ai_providers(id),
                model_id TEXT NOT NULL,
                display_name TEXT NOT NULL,
                context_limit INTEGER,
                is_custom INTEGER NOT NULL DEFAULT 0,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_ai_models_provider
                ON ai_models(provider_id);

            PRAGMA user_version = 3;",
        )?;
    }

    if version < 4 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS tool_calls (
                id TEXT PRIMARY KEY,
                message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                session_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                args TEXT NOT NULL,
                output TEXT,
                success INTEGER,
                duration_ms INTEGER,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_tool_calls_message
                ON tool_calls(message_id);

            PRAGMA user_version = 4;",
        )?;
    }

    if version < 5 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS context_summaries (
                key TEXT PRIMARY KEY,
                summary TEXT NOT NULL,
                source_hash TEXT NOT NULL,
                generated_at TEXT NOT NULL DEFAULT (datetime('now')),
                model_id TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS skill_proposals (
                id TEXT PRIMARY KEY,
                action TEXT NOT NULL CHECK(action IN ('create', 'update', 'delete')),
                skill_name TEXT NOT NULL,
                content TEXT,
                rationale TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'approved', 'rejected', 'expired')),
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                resolved_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_skill_proposals_status
                ON skill_proposals(status);

            PRAGMA user_version = 5;",
        )?;

        // Add summary column to sessions table (ALTER TABLE is separate from batch)
        // Check if column exists first to be idempotent
        let has_summary: bool = conn
            .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='sessions'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, String>(0)))
            .map(|sql| sql.contains("summary"))
            .unwrap_or(false);

        if !has_summary {
            conn.execute_batch("ALTER TABLE sessions ADD COLUMN summary TEXT;")?;
        }
    }

    if version < 6 {
        conn.execute_batch(
            "DROP TABLE IF EXISTS schedule_jobs;

            CREATE TABLE IF NOT EXISTS scheduled_jobs (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                schedule_json TEXT NOT NULL,
                session_target TEXT NOT NULL DEFAULT 'main',
                payload_json TEXT NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                error_count INTEGER NOT NULL DEFAULT 0,
                next_run TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                active_hours_json TEXT,
                delete_after_run INTEGER NOT NULL DEFAULT 0
            );

            PRAGMA user_version = 6;",
        )?;
    }

    if version < 7 {
        // Add source column to sessions table for channel tracking
        let has_source: bool = conn
            .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='sessions'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, String>(0)))
            .map(|sql| sql.contains("source"))
            .unwrap_or(false);

        if !has_source {
            conn.execute_batch(
                "ALTER TABLE sessions ADD COLUMN source TEXT NOT NULL DEFAULT 'web';",
            )?;
        }

        conn.execute_batch("PRAGMA user_version = 7;")?;
    }

    if version < 8 {
        // Add supports_tools column to ai_models table
        let has_supports_tools: bool = conn
            .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='ai_models'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, String>(0)))
            .map(|sql| sql.contains("supports_tools"))
            .unwrap_or(false);

        if !has_supports_tools {
            conn.execute_batch(
                "ALTER TABLE ai_models ADD COLUMN supports_tools INTEGER NOT NULL DEFAULT 1;",
            )?;
        }

        conn.execute_batch("PRAGMA user_version = 8;")?;
    }

    if version < 9 {
        // Add channel_key column to sessions for deduplication
        let has_channel_key: bool = conn
            .prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='sessions'")
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, String>(0)))
            .map(|sql| sql.contains("channel_key"))
            .unwrap_or(false);

        if !has_channel_key {
            conn.execute_batch("ALTER TABLE sessions ADD COLUMN channel_key TEXT;")?;
        }

        conn.execute_batch(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_channel_key
                ON sessions(channel_key) WHERE channel_key IS NOT NULL;

            PRAGMA user_version = 9;",
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn init_pool_creates_db() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = init_pool(&path).unwrap();
        assert!(path.exists());
        drop(pool);
    }

    #[test]
    fn run_migrations_creates_tables() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"sessions".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"providers".to_string()));
        assert!(tables.contains(&"scheduled_jobs".to_string()));
    }

    #[test]
    fn migrations_are_idempotent() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, 9);
    }

    #[test]
    fn migration_v2_creates_observations_table() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"user_observations".to_string()));
    }

    #[test]
    fn migration_v2_creates_indexes() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        let indexes: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='index' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(indexes.contains(&"idx_observations_category".to_string()));
        assert!(indexes.contains(&"idx_observations_confidence".to_string()));
    }

    #[test]
    fn migration_v3_creates_ai_providers() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"ai_providers".to_string()));
    }

    #[test]
    fn migration_v3_creates_ai_models() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"ai_models".to_string()));
    }

    // TV.18 — Migration v4 creates tool_calls table
    #[test]
    fn migration_v4_creates_tool_calls() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"tool_calls".to_string()));
    }

    // 15.3.38 — Migration v5 creates context_summaries table
    #[test]
    fn migration_v5_creates_context_summaries() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"context_summaries".to_string()));
    }

    // 15.3.39 — Migration v5 creates skill_proposals table
    #[test]
    fn migration_v5_creates_skill_proposals() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"skill_proposals".to_string()));

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert!(version >= 5);
    }

    // 15.3.39b — Migration v5 adds summary column to sessions
    #[test]
    fn migration_v5_adds_session_summary_column() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        // Verify we can insert and read a session summary
        conn.execute(
            "INSERT INTO sessions (id, title, summary) VALUES ('s1', 'Test', 'A summary')",
            [],
        )
        .unwrap();

        let summary: Option<String> = conn
            .query_row("SELECT summary FROM sessions WHERE id = 's1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(summary, Some("A summary".to_string()));
    }

    // 16.42 — Migration v6 creates scheduled_jobs table
    #[test]
    fn migration_v6_creates_scheduled_jobs() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"scheduled_jobs".to_string()));
        // Orphaned v1 schedule_jobs should be gone
        assert!(!tables.contains(&"schedule_jobs".to_string()));

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, 9);
    }

    // IN.9 — Migration v9 adds channel_key column and unique index
    #[test]
    fn migration_v9_adds_channel_key_column() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let conn = Connection::open(&path).unwrap();
        run_migrations(&conn).unwrap();

        // Verify channel_key column works
        conn.execute(
            "INSERT INTO sessions (id, title, channel_key) VALUES ('s1', 'Test', 'telegram:123')",
            [],
        )
        .unwrap();

        let ck: Option<String> = conn
            .query_row(
                "SELECT channel_key FROM sessions WHERE id = 's1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(ck, Some("telegram:123".to_string()));

        // Verify unique index
        let result = conn.execute(
            "INSERT INTO sessions (id, title, channel_key) VALUES ('s2', 'Test2', 'telegram:123')",
            [],
        );
        assert!(result.is_err());

        // NULL channel_key should be allowed (web sessions)
        conn.execute("INSERT INTO sessions (id, title) VALUES ('s3', 'Web')", [])
            .unwrap();
        conn.execute("INSERT INTO sessions (id, title) VALUES ('s4', 'Web2')", [])
            .unwrap();
    }

    #[tokio::test]
    async fn with_db_does_not_block_runtime() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.db");
        let pool = init_pool(&path).unwrap();

        // Run migration inside with_db
        with_db(&pool, |conn| run_migrations(conn)).await.unwrap();

        // Insert and query via with_db
        let count = with_db(&pool, |conn| {
            conn.execute(
                "INSERT INTO sessions (id, title) VALUES (?1, ?2)",
                rusqlite::params!["s1", "Test"],
            )
            .map_err(MesoError::from)?;

            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))
                .map_err(MesoError::from)?;

            Ok(count)
        })
        .await
        .unwrap();

        assert_eq!(count, 1);
    }
}
