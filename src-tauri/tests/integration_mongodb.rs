//! Integration tests for MongoDB provider.
//!
//! Run with: cargo test --test integration_mongodb -- --ignored
//!
//! Prerequisites:
//! 1. Start MongoDB Docker container:
//!    ```bash
//!    cd dockertests/mongodb
//!    docker-compose up -d
//!    ```
//! 2. Wait for MongoDB to initialize (10-15 seconds)
//! 3. Run tests: `cargo test --test integration_mongodb -- --ignored --nocapture`

#[cfg(test)]
mod tests {
    use local_ts_lib::database::config::ConnectionConfig;
    use local_ts_lib::database::metadata::{
        BsonType, CollectionType, ContainerMetadata, ContainerStructure,
        DatabaseParadigm, DocumentRelationshipType,
    };
    use local_ts_lib::database::mongodb_config::MongoDBConfig;
    use local_ts_lib::database::provider::DatabaseProvider;
    use local_ts_lib::database::providers::mongodb::MongoDBProvider;

    /// Default test connection configuration.
    fn test_config() -> ConnectionConfig {
        ConnectionConfig::MongoDB(
            MongoDBConfig::new("127.0.0.1", "test_database")
                .with_port(27017)
                .with_credentials("testuser", "testpass")
                .with_auth_database("admin"),
        )
    }

    // =========================================================================
    // Connection Tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_connect_and_disconnect() {
        let mut provider = MongoDBProvider::new();

        // Test connection
        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // Verify connected
        let connected = provider.test_connection().await.expect("test_connection failed");
        assert!(connected, "Should be connected after connect()");

        // Disconnect
        provider
            .disconnect()
            .await
            .expect("Failed to disconnect");

        // Verify disconnected
        let connected = provider.test_connection().await.expect("test_connection failed");
        assert!(!connected, "Should be disconnected after disconnect()");
    }

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_provider_metadata() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // Check provider type
        assert_eq!(
            provider.provider_type(),
            local_ts_lib::database::metadata::DatabaseType::MongoDB
        );

        // Check paradigm
        assert_eq!(provider.provider_paradigm(), DatabaseParadigm::Document);

        // Check version
        let version = provider.version().await.expect("Failed to get version");
        println!("MongoDB version: {}", version);
        assert!(version.starts_with("MongoDB"), "Version should start with 'MongoDB'");

        provider.disconnect().await.expect("Failed to disconnect");
    }

    // =========================================================================
    // Collection Tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_get_collections() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // Get containers (collections)
        let containers = provider
            .get_containers()
            .await
            .expect("Failed to get containers");

        println!("Found {} collections:", containers.len());
        for container in &containers {
            if let ContainerMetadata::Collection(col) = container {
                println!(
                    "  - {} (type: {:?}, docs: {:?})",
                    col.name, col.collection_type, col.estimated_document_count
                );
            }
        }

        // Should have test collections from init.js
        assert!(!containers.is_empty(), "Should have at least one collection");

        // Check for expected collections
        let collection_names: Vec<&str> = containers
            .iter()
            .filter_map(|c| {
                if let ContainerMetadata::Collection(col) = c {
                    Some(col.name.as_str())
                } else {
                    None
                }
            })
            .collect();

        assert!(
            collection_names.contains(&"users"),
            "Missing 'users' collection"
        );
        assert!(
            collection_names.contains(&"products"),
            "Missing 'products' collection"
        );
        assert!(
            collection_names.contains(&"orders"),
            "Missing 'orders' collection"
        );

        provider.disconnect().await.expect("Failed to disconnect");
    }

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_collection_types() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        let containers = provider
            .get_containers()
            .await
            .expect("Failed to get containers");

        // Find and verify capped collection (logs)
        let logs_collection = containers.iter().find_map(|c| {
            if let ContainerMetadata::Collection(col) = c {
                if col.name == "logs" {
                    return Some(col);
                }
            }
            None
        });

        if let Some(logs) = logs_collection {
            println!("logs collection: is_capped={}, type={:?}", logs.is_capped, logs.collection_type);
            assert!(logs.is_capped, "logs should be a capped collection");
            assert_eq!(logs.collection_type, CollectionType::Capped);
        } else {
            println!("Note: 'logs' collection not found (may need updated init.js)");
        }

        provider.disconnect().await.expect("Failed to disconnect");
    }

    // =========================================================================
    // Schema Inference Tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_schema_inference() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // Get container structure (which triggers schema inference)
        let structure = provider
            .get_container_structure("users")
            .await
            .expect("Failed to get container structure");

        match structure {
            ContainerStructure::Document {
                fields,
                schema_inference,
                sample_document,
            } => {
                println!("\nSchema inference for 'users':");
                println!("  - Documents sampled: {}", schema_inference.documents_sampled);
                println!("  - Confidence: {:.2}", schema_inference.confidence_score);
                println!("  - Is consistent: {}", schema_inference.is_consistent);

                println!("\nFields ({}):", fields.len());
                for field in &fields {
                    println!(
                        "  - {} (types: {:?}, presence: {:.0}%)",
                        field.name, field.data_types, field.presence_percentage
                    );
                }

                // Check for expected fields
                let field_names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
                assert!(field_names.contains(&"_id"), "Missing '_id' field");
                assert!(field_names.contains(&"name"), "Missing 'name' field");
                assert!(field_names.contains(&"email"), "Missing 'email' field");

                // Check _id field is ObjectId type
                let id_field = fields.iter().find(|f| f.name == "_id").unwrap();
                assert!(
                    id_field.data_types.contains(&BsonType::ObjectId),
                    "_id should be ObjectId"
                );

                // Check sample document is present
                if let Some(doc) = sample_document {
                    println!("\nSample document: {}", serde_json::to_string_pretty(&doc).unwrap());
                    assert!(doc.is_object(), "Sample should be an object");
                }
            }
            ContainerStructure::Relational { .. } => {
                panic!("Expected Document structure, got Relational");
            }
        }

        provider.disconnect().await.expect("Failed to disconnect");
    }

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_embedded_documents() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // products collection has embedded reviews
        let structure = provider
            .get_container_structure("products")
            .await
            .expect("Failed to get products structure");

        match structure {
            ContainerStructure::Document { fields, .. } => {
                println!("\nProduct fields:");
                for field in &fields {
                    println!(
                        "  - {} (embedded: {}, array: {})",
                        field.name, field.is_embedded_document, field.is_array
                    );
                    if let Some(ref nested) = field.nested_fields {
                        println!("    Nested: {:?}", nested);
                    }
                }

                // Check for embedded reviews array
                let reviews_field = fields.iter().find(|f| f.name == "reviews");
                if let Some(reviews) = reviews_field {
                    println!("\nReviews field: array={}", reviews.is_array);
                    if reviews.is_array {
                        if let Some(ref stats) = reviews.array_stats {
                            println!("  Array stats: min={}, max={}, avg={:.1}",
                                stats.min_length, stats.max_length, stats.avg_length);
                        }
                    }
                }
            }
            _ => panic!("Expected Document structure"),
        }

        provider.disconnect().await.expect("Failed to disconnect");
    }

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_mixed_schema_detection() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // mixed_schema collection has heterogeneous documents
        let structure = provider
            .get_container_structure("mixed_schema")
            .await
            .expect("Failed to get mixed_schema structure");

        match structure {
            ContainerStructure::Document {
                fields,
                schema_inference,
                ..
            } => {
                println!("\nMixed schema analysis:");
                println!("  - Consistent: {}", schema_inference.is_consistent);
                println!("  - Confidence: {:.2}", schema_inference.confidence_score);

                // Fields with multiple types should be detected
                for field in &fields {
                    if field.data_types.len() > 1 {
                        println!(
                            "  - {} has multiple types: {:?}",
                            field.name, field.data_types
                        );
                    }
                }

                // Check presence percentage (some fields should be <100%)
                let optional_fields: Vec<_> = fields
                    .iter()
                    .filter(|f| f.presence_percentage < 100.0)
                    .collect();

                println!(
                    "\nOptional fields (presence < 100%): {}",
                    optional_fields.len()
                );
                for field in optional_fields {
                    println!(
                        "  - {} ({:.0}%)",
                        field.name, field.presence_percentage
                    );
                }
            }
            _ => panic!("Expected Document structure"),
        }

        provider.disconnect().await.expect("Failed to disconnect");
    }

    // =========================================================================
    // Relationship Detection Tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_relationship_detection() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // Get detected relationships
        let relationships = provider
            .get_document_relationships()
            .await
            .expect("Failed to get relationships");

        println!("\nDetected relationships ({}):", relationships.len());
        for rel in &relationships {
            println!(
                "  - {}.{} -> {} ({:?}, confidence: {:.2})",
                rel.from_collection,
                rel.field_path,
                rel.to_collection,
                rel.relationship_type,
                rel.confidence
            );
        }

        // Should detect relationships from orders -> users and orders -> products
        let has_user_ref = relationships.iter().any(|r| {
            r.from_collection == "orders"
                && r.field_path.contains("user")
                && matches!(
                    r.relationship_type,
                    DocumentRelationshipType::ManualReference | DocumentRelationshipType::DbRef
                )
        });

        let has_product_ref = relationships.iter().any(|r| {
            r.from_collection == "orders"
                && r.field_path.contains("product")
        });

        println!("\nOrder references: user={}, product={}", has_user_ref, has_product_ref);

        // Check for embedded relationships
        let embedded_rels: Vec<_> = relationships
            .iter()
            .filter(|r| matches!(r.relationship_type, DocumentRelationshipType::Embedded))
            .collect();

        println!("\nEmbedded relationships ({}):", embedded_rels.len());
        for rel in embedded_rels {
            println!(
                "  - {}.{} (cardinality: {:?})",
                rel.from_collection, rel.field_path, rel.cardinality
            );
        }

        provider.disconnect().await.expect("Failed to disconnect");
    }

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_dbref_detection() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // dbref_example collection uses DBRef format
        let relationships = provider
            .get_document_relationships()
            .await
            .expect("Failed to get relationships");

        let dbref_rels: Vec<_> = relationships
            .iter()
            .filter(|r| {
                r.from_collection == "dbref_example"
                    && matches!(r.relationship_type, DocumentRelationshipType::DbRef)
            })
            .collect();

        println!("\nDBRef relationships from dbref_example:");
        for rel in &dbref_rels {
            println!(
                "  - {}.{} -> {} ({:?})",
                rel.from_collection, rel.field_path, rel.to_collection, rel.relationship_type
            );
        }

        // Should detect DBRef to users collection
        if dbref_rels.is_empty() {
            println!("Note: No DBRef relationships detected (may need updated init.js)");
        }

        provider.disconnect().await.expect("Failed to disconnect");
    }

    // =========================================================================
    // Index Tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_get_indexes() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // Get indexes for users collection
        let indexes = provider
            .get_indexes("users")
            .await
            .expect("Failed to get indexes");

        println!("\nIndexes on 'users' ({}):", indexes.len());
        for idx in &indexes {
            println!(
                "  - {} (columns: {:?}, unique: {}, primary: {})",
                idx.name, idx.columns, idx.is_unique, idx.is_primary
            );
        }

        // Should have _id index
        let has_id_index = indexes.iter().any(|i| i.name == "_id_" && i.is_primary);
        assert!(has_id_index, "Should have _id_ index");

        // Check if email has unique index (if defined in init.js)
        let email_index = indexes.iter().find(|i| i.columns.contains(&"email".to_string()));
        if let Some(idx) = email_index {
            println!("  Email index: unique={}", idx.is_unique);
        }

        provider.disconnect().await.expect("Failed to disconnect");
    }

    // =========================================================================
    // Data Query Tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_query_documents() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // Query users with limit and offset
        let data = provider
            .query_container("users", 5, 0)
            .await
            .expect("Failed to query users");

        match data {
            local_ts_lib::database::provider::ContainerData::Collection(col_data) => {
                println!("\nQueried {} documents (total: {})", col_data.documents.len(), col_data.total_count);
                println!("Fields: {:?}", col_data.field_names);

                assert!(!col_data.documents.is_empty(), "Should have documents");
                assert!(col_data.field_names.contains(&"_id".to_string()), "Should have _id field");

                // Print first document
                if let Some(first_doc) = col_data.documents.first() {
                    println!("\nFirst document: {}", serde_json::to_string_pretty(first_doc).unwrap());
                }
            }
            _ => panic!("Expected Collection data"),
        }

        provider.disconnect().await.expect("Failed to disconnect");
    }

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_query_with_pagination() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // Get table stats to know total
        let stats = provider
            .get_table_stats("users")
            .await
            .expect("Failed to get stats");

        println!("\nTotal documents in users: {}", stats.row_count);

        // Query first page
        let page1 = provider.query_container("users", 2, 0).await.expect("Failed page 1");

        // Query second page
        let page2 = provider.query_container("users", 2, 2).await.expect("Failed page 2");

        match (page1, page2) {
            (
                local_ts_lib::database::provider::ContainerData::Collection(c1),
                local_ts_lib::database::provider::ContainerData::Collection(c2),
            ) => {
                println!("Page 1: {} docs, Page 2: {} docs", c1.documents.len(), c2.documents.len());

                // Pages should be different (unless very few documents)
                if stats.row_count >= 4 {
                    // Get _id from each page
                    let id1 = c1.documents.first().and_then(|d| d.get("_id"));
                    let id2 = c2.documents.first().and_then(|d| d.get("_id"));

                    if let (Some(id1), Some(id2)) = (id1, id2) {
                        assert_ne!(id1, id2, "Page 1 and 2 should have different first documents");
                    }
                }
            }
            _ => panic!("Expected Collection data"),
        }

        provider.disconnect().await.expect("Failed to disconnect");
    }

    // =========================================================================
    // Relational Compatibility Tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_relational_api_compatibility() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // Test get_tables (compatibility layer)
        let tables = provider
            .get_tables()
            .await
            .expect("Failed to get tables");

        println!("\nRelational API - Tables ({}):", tables.len());
        for table in &tables {
            println!("  - {} (rows: {:?})", table.name, table.row_count);
        }

        assert!(!tables.is_empty(), "Should return collections as tables");

        // Test get_columns (compatibility layer)
        let columns = provider
            .get_columns("users")
            .await
            .expect("Failed to get columns");

        println!("\nRelational API - Columns for 'users' ({}):", columns.len());
        for col in &columns {
            println!(
                "  - {} ({}, nullable: {})",
                col.name, col.data_type, col.is_nullable
            );
        }

        // _id should be marked as primary key
        let id_col = columns.iter().find(|c| c.name == "_id");
        assert!(id_col.is_some(), "Should have _id column");
        assert!(id_col.unwrap().is_primary_key, "_id should be primary key");

        // Test query_table (compatibility layer)
        let table_data = provider
            .query_table("users", 3, 0)
            .await
            .expect("Failed to query table");

        println!("\nRelational API - Query result:");
        println!("  Columns: {:?}", table_data.columns);
        println!("  Rows: {}", table_data.rows.len());

        provider.disconnect().await.expect("Failed to disconnect");
    }

    // =========================================================================
    // Error Handling Tests
    // =========================================================================

    #[tokio::test]
    #[ignore = "Requires MongoDB Docker container"]
    async fn test_mongodb_invalid_collection() {
        let mut provider = MongoDBProvider::new();

        provider
            .connect(test_config())
            .await
            .expect("Failed to connect to MongoDB");

        // Query non-existent collection (MongoDB creates empty cursor, doesn't error)
        let result = provider.query_container("nonexistent_collection", 10, 0).await;

        match result {
            Ok(local_ts_lib::database::provider::ContainerData::Collection(data)) => {
                // MongoDB returns empty results for non-existent collections
                assert_eq!(data.documents.len(), 0, "Should have no documents");
            }
            Ok(_) => panic!("Unexpected data type"),
            Err(e) => println!("Error (acceptable): {}", e),
        }

        provider.disconnect().await.expect("Failed to disconnect");
    }

    #[tokio::test]
    async fn test_mongodb_operations_without_connection() {
        let provider = MongoDBProvider::new();

        // Operations should fail without connection
        let result = provider.get_tables().await;
        assert!(result.is_err(), "get_tables should fail without connection");

        let result = provider.get_containers().await;
        assert!(result.is_err(), "get_containers should fail without connection");

        let result = provider.query_container("users", 10, 0).await;
        assert!(result.is_err(), "query_container should fail without connection");
    }

    #[tokio::test]
    async fn test_mongodb_invalid_config_type() {
        use std::path::PathBuf;
        use local_ts_lib::database::config::SQLiteConfig;

        let mut provider = MongoDBProvider::new();

        // Try to connect with wrong config type
        let result = provider
            .connect(ConnectionConfig::SQLite(SQLiteConfig {
                path: PathBuf::from("/tmp/test.db"),
                read_only: false,
            }))
            .await;

        assert!(result.is_err(), "Should reject non-MongoDB config");

        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("Invalid config type"),
            "Error should mention invalid config type"
        );
    }
}
