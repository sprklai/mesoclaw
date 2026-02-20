use diesel::SqliteConnection;
use diesel::r2d2::{ConnectionManager, PooledConnection};

use crate::database::DbPool;

/// Get a pooled database connection, mapping the error to a `String`.
pub fn db_conn(
    pool: &DbPool,
) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>, String> {
    pool.get().map_err(|e| format!("Database error: {e}"))
}

/// Format a contextual error message.
pub fn err_ctx(ctx: &str, err: impl std::fmt::Display) -> String {
    format!("{ctx}: {err}")
}
