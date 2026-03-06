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
        assert!(tables.contains(&"schedule_jobs".to_string()));
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
        assert_eq!(version, 1);
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
