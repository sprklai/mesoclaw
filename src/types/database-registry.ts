/**
 * Database Type Registry
 *
 * This module defines metadata for supported database types and provides
 * a centralized registry for database-agnostic connection handling.
 *
 * With this architecture, adding a new database type requires only:
 * 1. Backend: Implement DatabaseProvider trait
 * 2. Frontend: Add entry to DATABASE_REGISTRY
 *
 * The UI automatically adapts to the new database type with no code changes!
 */

/**
 * Database categories for grouping in the selector UI
 */
export enum DatabaseCategory {
  Local = "Local",
  PostgreSQL = "PostgreSQL",
  MySQL = "MySQL",
  Document = "Document",
}

/**
 * Supported database types
 */
export enum DatabaseType {
  // Implemented
  SQLite = "SQLite",
  PostgreSQL = "PostgreSQL",
  MySQL = "MySQL",
  MongoDB = "MongoDB",

  // Coming Soon - Local
  DuckDB = "DuckDB",

  // Coming Soon - PostgreSQL Family
  Supabase = "Supabase",
  Neon = "Neon",
  CockroachDB = "CockroachDB",

  // Coming Soon - MySQL Family
  MariaDB = "MariaDB",
  PlanetScale = "PlanetScale",
  TiDB = "TiDB",
}

/**
 * Connection method types (how to reach the database)
 */
export enum ConnectionMethodType {
  DirectTcp = "DirectTcp",
  SshTunnel = "SshTunnel",
  LocalFile = "LocalFile",
  // Future connection methods:
  // ConnectionPooler = 'ConnectionPooler',
  // CloudProxy = 'CloudProxy',
  // SslCertificate = 'SslCertificate',
}

/**
 * Connection field types
 */
export type ConnectionFieldType =
  | "text"
  | "number"
  | "password"
  | "boolean"
  | "file"
  | "select"
  | "textarea";

/**
 * A single field in a database connection form
 */
export interface ConnectionField {
  /** Field name (matches config key) */
  name: string;
  /** Human-readable label */
  label: string;
  /** Field type */
  type: ConnectionFieldType;
  /** Whether the field is required */
  required?: boolean;
  /** Default value */
  default?: any;
  /** Placeholder text */
  placeholder?: string;
  /** For select fields - list of options */
  options?: string[];
  /** For file fields - accepted file patterns */
  accept?: string;
  /** Helper text */
  helperText?: string;
  /** Validation regex (optional) */
  pattern?: string;
}

/**
 * Database capabilities
 */
export interface DatabaseCapabilities {
  /** Supports SSH tunneling */
  supportsSsh: boolean;
  /** Supports SSL/TLS */
  supportsSsl: boolean;
  /** Supports multiple concurrent connections */
  supportsMultipleConnections: boolean;
}

/**
 * Implementation status for a database type
 */
export type DatabaseStatus = "implemented" | "coming-soon";

/**
 * Category metadata for UI rendering
 */
export interface DatabaseCategoryMetadata {
  id: DatabaseCategory;
  displayName: string;
  description: string;
  icon: string;
  sortOrder: number;
}

/**
 * Metadata for a database type
 */
export interface DatabaseTypeMetadata {
  /** Database type identifier */
  id: DatabaseType;
  /** Display name */
  displayName: string;
  /** Short description */
  description: string;
  /** Icon name (from lucide-react or similar) */
  icon: string;
  /** Default database port */
  defaultPort: number;
  /** Connection mode */
  connectionMode: "file" | "network";
  /** Available connection methods for this database */
  supportedConnectionMethods: ConnectionMethodType[];
  /** Fields required for this database */
  fields: ConnectionField[];
  /** What this database can do */
  capabilities: DatabaseCapabilities;
  /** Which category this database belongs to */
  category: DatabaseCategory;
  /** Implementation status */
  status: DatabaseStatus;
  /** For variants: which base type this uses (e.g., Supabase uses PostgreSQL) */
  baseType?: DatabaseType;
  /** Order within category */
  sortOrder?: number;
}

/**
 * Category registry for grouping databases in the UI
 */
export const DATABASE_CATEGORY_REGISTRY: Record<
  DatabaseCategory,
  DatabaseCategoryMetadata
> = {
  [DatabaseCategory.Local]: {
    id: DatabaseCategory.Local,
    displayName: "Local Databases",
    description: "File-based databases",
    icon: "hard-drive",
    sortOrder: 1,
  },
  [DatabaseCategory.PostgreSQL]: {
    id: DatabaseCategory.PostgreSQL,
    displayName: "PostgreSQL",
    description: "PostgreSQL compatible",
    icon: "database",
    sortOrder: 2,
  },
  [DatabaseCategory.MySQL]: {
    id: DatabaseCategory.MySQL,
    displayName: "MySQL",
    description: "MySQL compatible",
    icon: "database",
    sortOrder: 3,
  },
  [DatabaseCategory.Document]: {
    id: DatabaseCategory.Document,
    displayName: "Document",
    description: "Document-oriented databases",
    icon: "file-json",
    sortOrder: 4,
  },
};

/**
 * Central registry of supported database types
 *
 * To add a new database:
 * 1. Add to DatabaseType enum
 * 2. Add entry to this registry
 * 3. Update backend types if needed
 *
 * That's it! The UI will automatically support the new database.
 */
export const DATABASE_REGISTRY: Record<DatabaseType, DatabaseTypeMetadata> = {
  // ============================================
  // LOCAL DATABASES
  // ============================================
  [DatabaseType.SQLite]: {
    id: DatabaseType.SQLite,
    displayName: "SQLite",
    description: "Local file-based relational database",
    icon: "database",
    defaultPort: 0,
    connectionMode: "file",
    supportedConnectionMethods: [ConnectionMethodType.LocalFile],
    fields: [
      {
        name: "path",
        label: "Database File",
        type: "file",
        required: true,
        accept: ".sqlite,.db,.sqlite3",
        placeholder: "/path/to/database.db",
      },
      {
        name: "read_only",
        label: "Read-Only Mode",
        type: "boolean",
        default: false,
      },
    ],
    capabilities: {
      supportsSsh: false,
      supportsSsl: false,
      supportsMultipleConnections: false,
    },
    category: DatabaseCategory.Local,
    status: "implemented",
    sortOrder: 1,
  },

  [DatabaseType.DuckDB]: {
    id: DatabaseType.DuckDB,
    displayName: "DuckDB",
    description: "In-process analytical database",
    icon: "database",
    defaultPort: 0,
    connectionMode: "file",
    supportedConnectionMethods: [ConnectionMethodType.LocalFile],
    fields: [
      {
        name: "path",
        label: "Database File",
        type: "file",
        required: true,
        accept: ".duckdb,.db",
        helperText: "Path to the DuckDB database file",
      },
      {
        name: "read_only",
        label: "Read-Only Mode",
        type: "boolean",
        default: false,
        helperText: "Prevent accidental data modifications",
      },
    ],
    capabilities: {
      supportsSsh: false,
      supportsSsl: false,
      supportsMultipleConnections: false,
    },
    category: DatabaseCategory.Local,
    status: "coming-soon",
    sortOrder: 2,
  },

  // ============================================
  // POSTGRESQL FAMILY
  // ============================================
  [DatabaseType.PostgreSQL]: {
    id: DatabaseType.PostgreSQL,
    displayName: "PostgreSQL",
    description: "Enterprise-grade open-source database",
    icon: "database",
    defaultPort: 5432,
    connectionMode: "network",
    supportedConnectionMethods: [
      ConnectionMethodType.DirectTcp,
      ConnectionMethodType.SshTunnel,
    ],
    fields: [
      {
        name: "connection_string",
        label: "Connection String",
        type: "textarea",
        required: false,
        placeholder: "postgresql://username:password@localhost:5432/database",
      },
      {
        name: "host",
        label: "Host",
        type: "text",
        required: true,
        placeholder: "localhost or 192.168.1.100",
      },
      {
        name: "port",
        label: "Port",
        type: "number",
        default: 5432,
        placeholder: "5432",
      },
      {
        name: "database",
        label: "Database",
        type: "text",
        required: true,
        placeholder: "my_database",
      },
      {
        name: "username",
        label: "Username",
        type: "text",
        required: true,
        placeholder: "postgres",
      },
      {
        name: "password",
        label: "Password",
        type: "password",
        required: false,
        placeholder: "Enter password",
      },
      {
        name: "use_ssl",
        label: "Use SSL/TLS",
        type: "boolean",
        default: false,
      },
    ],
    capabilities: {
      supportsSsh: true,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
    category: DatabaseCategory.PostgreSQL,
    status: "implemented",
    sortOrder: 1,
  },

  [DatabaseType.Supabase]: {
    id: DatabaseType.Supabase,
    displayName: "Supabase",
    description: "Open-source Firebase alternative with PostgreSQL",
    icon: "database",
    defaultPort: 5432,
    connectionMode: "network",
    supportedConnectionMethods: [
      ConnectionMethodType.DirectTcp,
      ConnectionMethodType.SshTunnel,
    ],
    fields: [
      {
        name: "connection_string",
        label: "Connection String",
        type: "textarea",
        required: false,
        placeholder:
          "postgresql://postgres:[PASSWORD]@db.[PROJECT].supabase.co:5432/postgres",
      },
      {
        name: "supabase_mode",
        label: "Connection Mode",
        type: "select",
        required: false,
        default: "direct",
        options: ["direct", "pooler-session", "pooler-transaction"],
      },
      {
        name: "host",
        label: "Host",
        type: "text",
        required: true,
        placeholder: "db.xxxxxxxxxxxx.supabase.co",
      },
      {
        name: "port",
        label: "Port",
        type: "number",
        default: 5432,
        placeholder: "5432",
      },
      {
        name: "database",
        label: "Database",
        type: "text",
        required: true,
        default: "postgres",
        placeholder: "postgres",
      },
      {
        name: "username",
        label: "Username",
        type: "text",
        required: true,
        default: "postgres",
        placeholder: "postgres or postgres.[PROJECT]",
      },
      {
        name: "password",
        label: "Password",
        type: "password",
        required: true,
        placeholder: "Enter database password",
      },
      {
        name: "use_ssl",
        label: "Use SSL/TLS",
        type: "boolean",
        default: true,
      },
    ],
    capabilities: {
      supportsSsh: true,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
    category: DatabaseCategory.PostgreSQL,
    status: "implemented",
    baseType: DatabaseType.PostgreSQL,
    sortOrder: 2,
  },

  [DatabaseType.Neon]: {
    id: DatabaseType.Neon,
    displayName: "Neon",
    description: "Serverless PostgreSQL with branching",
    icon: "database",
    defaultPort: 5432,
    connectionMode: "network",
    supportedConnectionMethods: [ConnectionMethodType.DirectTcp],
    fields: [
      {
        name: "connection_string",
        label: "Connection String",
        type: "textarea",
        required: false,
        placeholder:
          "postgresql://user:password@ep-xxx-xxx-123456.us-east-2.aws.neon.tech/neondb?sslmode=require",
        helperText: "Find this in Neon Console → Connection Details",
      },
      {
        name: "host",
        label: "Host",
        type: "text",
        required: true,
        placeholder: "ep-xxx-xxx-123456.us-east-2.aws.neon.tech",
        helperText: "Neon endpoint hostname",
      },
      {
        name: "port",
        label: "Port",
        type: "number",
        default: 5432,
        placeholder: "5432",
      },
      {
        name: "database",
        label: "Database",
        type: "text",
        required: true,
        default: "neondb",
        placeholder: "neondb",
        helperText: "Default database is neondb",
      },
      {
        name: "username",
        label: "Username",
        type: "text",
        required: true,
        placeholder: "neondb_owner",
      },
      {
        name: "password",
        label: "Password",
        type: "password",
        required: true,
        placeholder: "Enter password",
      },
      {
        name: "use_ssl",
        label: "Use SSL/TLS",
        type: "boolean",
        default: true,
        helperText: "SSL is required for Neon connections",
      },
    ],
    capabilities: {
      supportsSsh: false,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
    category: DatabaseCategory.PostgreSQL,
    status: "implemented",
    baseType: DatabaseType.PostgreSQL,
    sortOrder: 3,
  },

  [DatabaseType.CockroachDB]: {
    id: DatabaseType.CockroachDB,
    displayName: "CockroachDB",
    description: "Distributed SQL database",
    icon: "database",
    defaultPort: 26257,
    connectionMode: "network",
    supportedConnectionMethods: [
      ConnectionMethodType.DirectTcp,
      ConnectionMethodType.SshTunnel,
    ],
    fields: [
      {
        name: "connection_string",
        label: "Connection String",
        type: "textarea",
        required: false,
        placeholder:
          "postgresql://user:password@free-tier.gcp-us-central1.cockroachlabs.cloud:26257/defaultdb?sslmode=verify-full",
        helperText: "Find this in CockroachDB Cloud Console",
      },
      {
        name: "host",
        label: "Host",
        type: "text",
        required: true,
        placeholder: "localhost or cluster.cockroachlabs.cloud",
        helperText:
          "Use localhost for Docker, cloud hostname for CockroachDB Cloud",
      },
      {
        name: "port",
        label: "Port",
        type: "number",
        default: 26257,
        placeholder: "26257",
        helperText: "Default CockroachDB port is 26257",
      },
      {
        name: "database",
        label: "Database",
        type: "text",
        required: true,
        default: "defaultdb",
        placeholder: "defaultdb",
      },
      {
        name: "username",
        label: "Username",
        type: "text",
        required: true,
        placeholder: "root",
        helperText:
          "Use 'root' for local Docker, cloud username for CockroachDB Cloud",
      },
      {
        name: "password",
        label: "Password",
        type: "password",
        required: false,
        placeholder: "Enter password (optional for local)",
      },
      {
        name: "use_ssl",
        label: "Use SSL/TLS",
        type: "boolean",
        default: false,
        helperText: "Required for CockroachDB Cloud, optional for local Docker",
      },
    ],
    capabilities: {
      supportsSsh: true,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
    category: DatabaseCategory.PostgreSQL,
    status: "implemented",
    baseType: DatabaseType.PostgreSQL,
    sortOrder: 4,
  },

  // ============================================
  // MYSQL FAMILY
  // ============================================
  [DatabaseType.MySQL]: {
    id: DatabaseType.MySQL,
    displayName: "MySQL",
    description: "Popular open-source database",
    icon: "database",
    defaultPort: 3306,
    connectionMode: "network",
    supportedConnectionMethods: [
      ConnectionMethodType.DirectTcp,
      ConnectionMethodType.SshTunnel,
    ],
    fields: [
      {
        name: "connection_string",
        label: "Connection String",
        type: "textarea",
        required: false,
        placeholder: "mysql://username:password@localhost:3306/database",
      },
      {
        name: "host",
        label: "Host",
        type: "text",
        required: true,
        placeholder: "localhost or 192.168.1.100",
      },
      {
        name: "port",
        label: "Port",
        type: "number",
        default: 3306,
        placeholder: "3306",
      },
      {
        name: "database",
        label: "Database",
        type: "text",
        required: true,
        placeholder: "my_database",
      },
      {
        name: "username",
        label: "Username",
        type: "text",
        required: true,
        placeholder: "root",
      },
      {
        name: "password",
        label: "Password",
        type: "password",
        required: false,
        placeholder: "Enter password",
      },
      {
        name: "use_ssl",
        label: "Use SSL/TLS",
        type: "boolean",
        default: false,
      },
    ],
    capabilities: {
      supportsSsh: true,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
    category: DatabaseCategory.MySQL,
    status: "implemented",
    sortOrder: 1,
  },

  [DatabaseType.MariaDB]: {
    id: DatabaseType.MariaDB,
    displayName: "MariaDB",
    description: "Community MySQL fork with enhanced features",
    icon: "database",
    defaultPort: 3306,
    connectionMode: "network",
    supportedConnectionMethods: [
      ConnectionMethodType.DirectTcp,
      ConnectionMethodType.SshTunnel,
    ],
    fields: [
      {
        name: "connection_string",
        label: "Connection String",
        type: "textarea",
        required: false,
        placeholder: "mysql://username:password@localhost:3306/database",
        helperText: "MariaDB uses MySQL-compatible connection strings",
      },
      {
        name: "host",
        label: "Host",
        type: "text",
        required: true,
        placeholder: "localhost or 192.168.1.100",
      },
      {
        name: "port",
        label: "Port",
        type: "number",
        default: 3306,
        placeholder: "3306",
      },
      {
        name: "database",
        label: "Database",
        type: "text",
        required: true,
        placeholder: "my_database",
      },
      {
        name: "username",
        label: "Username",
        type: "text",
        required: true,
        placeholder: "root",
      },
      {
        name: "password",
        label: "Password",
        type: "password",
        required: false,
        placeholder: "Enter password",
      },
      {
        name: "use_ssl",
        label: "Use SSL/TLS",
        type: "boolean",
        default: false,
      },
    ],
    capabilities: {
      supportsSsh: true,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
    category: DatabaseCategory.MySQL,
    status: "implemented",
    baseType: DatabaseType.MySQL,
    sortOrder: 2,
  },

  [DatabaseType.PlanetScale]: {
    id: DatabaseType.PlanetScale,
    displayName: "PlanetScale",
    description: "MySQL serverless platform with branching",
    icon: "database",
    defaultPort: 3306,
    connectionMode: "network",
    supportedConnectionMethods: [ConnectionMethodType.DirectTcp],
    fields: [
      {
        name: "connection_string",
        label: "Connection String",
        type: "textarea",
        required: false,
        placeholder:
          "mysql://user:password@aws.connect.psdb.cloud/database?sslmode=verify_identity",
        helperText: "Find this in PlanetScale Console → Connect → General",
      },
      {
        name: "host",
        label: "Host",
        type: "text",
        required: true,
        placeholder: "aws.connect.psdb.cloud",
        helperText: "PlanetScale connection host",
      },
      {
        name: "port",
        label: "Port",
        type: "number",
        default: 3306,
        placeholder: "3306",
      },
      {
        name: "database",
        label: "Database",
        type: "text",
        required: true,
        placeholder: "my_database",
      },
      {
        name: "username",
        label: "Username",
        type: "text",
        required: true,
        placeholder: "branch_username",
        helperText: "Generated username for your branch",
      },
      {
        name: "password",
        label: "Password",
        type: "password",
        required: true,
        placeholder: "Enter password",
        helperText: "Generated password for your branch",
      },
      {
        name: "use_ssl",
        label: "Use SSL/TLS",
        type: "boolean",
        default: true,
        helperText: "SSL is required for PlanetScale connections",
      },
    ],
    capabilities: {
      supportsSsh: false,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
    category: DatabaseCategory.MySQL,
    status: "implemented",
    baseType: DatabaseType.MySQL,
    sortOrder: 3,
  },

  [DatabaseType.TiDB]: {
    id: DatabaseType.TiDB,
    displayName: "TiDB",
    description: "Distributed MySQL-compatible",
    icon: "database",
    defaultPort: 4000,
    connectionMode: "network",
    supportedConnectionMethods: [ConnectionMethodType.DirectTcp],
    fields: [],
    capabilities: {
      supportsSsh: false,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
    category: DatabaseCategory.MySQL,
    status: "coming-soon",
    baseType: DatabaseType.MySQL,
    sortOrder: 4,
  },

  // ============================================
  // DOCUMENT DATABASES
  // ============================================
  [DatabaseType.MongoDB]: {
    id: DatabaseType.MongoDB,
    displayName: "MongoDB",
    description: "Document-oriented NoSQL database",
    icon: "file-json",
    defaultPort: 27017,
    connectionMode: "network",
    supportedConnectionMethods: [
      ConnectionMethodType.DirectTcp,
      ConnectionMethodType.SshTunnel,
    ],
    fields: [
      {
        name: "connection_string",
        label: "Connection String",
        type: "textarea",
        required: false,
        placeholder: "mongodb://username:password@localhost:27017/database",
      },
      {
        name: "host",
        label: "Host",
        type: "text",
        required: true,
        placeholder: "localhost or cluster.mongodb.net",
      },
      {
        name: "port",
        label: "Port",
        type: "number",
        default: 27017,
        placeholder: "27017",
      },
      {
        name: "database",
        label: "Database",
        type: "text",
        required: true,
        placeholder: "my_database",
      },
      {
        name: "auth_database",
        label: "Auth Database",
        type: "text",
        required: false,
        default: "admin",
        placeholder: "admin",
      },
      {
        name: "username",
        label: "Username",
        type: "text",
        required: false,
        placeholder: "mongo_user",
      },
      {
        name: "password",
        label: "Password",
        type: "password",
        required: false,
        placeholder: "Enter password",
      },
      {
        name: "auth_mechanism",
        label: "Auth Mechanism",
        type: "select",
        required: false,
        default: "SCRAM-SHA-256",
        options: ["SCRAM-SHA-256", "SCRAM-SHA-1", "MONGODB-X509", "PLAIN"],
      },
      {
        name: "use_tls",
        label: "Use TLS",
        type: "boolean",
        default: false,
      },
    ],
    capabilities: {
      supportsSsh: true,
      supportsSsl: true,
      supportsMultipleConnections: true,
    },
    category: DatabaseCategory.Document,
    status: "implemented",
    sortOrder: 1,
  },
};

/**
 * Get metadata for a database type
 */
export function getDatabaseTypeMetadata(
  dbType: DatabaseType
): DatabaseTypeMetadata {
  return DATABASE_REGISTRY[dbType];
}

/**
 * Get all supported database types
 */
export function getSupportedDatabaseTypes(): DatabaseType[] {
  return Object.keys(DATABASE_REGISTRY) as DatabaseType[];
}

/**
 * Get default port for a database type
 */
export function getDefaultPort(dbType: DatabaseType): number {
  return DATABASE_REGISTRY[dbType].defaultPort;
}

/**
 * Check if a database type supports a specific connection method
 */
export function supportsConnectionMethod(
  dbType: DatabaseType,
  method: ConnectionMethodType
): boolean {
  return DATABASE_REGISTRY[dbType].supportedConnectionMethods.includes(method);
}

/**
 * Check if a database type supports SSH tunneling
 */
export function supportsSshTunnel(dbType: DatabaseType): boolean {
  return DATABASE_REGISTRY[dbType].capabilities.supportsSsh;
}

/**
 * Check if a database type is file-based (vs network-based)
 */
export function isFileBased(dbType: DatabaseType): boolean {
  return DATABASE_REGISTRY[dbType].connectionMode === "file";
}

/**
 * Get connection fields for a database type
 */
export function getConnectionFields(dbType: DatabaseType): ConnectionField[] {
  return DATABASE_REGISTRY[dbType].fields;
}

/**
 * Get supported connection methods for a database type
 */
export function getSupportedConnectionMethods(
  dbType: DatabaseType
): ConnectionMethodType[] {
  return DATABASE_REGISTRY[dbType].supportedConnectionMethods;
}

// ============================================
// CATEGORY HELPER FUNCTIONS
// ============================================

/**
 * Get all categories sorted by sortOrder
 */
export function getSortedCategories(): DatabaseCategory[] {
  return Object.values(DATABASE_CATEGORY_REGISTRY)
    .sort((a, b) => a.sortOrder - b.sortOrder)
    .map((cat) => cat.id);
}

/**
 * Get databases by category, sorted by sortOrder
 */
export function getDatabasesByCategory(
  category: DatabaseCategory
): DatabaseTypeMetadata[] {
  return Object.values(DATABASE_REGISTRY)
    .filter((db) => db.category === category)
    .sort((a, b) => (a.sortOrder ?? 0) - (b.sortOrder ?? 0));
}

/**
 * Check if a database is implemented (vs coming soon)
 */
export function isDatabaseImplemented(dbType: DatabaseType): boolean {
  return DATABASE_REGISTRY[dbType].status === "implemented";
}

/**
 * Get category metadata
 */
export function getCategoryMetadata(
  category: DatabaseCategory
): DatabaseCategoryMetadata {
  return DATABASE_CATEGORY_REGISTRY[category];
}

/**
 * Get all implemented database types (excludes coming-soon)
 */
export function getImplementedDatabaseTypes(): DatabaseType[] {
  return Object.values(DATABASE_REGISTRY)
    .filter((db) => db.status === "implemented")
    .map((db) => db.id);
}
