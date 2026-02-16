pub mod models;
pub mod pool;
pub mod schema;
pub mod utils;

use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub type DbPool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("Database initialization error: {0}")]
    Init(String),

    #[error("Database migration error: {0}")]
    Migration(String),

    #[error("Connection pool error: {0}")]
    Pool(#[from] diesel::r2d2::Error),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Diesel error: {0}")]
    Diesel(#[from] diesel::result::Error),
}

fn get_database_path(app: &AppHandle) -> Result<PathBuf, DbError> {
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| DbError::Init(format!("Failed to get app data directory: {}", e)))?;

    std::fs::create_dir_all(&app_dir)
        .map_err(|e| DbError::Init(format!("Failed to create app data directory: {}", e)))?;

    Ok(app_dir.join("app.db"))
}

pub fn init(app: &AppHandle) -> Result<DbPool, DbError> {
    let db_path = get_database_path(app)?;
    let db_url = db_path.to_string_lossy().to_string();

    log::info!("Initializing database at: {}", db_url);

    // Create connection manager
    let manager = ConnectionManager::<SqliteConnection>::new(&db_url);

    // Build connection pool
    let pool = r2d2::Pool::builder()
        .max_size(10)
        .build(manager)
        .map_err(|e| DbError::Init(format!("Failed to create connection pool: {}", e)))?;

    // Run migrations
    let mut conn = pool.get().map_err(|e| DbError::Init(format!("Failed to get database connection: {}", e)))?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| DbError::Migration(e.to_string()))?;

    log::info!("Database initialized successfully");

    Ok(pool)
}
