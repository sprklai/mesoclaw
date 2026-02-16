/**
 * Generic Connection String Parser
 *
 * Database-agnostic utility for parsing connection strings and extracting
 * connection parameters. Supports PostgreSQL, MySQL, MongoDB, and variants.
 */

import { invoke } from "@tauri-apps/api/core";

import { DatabaseType } from "@/types/database-registry";

/**
 * Backend response from parse_mongodb_connection_string_command.
 * This matches the actual Rust struct with serde(rename_all = "camelCase").
 */
interface MongoDBParsedConnectionStringResponse {
  valid: boolean;
  host: string | null;
  port: number | null;
  database: string | null;
  username: string | null;
  requiresAuth: boolean;
  useTls: boolean;
  authMechanism: string | null;
  replicaSet: string | null;
  authSource: string | null;
  directConnection: boolean;
  error: string | null;
}

/**
 * Generic parsed connection result that can be mapped to form fields
 */
export interface ParsedConnectionResult {
  /** Whether parsing was successful */
  valid: boolean;
  /** Error message if parsing failed */
  error?: string;
  /** Parsed field values to merge into form config */
  fields: Record<string, unknown>;
}

/**
 * Parse a PostgreSQL/Supabase connection string.
 * Supports standard PostgreSQL URIs and Supabase-specific formats.
 */
function parsePostgreSQLConnectionString(
  connectionString: string
): ParsedConnectionResult {
  if (!connectionString?.trim()) {
    return { valid: false, error: "Empty connection string", fields: {} };
  }

  try {
    const url = new URL(connectionString.trim());

    // Must be postgres/postgresql protocol
    if (!url.protocol.startsWith("postgres")) {
      return {
        valid: false,
        error: "Invalid protocol. Expected postgresql:// or postgres://",
        fields: {},
      };
    }

    const host = url.hostname;
    const port = Number.parseInt(url.port, 10) || 5432;
    const database = url.pathname.replace("/", "") || "postgres";
    const password = url.password
      ? decodeURIComponent(url.password)
      : undefined;
    let username = decodeURIComponent(url.username) || "postgres";

    // Check for Supabase pooler format (username contains project ID)
    const isPooler = host.includes("pooler.supabase.com");
    if (isPooler && username.includes(".")) {
      // Extract base username for pooler format (postgres.PROJECT -> postgres)
      username = username.split(".")[0];
    }

    // Check for SSL parameter in query string
    const sslMode = url.searchParams.get("sslmode");
    const useSsl =
      sslMode === "require" ||
      sslMode === "verify-ca" ||
      sslMode === "verify-full" ||
      host.includes(".supabase.co") ||
      host.includes(".supabase.com");

    // Detect Supabase mode from port
    let supabaseMode: string | undefined;
    if (
      host.includes(".supabase.co") ||
      host.includes(".supabase.com") ||
      host.includes("pooler.supabase.com")
    ) {
      if (port === 6543) {
        supabaseMode = "pooler-session";
      } else if (port === 5433) {
        supabaseMode = "pooler-transaction";
      } else {
        supabaseMode = "direct";
      }
    }

    return {
      valid: true,
      fields: {
        host,
        port,
        database,
        username,
        password,
        use_ssl: useSsl,
        ...(supabaseMode && { supabase_mode: supabaseMode }),
      },
    };
  } catch (error) {
    return {
      valid: false,
      error: `Invalid connection string format: ${error instanceof Error ? error.message : "Unknown error"}`,
      fields: {},
    };
  }
}

/**
 * Parse a MySQL connection string.
 * Supports standard MySQL URIs: mysql://user:pass@host:port/database
 */
function parseMySQLConnectionString(
  connectionString: string
): ParsedConnectionResult {
  if (!connectionString?.trim()) {
    return { valid: false, error: "Empty connection string", fields: {} };
  }

  try {
    const url = new URL(connectionString.trim());

    // Must be mysql protocol
    if (!url.protocol.startsWith("mysql")) {
      return {
        valid: false,
        error: "Invalid protocol. Expected mysql://",
        fields: {},
      };
    }

    const host = url.hostname;
    const port = Number.parseInt(url.port, 10) || 3306;
    const database = url.pathname.replace("/", "") || "";
    const password = url.password
      ? decodeURIComponent(url.password)
      : undefined;
    const username = decodeURIComponent(url.username) || "root";

    // Check for SSL parameter
    const ssl = url.searchParams.get("ssl");
    const useSsl = ssl === "true" || ssl === "required";

    return {
      valid: true,
      fields: {
        host,
        port,
        database,
        username,
        password,
        use_ssl: useSsl,
      },
    };
  } catch (error) {
    return {
      valid: false,
      error: `Invalid connection string format: ${error instanceof Error ? error.message : "Unknown error"}`,
      fields: {},
    };
  }
}

/**
 * Parse a MongoDB connection string using the backend parser.
 * Supports mongodb:// and mongodb+srv:// formats.
 */
async function parseMongoDBConnectionString(
  connectionString: string
): Promise<ParsedConnectionResult> {
  if (!connectionString?.trim()) {
    return { valid: false, error: "Empty connection string", fields: {} };
  }

  try {
    const parsed = await invoke<MongoDBParsedConnectionStringResponse>(
      "parse_mongodb_connection_string_command",
      { connectionString }
    );

    // Check if parsing was successful
    if (!parsed.valid) {
      return {
        valid: false,
        error: parsed.error || "Invalid connection string",
        fields: {},
      };
    }

    // Check if we got host data
    if (!parsed.host) {
      return {
        valid: false,
        error: "No host found in connection string",
        fields: {},
      };
    }

    return {
      valid: true,
      fields: {
        host: parsed.host,
        port: parsed.port || 27017,
        database: parsed.database,
        username: parsed.username,
        auth_database: parsed.authSource,
        use_tls: parsed.useTls,
        auth_mechanism: parsed.authMechanism,
      },
    };
  } catch (error) {
    return {
      valid: false,
      error: `Failed to parse MongoDB connection string: ${error instanceof Error ? error.message : "Unknown error"}`,
      fields: {},
    };
  }
}

/**
 * Parse a connection string for any supported database type.
 *
 * @param databaseType - The type of database
 * @param connectionString - The connection string to parse
 * @returns Parsed connection result with field values
 */
export async function parseConnectionString(
  databaseType: DatabaseType,
  connectionString: string
): Promise<ParsedConnectionResult> {
  if (!connectionString?.trim()) {
    return { valid: false, error: "Empty connection string", fields: {} };
  }

  switch (databaseType) {
    case DatabaseType.PostgreSQL:
    case DatabaseType.Supabase:
    case DatabaseType.Neon:
    case DatabaseType.CockroachDB:
      return parsePostgreSQLConnectionString(connectionString);

    case DatabaseType.MySQL:
    case DatabaseType.MariaDB:
    case DatabaseType.PlanetScale:
    case DatabaseType.TiDB:
      return parseMySQLConnectionString(connectionString);

    case DatabaseType.MongoDB:
      return parseMongoDBConnectionString(connectionString);

    default:
      return {
        valid: false,
        error: `Connection string parsing not supported for ${databaseType}`,
        fields: {},
      };
  }
}

/**
 * Check if a database type supports connection string parsing.
 */
export function supportsConnectionString(databaseType: DatabaseType): boolean {
  const supportedTypes: DatabaseType[] = [
    DatabaseType.PostgreSQL,
    DatabaseType.Supabase,
    DatabaseType.Neon,
    DatabaseType.CockroachDB,
    DatabaseType.MySQL,
    DatabaseType.MariaDB,
    DatabaseType.PlanetScale,
    DatabaseType.TiDB,
    DatabaseType.MongoDB,
  ];
  return supportedTypes.includes(databaseType);
}

/**
 * Get placeholder text for connection string input based on database type.
 */
export function getConnectionStringPlaceholder(
  databaseType: DatabaseType
): string {
  switch (databaseType) {
    case DatabaseType.PostgreSQL:
      return "postgresql://username:password@localhost:5432/database";
    case DatabaseType.Supabase:
      return "postgresql://postgres:[PASSWORD]@db.[PROJECT].supabase.co:5432/postgres";
    case DatabaseType.MySQL:
    case DatabaseType.MariaDB:
      return "mysql://username:password@localhost:3306/database";
    case DatabaseType.MongoDB:
      return "mongodb://username:password@localhost:27017/database";
    default:
      return "";
  }
}
