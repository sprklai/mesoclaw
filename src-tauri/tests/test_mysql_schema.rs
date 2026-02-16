// Integration test for MySQL schema loading
// Run with: cargo test --test test_mysql_schema -- --ignored

#[cfg(test)]
mod tests {
    use local_ts_lib::database::config::{ConnectionConfig, MySQLConfig};
    use local_ts_lib::database::providers::mysql::MySQLProvider;
    use local_ts_lib::database::provider::DatabaseProvider;

    #[tokio::test]
    #[ignore = "Requires MySQL Docker container"]
    async fn test_mysql_get_tables() {
        let mut provider = MySQLProvider::new();

        let config = ConnectionConfig::MySQL(MySQLConfig {
            host: "127.0.0.1".to_string(),
            port: 3307,
            database: "testdb".to_string(),
            username: "testuser".to_string(),
            password: Some("testpass".to_string()),
            use_ssl: false,
            ssh_tunnel: None,
        });

        // Test connection
        provider
            .connect(config)
            .await
            .expect("Failed to connect to MySQL");

        // Test get_tables
        let tables = provider
            .get_tables()
            .await
            .expect("Failed to get tables");

        println!("Found {} tables:", tables.len());
        for table in &tables {
            println!(
                "  - {} (type: {:?}, rows: {:?})",
                table.name, table.table_type, table.row_count
            );
        }

        // Should have 11 tables
        assert_eq!(tables.len(), 11, "Expected 11 tables in testdb");

        // Check for expected tables
        let table_names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
        assert!(table_names.contains(&"users"), "Missing 'users' table");
        assert!(table_names.contains(&"posts"), "Missing 'posts' table");
        assert!(table_names.contains(&"comments"), "Missing 'comments' table");

        // Test get_columns for a specific table
        let columns = provider
            .get_columns("users")
            .await
            .expect("Failed to get columns for users table");

        println!("\n'users' table has {} columns:", columns.len());
        for column in &columns {
            println!(
                "  - {} ({}, nullable: {}, pk: {})",
                column.name,
                column.data_type,
                column.is_nullable,
                column.is_primary_key
            );
        }

        // Verify expected columns
        assert!(!columns.is_empty(), "users table should have columns");
        let column_names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
        assert!(column_names.contains(&"id"), "Missing 'id' column");
        assert!(column_names.contains(&"email"), "Missing 'email' column");
        assert!(column_names.contains(&"username"), "Missing 'username' column");

        provider
            .disconnect()
            .await
            .expect("Failed to disconnect");
    }

    #[tokio::test]
    #[ignore = "Requires MySQL Docker container"]
    async fn test_mysql_table_stats() {
        let mut provider = MySQLProvider::new();

        let config = ConnectionConfig::MySQL(MySQLConfig {
            host: "127.0.0.1".to_string(),
            port: 3307,
            database: "testdb".to_string(),
            username: "testuser".to_string(),
            password: Some("testpass".to_string()),
            use_ssl: false,
            ssh_tunnel: None,
        });

        provider
            .connect(config)
            .await
            .expect("Failed to connect to MySQL");

        // Test get_table_stats
        let stats = provider
            .get_table_stats("users")
            .await
            .expect("Failed to get table stats");

        println!("\n'users' table stats:");
        println!("  - Row count: {}", stats.row_count);
        println!("  - Size: {:?}", stats.size_bytes);

        assert!(stats.row_count > 0, "users table should have rows");

        provider
            .disconnect()
            .await
            .expect("Failed to disconnect");
    }
}
