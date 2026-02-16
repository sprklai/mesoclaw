export interface Workspace {
  id: string;
  name: string;
  database_type: string;
  connection_config: ConnectionConfig;
  created_at: string;
  last_accessed: string;
  llm_provider_id?: string | null;
  llm_model_id?: string | null;
}

// SSH authentication types
export interface SshAuthPassword {
  type: "Password";
  password: string;
}

export interface SshAuthPrivateKey {
  type: "PrivateKey";
  key_path: string;
  passphrase?: string;
}

export type SshAuth = SshAuthPassword | SshAuthPrivateKey;

// SSH tunnel configuration
export interface SshConfig {
  host: string;
  port: number;
  username: string;
  auth: SshAuth;
  local_port?: number;
}

/**
 * MongoDB authentication mechanism.
 */
export type MongoAuthMechanism =
  | "SCRAM_SHA_1"
  | "SCRAM_SHA_256"
  | "MONGODB_X509"
  | "GSSAPI"
  | "PLAIN";

/**
 * MongoDB connection configuration.
 */
export interface MongoDBConfigData {
  /** MongoDB connection URI (mongodb:// or mongodb+srv://) */
  uri: string;
  /** Target database name */
  database: string;
  /** Authentication database (defaults to "admin") */
  authDatabase?: string;
  /** Username for authentication */
  username?: string;
  /** Authentication mechanism */
  authMechanism?: MongoAuthMechanism;
  /** Enable TLS/SSL */
  useTls?: boolean;
  /** Allow invalid TLS certificates (development only) */
  tlsAllowInvalidCertificates?: boolean;
  /** Optional SSH tunnel configuration */
  sshTunnel?: SshConfig;
  /** Connection timeout in milliseconds */
  connectTimeoutMs?: number;
}

export interface ConnectionConfig {
  SQLite?: {
    path: string;
    read_only: boolean;
  };
  PostgreSQL?: {
    host: string;
    port: number;
    database: string;
    username: string;
    ssh_tunnel?: SshConfig;
  };
  MySQL?: {
    host: string;
    port: number;
    database: string;
    username: string;
    ssh_tunnel?: SshConfig;
  };
  MongoDB?: MongoDBConfigData;
}

export interface ConnectionTestResult {
  success: boolean;
  error?: string;
  metadata?: ConnectionMetadata;
}

export interface ConnectionMetadata {
  table_count: number;
  file_size: number;
}

export interface WorkspaceResponse {
  id: string;
  name: string;
  database_type: string;
  connection_config: Record<string, unknown>;
  created_at: string;
  last_accessed: string;
  llm_provider_id?: string | null;
  llm_model_id?: string | null;
}

// ============================================
// MONGODB-SPECIFIC TYPES
// ============================================

/**
 * Parsed MongoDB connection string result.
 * Returned by parse_mongodb_connection_string_command.
 */
export interface ParsedConnectionString {
  /** Parsed hosts (hostname, port pairs) */
  hosts: Array<{ host: string; port: number }>;
  /** Database name from connection string */
  database?: string;
  /** Username from connection string */
  username?: string;
  /** Authentication source database */
  authSource?: string;
  /** Replica set name */
  replicaSet?: string;
  /** Whether TLS is enabled */
  tls: boolean;
  /** Authentication mechanism */
  authMechanism?: MongoAuthMechanism;
  /** Additional connection options */
  options: Record<string, string>;
}

/**
 * Request for testing MongoDB connection.
 */
export interface MongoDBConnectionTestRequest {
  /** Host to connect to */
  host: string;
  /** Port number */
  port: number;
  /** Database name */
  database: string;
  /** Username (optional) */
  username?: string;
  /** Password (optional) */
  password?: string;
  /** Auth database (optional) */
  authDatabase?: string;
  /** Use TLS */
  useTls?: boolean;
  /** SSH tunnel config (optional) */
  sshTunnel?: SshConfig;
}

/**
 * Response from testing MongoDB connection.
 */
export interface MongoDBConnectionTestResponse {
  /** Whether connection was successful */
  success: boolean;
  /** Error message if failed */
  error?: string;
  /** MongoDB server version */
  version?: string;
  /** Number of collections in the database */
  collectionCount?: number;
}
