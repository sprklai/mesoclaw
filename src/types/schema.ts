/**
 * Database schema type definitions.
 * These types mirror the Rust metadata structures from src-tauri/src/database/metadata.rs
 */

export type DatabaseType = "SQLite" | "PostgreSQL" | "MySQL" | "MongoDB";

/**
 * Database paradigm - distinguishes relational vs document databases.
 *
 * NOTE: This is different from DatabaseCategory in database-registry.ts
 * which is used for UI grouping (Local, PostgreSQL, MySQL, Document).
 * This type represents the fundamental database paradigm.
 */
export type DatabaseParadigm =
  | "relational"
  | "document"
  | "keyvalue"
  | "search"
  | "graph"
  | "timeseries";

export type TableType = "Table" | "View" | "MaterializedView";

export type ConstraintType = "PrimaryKey" | "ForeignKey" | "Unique" | "Check";

export type RelationshipType =
  | "OneToOne"
  | "OneToMany"
  | "ManyToOne"
  | "ManyToMany";

/**
 * Metadata about a database table.
 */
export interface TableMetadata {
  /** Table name */
  name: string;
  /** Schema name (if applicable, null for SQLite) */
  schema: string | null;
  /** Table type (table, view, etc.) */
  table_type: TableType;
  /** Table comment/description (if available) */
  comment: string | null;
  /** Estimated row count */
  row_count: number | null;
}

/**
 * Metadata about a table column.
 */
export interface ColumnMetadata {
  /** Column name */
  name: string;
  /** Table name this column belongs to */
  table_name: string;
  /** Column data type */
  data_type: string;
  /** Whether the column is nullable */
  is_nullable: boolean;
  /** Whether this is a primary key column */
  is_primary_key: boolean;
  /** Default value (if any) */
  default_value: string | null;
  /** Column comment/description (if available) */
  comment: string | null;
  /** Column position in the table */
  ordinal_position: number;
}

/**
 * Metadata about a table constraint.
 */
export interface ConstraintMetadata {
  /** Constraint name */
  name: string;
  /** Table name this constraint belongs to */
  table_name: string;
  /** Type of constraint */
  constraint_type: ConstraintType;
  /** Columns involved in the constraint */
  columns: string[];
  /** Referenced table (for foreign keys) */
  referenced_table: string | null;
  /** Referenced columns (for foreign keys) */
  referenced_columns: string[] | null;
}

/**
 * Metadata about a table index.
 */
export interface IndexMetadata {
  /** Index name */
  name: string;
  /** Table name this index belongs to */
  table_name: string;
  /** Whether the index is unique */
  is_unique: boolean;
  /** Columns included in the index */
  columns: string[];
  /** Whether this is a primary key index */
  is_primary: boolean;
}

/**
 * Metadata about a relationship between tables.
 */
export interface RelationshipMetadata {
  /** Unique identifier for this relationship */
  id: string;
  /** Source table name */
  from_table: string;
  /** Source column name */
  from_column: string;
  /** Target table name */
  to_table: string;
  /** Target column name */
  to_column: string;
  /** Whether this is an explicit foreign key or inferred */
  is_explicit: boolean;
  /** Relationship type */
  relationship_type: RelationshipType;
  /** Confidence score for inferred relationships (0.0 - 1.0) */
  confidence: number | null;
}

/**
 * Lightweight statistics about a table.
 */
export interface TableStats {
  /** Table name */
  table_name: string;
  /** Total number of rows */
  row_count: number;
  /** Approximate size in bytes (if available) */
  size_bytes: number | null;
  /** When the table was last modified (if available) */
  last_modified: string | null;
}

/**
 * Lightweight statistics about a column.
 */
export interface ColumnStats {
  /** Table name */
  table_name: string;
  /** Column name */
  column_name: string;
  /** Number of null values */
  null_count: number;
  /** Number of distinct values (if available) */
  distinct_count: number | null;
  /** Minimum value (if applicable) */
  min_value: string | null;
  /** Maximum value (if applicable) */
  max_value: string | null;
}

/**
 * Complete schema snapshot.
 * Supports both relational (tables) and document (collections) databases.
 */
export interface SchemaSnapshot {
  // ============================================
  // RELATIONAL DATABASE FIELDS
  // ============================================

  /** All tables in the schema (relational databases) */
  tables: TableMetadata[];
  /** All columns in the schema (relational databases) */
  columns: ColumnMetadata[];
  /** All indexes in the schema */
  indexes: IndexMetadata[];
  /** All relationships in the schema (foreign key based) */
  relationships: RelationshipMetadata[];
  /** Table statistics */
  table_stats: TableStats[];

  // ============================================
  // DOCUMENT DATABASE FIELDS (Optional)
  // ============================================

  /** All collections in the database (document databases like MongoDB) */
  collections?: CollectionMetadata[];
  /** All fields across collections (document databases) */
  fields?: FieldMetadata[];
  /** Document relationships (reference/embedded based) */
  documentRelationships?: DocumentRelationshipMetadata[];
  /** Schema inference results per collection */
  schemaInferences?:
    | Map<string, SchemaInference>
    | Record<string, SchemaInference>;
  /** Database paradigm (relational, document, etc.) */
  paradigm?: DatabaseParadigm;
}

/**
 * Detailed table information including all metadata.
 */
export interface TableDetails {
  /** Table metadata */
  table: TableMetadata;
  /** Columns in this table */
  columns: ColumnMetadata[];
  /** Indexes on this table */
  indexes: IndexMetadata[];
  /** Outgoing relationships (from this table) */
  outgoing_relationships: RelationshipMetadata[];
  /** Incoming relationships (to this table) */
  incoming_relationships: RelationshipMetadata[];
  /** Table statistics */
  stats: TableStats | null;
}

/**
 * Search filter options for schema tree.
 */
export interface SchemaFilter {
  /** Search query */
  query: string;
  /** Filter by entity types */
  entityTypes: ("table" | "view" | "index")[];
  /** Show only tables with relationships */
  onlyWithRelationships: boolean;
  /** Filter by database schema (e.g., 'public', null for all) */
  selectedSchema: string | null;
}

// ============================================
// DOCUMENT DATABASE TYPES (MongoDB, etc.)
// ============================================

/**
 * Collection type for document databases.
 */
export type CollectionType = "Collection" | "View" | "TimeSeries" | "Capped";

/**
 * Container abstraction for tables and collections.
 * Used for paradigm-agnostic database handling.
 */
export type ContainerMetadata = TableMetadata | CollectionMetadata;

/**
 * Metadata about a document collection.
 * Mirrors Rust CollectionMetadata in src-tauri/src/database/metadata.rs
 */
export interface CollectionMetadata {
  /** Collection name */
  name: string;
  /** Database name */
  database: string;
  /** Type of collection */
  collectionType: CollectionType;
  /** Estimated document count */
  estimatedDocumentCount?: number;
  /** Collection size in bytes */
  sizeBytes?: number;
  /** Average document size in bytes */
  avgDocumentSize?: number;
  /** Whether this is a capped collection */
  isCapped: boolean;
  /** Maximum size for capped collections */
  cappedMaxSize?: number;
  /** JSON schema validation rules (if any) */
  validationRules?: ValidationRule[];
  /** Collection comment/description */
  comment?: string;
}

/**
 * JSON Schema validation rule for MongoDB collections.
 */
export interface ValidationRule {
  /** Validation level (off, strict, moderate) */
  level: string;
  /** Validation action (error, warn) */
  action: string;
  /** JSON Schema validator */
  validator?: Record<string, unknown>;
}

/**
 * BSON types used in MongoDB documents.
 */
export type BsonType =
  | "Double"
  | "String"
  | "Object"
  | "Array"
  | "BinData"
  | "ObjectId"
  | "Bool"
  | "Date"
  | "Null"
  | "Regex"
  | "JavaScript"
  | "Int32"
  | "Int64"
  | "Decimal128"
  | "Timestamp"
  | "MinKey"
  | "MaxKey";

/**
 * Statistics for array fields in documents.
 */
export interface ArrayStats {
  /** Minimum array length observed */
  minLength: number;
  /** Maximum array length observed */
  maxLength: number;
  /** Average array length */
  avgLength: number;
  /** BSON types found in array elements */
  elementTypes: BsonType[];
}

/**
 * Metadata about a field in a document collection.
 * Mirrors Rust FieldMetadata in src-tauri/src/database/metadata.rs
 */
export interface FieldMetadata {
  /** Field name (dot notation for nested fields) */
  name: string;
  /** Collection this field belongs to */
  collectionName: string;
  /** BSON types observed for this field */
  dataTypes: BsonType[];
  /** Percentage of documents containing this field (0.0 - 100.0) */
  presencePercentage: number;
  /** Whether this field is indexed */
  isIndexed: boolean;
  /** Types of indexes on this field */
  indexTypes: string[];
  /** Whether this field contains arrays */
  isArray: boolean;
  /** Statistics for array fields */
  arrayStats?: ArrayStats;
  /** Whether this field contains embedded documents */
  isEmbeddedDocument: boolean;
  /** Nested field paths (for embedded documents) */
  nestedFields?: string[];
  /** Sample values from this field */
  sampleValues: unknown[];
  /** Whether this field appears to be a reference to another collection */
  isReference: boolean;
  /** Target collection if this is a reference */
  referenceTarget?: string;
}

/**
 * A document type pattern detected in a collection.
 */
export interface DocumentTypeDefinition {
  /** Identifier for this document type */
  typeId: string;
  /** Fields that define this type */
  definingFields: string[];
  /** Percentage of documents matching this type */
  percentage: number;
  /** Example document of this type */
  sampleDocument?: Record<string, unknown>;
}

/**
 * Schema inference results for a document collection.
 * Mirrors Rust SchemaInference in src-tauri/src/database/metadata.rs
 */
export interface SchemaInference {
  /** Number of documents sampled for inference */
  documentsSampled: number;
  /** Confidence score for the inferred schema (0.0 - 1.0) */
  confidenceScore: number;
  /** Whether the collection has a consistent schema */
  isConsistent: boolean;
  /** Document type patterns detected */
  documentTypes: DocumentTypeDefinition[];
}

/**
 * Container structure abstraction for tables (columns) and collections (fields).
 */
export type ContainerStructure =
  | {
      type: "relational";
      columns: ColumnMetadata[];
      constraints: ConstraintMetadata[];
    }
  | {
      type: "document";
      fields: FieldMetadata[];
      sampleDocument: Record<string, unknown>;
      schemaInference: SchemaInference;
    };

// ============================================
// DOCUMENT RELATIONSHIP TYPES
// ============================================

/**
 * Cardinality for relationships (shared between relational and document).
 */
export type Cardinality = "OneToOne" | "OneToMany" | "ManyToOne" | "ManyToMany";

/**
 * Type of relationship in document databases.
 */
export type DocumentRelationshipType =
  | "ManualReference" // Field stores ObjectId referencing another collection
  | "DbRef" // MongoDB DBRef format { $ref, $id, $db }
  | "Embedded" // Nested document(s)
  | "ArrayReference"; // Array of ObjectIds

/**
 * Structure of embedded documents.
 */
export type EmbeddedStructure = "Single" | "Array";

/**
 * Details about an embedded relationship.
 */
export interface EmbeddedRelationship {
  /** Whether single document or array of documents */
  structure: EmbeddedStructure;
  /** Average number of embedded documents (for arrays) */
  avgEmbeddedCount?: number;
}

/**
 * Metadata about a relationship in document databases.
 * Mirrors Rust DocumentRelationshipMetadata in src-tauri/src/database/metadata.rs
 */
export interface DocumentRelationshipMetadata {
  /** Source collection name */
  fromCollection: string;
  /** Target collection name */
  toCollection: string;
  /** Type of relationship */
  relationshipType: DocumentRelationshipType;
  /** Field path containing the reference */
  fieldPath: string;
  /** Confidence score for inferred relationships (0.0 - 1.0) */
  confidence: number;
  /** Relationship cardinality */
  cardinality: Cardinality;
  /** Details for embedded relationships */
  embedded?: EmbeddedRelationship;
}

/**
 * Detailed collection information including all metadata.
 */
export interface CollectionDetails {
  /** Collection metadata */
  collection: CollectionMetadata;
  /** Fields in this collection */
  fields: FieldMetadata[];
  /** Indexes on this collection */
  indexes: IndexMetadata[];
  /** Schema inference results */
  schemaInference: SchemaInference;
  /** Relationships from/to this collection */
  relationships: DocumentRelationshipMetadata[];
}
