# Backend Code Standards (src-tauri/)

This directory contains the Rust backend for the application, built with Tauri 2, Diesel ORM, and async patterns.

## Tech Stack

- **Rust 2024 Edition** - Systems programming language
- **Tauri 2** - Desktop application framework
- **Diesel ORM** - Database ORM and migrations
- **Tokio** - Async runtime
- **async-trait** - Async trait support
- **thiserror** - Error handling
- **serde** - Serialization/deserialization
- **ssh2** - SSH tunneling for remote databases

## Quick Reference

```bash
# Development (run from src-tauri/)
cargo run                 # Build and run in debug mode
cargo test --lib          # Run unit tests
cargo test --lib -- --nocapture  # Run with output

# Build
cargo build --release     # Optimized release build
cargo check               # Quick compile check
cargo clippy              # Lint checks

# Database
diesel migration run      # Run pending migrations
diesel migration revert   # Revert last migration
diesel migration refresh  # Rebuild database (destructive)
```

## Code Style

### Naming Conventions

- **Modules**: `snake_case` (e.g., `connection_manager.rs`)
- **Types/Structs**: `PascalCase` (e.g., `ConnectionConfig`)
- **Functions**: `snake_case` (e.g., `get_table_details`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_CONNECTIONS`)
- **Traits**: `PascalCase` (e.g., `DatabaseProvider`)

### Error Handling

- Use `thiserror` for typed errors
- Provide context for errors with `.context()` or custom messages
- Use `anyhow` for application-level error propagation
- Never unwrap() in production code - use `?` or proper error handling

```rust
// Good: Custom error with thiserror
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Query execution failed: {0}")]
    QueryFailed(#[from] diesel::result::Error),
}

// Usage
pub async fn connect(&self) -> Result<Connection, ProviderError> {
    let conn = self.pool.get().await?;
    Ok(Connection::new(conn))
}
```

### Async/Await Patterns

- Use `async fn` for I/O operations
- Prefer `.await` over blocking calls
- Use `tokio::sync` types for concurrent access
- Be mindful of cancel safety

```rust
// Good: Async with proper error handling
pub async fn get_table_metadata(
    &self,
    table_name: &str,
) -> Result<TableMetadata, ProviderError> {
    let query = schema::table.filter(schema::name.eq(table_name));
    let result = query.first::<TableSchema>(&mut self.conn).await?;
    Ok(TableMetadata::from(result))
}
```

### Traits and Generics

- Use traits for shared behavior across providers
- Prefer `async_trait` for async trait methods
- Use generic bounds sparingly and clearly
- Document trait requirements

```rust
// Good: Database provider trait
#[async_trait]
pub trait DatabaseProvider: Send + Sync {
    async fn connect(&self, config: &ConnectionConfig) -> Result<Box<dyn Connection>, ProviderError>;
    async fn get_tables(&self) -> Result<Vec<Table>, ProviderError>;
    async fn get_columns(&self, table: &str) -> Result<Vec<Column>, ProviderError>;
}
```

### Structs and Enums

- Derive common traits: `Debug`, `Clone`, `Serialize`, `Deserialize`
- Use `#[serde(rename_all = "camelCase")]` for frontend compatibility
- Prefer enums over string constants
- Use builder pattern for complex construction

```rust
// Good: Well-annotated struct
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableMetadata {
    pub name: String,
    pub row_count: i64,
    pub table_type: TableType,
    pub columns: Vec<ColumnMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TableType {
    Table,
    View,
    Junction,
    Audit,
}
```

### Database Operations (Diesel)

- Use the query DSL for type-safe queries
- Prepare queries when used multiple times
- Use transactions for multi-step operations
- Filter at database level, not in Rust

```rust
// Good: Type-safe Diesel query
use schema::tables::dsl::*;

pub fn get_workspace(id: &str) -> Result<Workspace, DbError> {
    let result = tables
        .filter(uuid.eq(id))
        .first::<Workspace>(&mut conn)?;
    Ok(result)
}
```

### Testing

- Write unit tests in the same module with `#[cfg(test)]`
- Use `tempfile` for test databases
- Test error cases, not just happy paths
- Mock external dependencies (SSH, network)
- Aim for high test coverage

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_create_workspace() {
        let temp = NamedTempFile::new().unwrap();
        let result = create_workspace(temp.path()).await;
        assert!(result.is_ok());
    }
}
```

### Tauri Commands

- Use `#[tauri::command]` macro for exported functions
- Return `Result<T, E>` for error propagation
- Use JSON-serializable types only
- Keep commands focused and testable
- Use the connection manager for database access

```rust
#[tauri::command]
pub async fn get_schema_metadata_command(
    workspace_id: String,
    state: State<'_, ConnectionManager>,
) -> Result<SchemaSnapshot, String> {
    let provider = state.get_provider(&workspace_id)
        .await
        .map_err(|e| e.to_string())?;

    let metadata = provider.get_schema_metadata()
        .await
        .map_err(|e| e.to_string())?;

    Ok(metadata)
}
```

### Security

- Use SQL parameterization, never string concatenation
- Validate and sanitize user input
- Use keyring for credential storage (keyring crate)
- Zeroize sensitive data in memory (zeroize crate)
- Never log credentials or sensitive data
- Use read-only connections where possible

```rust
// Good: Parameterized query
pub async fn get_table_by_name(name: &str) -> Result<Table, Error> {
    tables
        .filter(table_name.eq(name))  // Safe parameterization
        .first(&mut conn)
        .await
}

// Good: Zeroize sensitive data
use zeroize::Zeroize;

fn process_password(mut password: String) {
    // Use password
    do_something(&password);
    // Zero out memory
    password.zeroize();
}
```

### Concurrency

- Use `Arc<T>` for shared read-only state
- Use `Arc<Mutex<T>>` or `Arc<RwLock<T>>` for shared mutable state
- Prefer channels over shared state when possible
- Use `tokio::sync` types for async synchronization

```rust
// Good: Thread-safe shared state
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ConnectionManager {
    providers: Arc<RwLock<HashMap<String, Box<dyn DatabaseProvider>>>>,
}
```

## File Organization

```
src-tauri/src/
├── ai/              # AI integration (providers, agents, prompts, cache)
├── commands/        # Tauri IPC commands
├── database/        # Database layer (providers, models, migrations)
│   ├── providers/   # Database provider implementations
│   └── models/      # Diesel ORM models
├── services/        # Business logic services
├── plugins/         # Tauri plugins
├── lib.rs           # Library entry point, app setup
└── main.rs          # Binary entry point
```

## Common Patterns

### Provider Pattern

All database providers implement the `DatabaseProvider` trait for consistent behavior across SQLite, PostgreSQL, and MySQL.

### Error Context

Always provide context when propagating errors:

```rust
let result = operation()
    .context("Failed to fetch table metadata")?;
```

### Resource Cleanup

Use RAII patterns and ensure cleanup happens:

```rust
// Use Drop trait or explicit cleanup
impl Drop for Connection {
    fn drop(&mut self) {
        // Cleanup resources
    }
}
```

## Documentation

- Document public APIs with `///` doc comments
- Include examples in documentation
- Use `# Examples` section for code samples
- Document error conditions and panics

## Performance

- Use connection pooling for database access
- Cache frequently accessed data
- Use LRU cache for AI responses
- Avoid unnecessary cloning
- Profile with `cargo flamegraph` when needed
