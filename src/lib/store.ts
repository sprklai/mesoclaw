/**
 * Frontend wrapper for tauri-plugin-store API.
 *
 * This module provides utilities for caching schema metadata using
 * the tauri-plugin-store plugin, enabling offline-first capabilities.
 *
 * Store file: schema_cache.json (managed by Tauri)
 */

import { invoke } from "@tauri-apps/api/core";
import { Store } from "@tauri-apps/plugin-store";

import type { SchemaSnapshot } from "@/types/schema";

/**
 * Schema cache entry containing the snapshot and metadata.
 */
export interface SchemaCacheEntry {
  /** Workspace ID for this cache entry */
  workspaceId: string;
  /** Fingerprint of the schema (hash of schema structure) */
  fingerprint: string;
  /** Unix timestamp when this cache was created */
  cachedAt: number;
  /** The actual schema snapshot data */
  schema: SchemaSnapshot;
  /** Metadata about this cache entry */
  metadata: {
    /** Number of tables in the schema */
    tableCount: number;
    /** Number of relationships in the schema */
    relationshipCount: number;
    /** Database type (SQLite, PostgreSQL, MySQL) */
    databaseType: "sqlite" | "postgresql" | "mysql";
    /** Unix timestamp of last connection (if available) */
    lastConnectedAt?: number;
  };
}

/**
 * Singleton store instance for schema cache.
 */
let schemaStore: Store | null = null;

/**
 * Get or create the singleton store instance.
 *
 * @returns Promise resolving to the Store instance
 * @throws Error if store cannot be loaded
 */
export async function getStore(): Promise<Store> {
  if (schemaStore === null) {
    schemaStore = await Store.load("schema_cache.json");
  }
  return schemaStore;
}

/**
 * Cache schema metadata for a workspace.
 *
 * This function stores the schema snapshot along with metadata
 * to enable offline access and fast loading.
 *
 * @param workspaceId - The workspace ID to cache schema for
 * @param schema - The schema snapshot to cache
 * @param fingerprint - Fingerprint of the schema (hash of schema structure)
 * @throws Error if caching fails
 */
export async function cacheSchema(
  workspaceId: string,
  schema: SchemaSnapshot,
  fingerprint: string
): Promise<void> {
  try {
    // Prepare simplified cache data for backend
    const cacheData = {
      tables: schema.tables.map((table) => ({
        name: table.name,
        schema: table.schema,
        tableType: table.table_type,
        columnCount: schema.columns.filter(
          (col) => col.table_name === table.name
        ).length,
      })),
      relationships: schema.relationships.map((rel) => ({
        fromTable: rel.from_table,
        fromColumn: rel.from_column,
        toTable: rel.to_table,
        toColumn: rel.to_column,
        isExplicit: rel.is_explicit,
      })),
      databaseType:
        schema.tables[0]?.schema === null
          ? "sqlite"
          : schema.tables[0]?.schema === "public"
            ? "postgresql"
            : "mysql",
    };

    // Invoke backend command to cache schema
    await invoke("cache_schema_command", {
      workspaceId,
      schema: cacheData,
      fingerprint,
    });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to cache schema: ${message}`);
  }
}

/**
 * Retrieve cached schema for a workspace.
 *
 * @param workspaceId - The workspace ID to get cached schema for
 * @returns Promise resolving to the cache entry, or null if not found
 * @throws Error if retrieval fails
 */
export async function getCachedSchema(
  workspaceId: string
): Promise<SchemaCacheEntry | null> {
  try {
    const entry = await invoke<SchemaCacheEntry | null>(
      "get_cached_schema_command",
      { workspaceId }
    );
    return entry;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to get cached schema: ${message}`);
  }
}

/**
 * Mark cached schema as stale (schema may have changed).
 *
 * This sets a flag indicating that the cached schema may be outdated
 * and should be refreshed on next access.
 *
 * @param workspaceId - The workspace ID to invalidate schema for
 * @throws Error if invalidation fails
 */
export async function invalidateSchema(workspaceId: string): Promise<void> {
  try {
    await invoke("invalidate_schema_command", { workspaceId });
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to invalidate schema: ${message}`);
  }
}

/**
 * Check if cached schema is marked as stale.
 *
 * @param workspaceId - The workspace ID to check staleness for
 * @returns Promise resolving to true if schema is stale, false otherwise
 * @throws Error if check fails
 */
export async function isSchemaStale(workspaceId: string): Promise<boolean> {
  try {
    const stale = await invoke<boolean>("is_schema_stale_command", {
      workspaceId,
    });
    return stale;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to check schema staleness: ${message}`);
  }
}

/**
 * Get store keys by pattern.
 *
 * This function is useful for bulk operations like clearing all
 * cached schemas or finding specific entries.
 *
 * @param pattern - The key pattern to match (e.g., "schema:*")
 * @returns Promise resolving to array of matching keys
 * @throws Error if key retrieval fails
 */
export async function keys(pattern: string): Promise<string[]> {
  try {
    const store = await getStore();
    const allKeys = await store.keys();
    // Filter keys by pattern
    const regex = new RegExp(
      "^" + pattern.replace(/\*/g, ".*").replace(/\?/g, ".") + "$"
    );
    return allKeys.filter((key) => regex.test(key));
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to get store keys: ${message}`);
  }
}

/**
 * Get the store file path for debugging purposes.
 *
 * @returns Promise resolving to the store file path
 * @throws Error if path retrieval fails
 */
export async function getStorePath(): Promise<string> {
  try {
    const path = await invoke<string>("get_store_command");
    return path;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`Failed to get store path: ${message}`);
  }
}
