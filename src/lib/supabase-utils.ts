/**
 * Supabase Connection Utilities
 *
 * Utilities for parsing Supabase connection strings and handling
 * different connection modes (direct, pooler-session, pooler-transaction).
 */

/**
 * Supabase connection modes
 * - direct: Direct IPv4 connection (port 5432)
 * - pooler-session: Connection pooler with session mode (port 6543)
 * - pooler-transaction: Connection pooler with transaction mode (port 5433)
 */
export type SupabaseConnectionMode =
  | "direct"
  | "pooler-session"
  | "pooler-transaction";

/**
 * Parsed Supabase connection information
 */
export interface ParsedSupabaseConnection {
  host: string;
  port: number;
  database: string;
  username: string;
  password?: string;
  mode: SupabaseConnectionMode;
  projectId: string;
  region?: string;
  useSsl: boolean;
}

/**
 * Parse a Supabase/PostgreSQL connection string into its components.
 *
 * Supports:
 * - Supabase Cloud Direct: postgresql://postgres:[PASSWORD]@db.[PROJECT].supabase.co:5432/postgres
 * - Supabase Cloud Pooler Session: postgres://postgres.[PROJECT]:[PASSWORD]@aws-0-[REGION].pooler.supabase.com:6543/postgres
 * - Supabase Cloud Pooler Transaction: postgres://postgres.[PROJECT]:[PASSWORD]@aws-0-[REGION].pooler.supabase.com:5433/postgres
 * - Local Supabase (Docker): postgresql://postgres:postgres@127.0.0.1:54322/postgres
 * - Any standard PostgreSQL connection string
 *
 * @param connectionString - The connection string to parse
 * @returns Parsed connection info or null if invalid
 */
export function parseSupabaseConnectionString(
  connectionString: string
): ParsedSupabaseConnection | null {
  if (!connectionString?.trim()) {
    return null;
  }

  try {
    const url = new URL(connectionString.trim());

    // Must be postgres/postgresql protocol
    if (!url.protocol.startsWith("postgres")) {
      return null;
    }

    const host = url.hostname;
    const port = Number.parseInt(url.port, 10) || 5432;
    const database = url.pathname.replace("/", "") || "postgres";
    const password = url.password
      ? decodeURIComponent(url.password)
      : undefined;
    const username = decodeURIComponent(url.username) || "postgres";

    // Check if this is a Supabase cloud host
    const isSupabase = isSupabaseHost(host);
    const isPooler = host.includes("pooler.supabase.com");

    let mode: SupabaseConnectionMode;
    let projectId = "";
    let region: string | undefined;

    if (isPooler) {
      // Supabase Cloud Pooler format
      mode = port === 5433 ? "pooler-transaction" : "pooler-session";

      // Extract project from username (postgres.PROJECT)
      const userParts = username.split(".");
      projectId = userParts.slice(1).join(".");

      // Extract region from host (aws-0-REGION.pooler.supabase.com)
      const hostMatch = host.match(/aws-\d+-(\w+)\.pooler\.supabase\.com/);
      region = hostMatch?.[1];
    } else if (isSupabase) {
      // Supabase Cloud Direct format
      mode = "direct";

      // Extract project from host (db.PROJECT.supabase.co)
      const hostMatch = host.match(/db\.(.+)\.supabase\.co/);
      projectId = hostMatch?.[1] || "";
    } else {
      // Local/custom PostgreSQL (e.g., Docker Supabase, localhost)
      // Detect mode from port
      mode = detectModeFromPort(port);
    }

    // For local connections, SSL is typically not required
    const useSsl = isSupabase;

    return {
      host,
      port,
      database,
      username: username.split(".")[0], // Extract base username for pooler format
      password,
      mode,
      projectId,
      region,
      useSsl,
    };
  } catch {
    return null;
  }
}

/**
 * Check if a hostname is a Supabase host.
 *
 * @param host - The hostname to check
 * @returns True if the host is a Supabase host
 */
export function isSupabaseHost(host: string): boolean {
  if (!host) return false;
  return host.includes(".supabase.co") || host.includes(".supabase.com");
}

/**
 * Get the recommended port for a Supabase connection mode.
 *
 * @param mode - The connection mode
 * @returns The recommended port number
 */
export function getSupabasePort(mode: SupabaseConnectionMode): number {
  switch (mode) {
    case "direct":
      return 5432;
    case "pooler-session":
      return 6543;
    case "pooler-transaction":
      return 5433;
  }
}

/**
 * Detect the connection mode from a port number.
 *
 * @param port - The port number
 * @returns The detected connection mode
 */
export function detectModeFromPort(port: number): SupabaseConnectionMode {
  switch (port) {
    case 6543:
      return "pooler-session";
    case 5433:
      return "pooler-transaction";
    default:
      return "direct";
  }
}

/**
 * Format a connection mode for display.
 *
 * @param mode - The connection mode
 * @returns Human-readable mode description
 */
export function formatConnectionMode(mode: SupabaseConnectionMode): string {
  switch (mode) {
    case "direct":
      return "Direct Connection (IPv4)";
    case "pooler-session":
      return "Connection Pooler (Session Mode)";
    case "pooler-transaction":
      return "Connection Pooler (Transaction Mode)";
  }
}

/**
 * Get a description of when to use each connection mode.
 *
 * @param mode - The connection mode
 * @returns Description of the mode's use case
 */
export function getModeDescription(mode: SupabaseConnectionMode): string {
  switch (mode) {
    case "direct":
      return "Best for migrations, long-running queries, and schema changes";
    case "pooler-session":
      return "Recommended for most web applications with persistent connections";
    case "pooler-transaction":
      return "Best for serverless functions and edge computing";
  }
}
