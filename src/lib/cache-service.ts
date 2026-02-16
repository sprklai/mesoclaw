/**
 * Centralized cache service for offline-first data persistence.
 *
 * This service provides a database-agnostic caching layer that works with
 * SQLite, PostgreSQL, and MySQL. It uses tauri-plugin-store for disk
 * persistence and supports TTL-based invalidation, stale flags, and
 * workspace-scoped cache isolation.
 *
 * Features:
 * - Workspace-scoped cache keys (prefixed with workspaceId)
 * - TTL (Time To Live) based expiration
 * - Stale-while-revalidate (SWR) pattern support
 * - Offline detection and graceful degradation
 * - Cache metadata (database type, timestamps, fingerprints)
 */

import { Store } from "@tauri-apps/plugin-store";

/**
 * Default TTL values for different cache types (in milliseconds)
 */
export const CACHE_TTL = {
  /** Schema metadata - 24 hours */
  SCHEMA: 24 * 60 * 60 * 1000,
  /** Database overview - 1 hour */
  OVERVIEW: 60 * 60 * 1000,
  /** ERD relationship data - 6 hours */
  ERD: 6 * 60 * 60 * 1000,
  /** Chat sessions - 7 days */
  CHAT: 7 * 24 * 60 * 60 * 1000,
  /** AI explanations - 7 days */
  EXPLANATION: 7 * 24 * 60 * 60 * 1000,
  /** Insights data - 1 hour */
  INSIGHTS: 60 * 60 * 1000,
  /** Table/column metadata - 6 hours */
  METADATA: 6 * 60 * 60 * 1000,
} as const;

/**
 * Database type enumeration for cache metadata
 */
export type DatabaseType = "sqlite" | "postgresql" | "mysql" | "mongodb";

/**
 * Cache entry metadata
 */
export interface CacheMetadata {
  /** Workspace ID for this cache entry */
  workspaceId: string;
  /** Database type (sqlite, postgresql, mysql) */
  databaseType: DatabaseType;
  /** Unix timestamp when this cache was created */
  createdAt: number;
  /** Unix timestamp when this cache expires */
  expiresAt: number;
  /** Fingerprint hash for change detection */
  fingerprint?: string;
  /** Whether this cache is marked as stale */
  isStale: boolean;
  /** Cache type/category */
  cacheType: keyof typeof CACHE_TTL;
}

/**
 * Complete cache entry with data and metadata
 */
export interface CacheEntry<T> {
  /** The cached data */
  data: T;
  /** Cache metadata */
  metadata: CacheMetadata;
}

/**
 * Cache key parts for type-safe key generation
 */
export interface CacheKeyParts {
  workspaceId: string;
  cacheType: string;
  entityType?: string;
  entityId?: string;
}

/**
 * Cache service class for managing cached data
 */
class CacheService {
  private store: Store | null = null;
  private storePath = "cache_db.json";

  /**
   * Get or create the singleton store instance
   */
  private async getStore(): Promise<Store> {
    if (this.store === null) {
      this.store = await Store.load(this.storePath);
    }
    return this.store;
  }

  /**
   * Generate a cache key from parts
   *
   * Format: {workspaceId}:{cacheType}[:{entityType}][:{entityId}]
   *
   * Examples:
   * - "abc123:schema"
   * - "abc123:overview"
   * - "abc123:erd:relationships"
   * - "abc123:chat:sessions"
   * - "abc123:explanation:table:users"
   */
  generateKey(parts: CacheKeyParts): string {
    const { workspaceId, cacheType, entityType, entityId } = parts;
    const keyParts = [workspaceId, cacheType];

    if (entityType) {
      keyParts.push(entityType);
    }

    if (entityId) {
      keyParts.push(entityId);
    }

    return keyParts.join(":");
  }

  /**
   * Parse a cache key back into parts
   */
  parseKey(key: string): CacheKeyParts | null {
    const parts = key.split(":");

    if (parts.length < 2) {
      return null;
    }

    return {
      workspaceId: parts[0],
      cacheType: parts[1],
      entityType: parts[2],
      entityId: parts[3],
    };
  }

  /**
   * Get cached data for a key
   *
   * @param key - Cache key
   * @returns Cached entry or null if not found/expired
   */
  async get<T>(key: string): Promise<CacheEntry<T> | null> {
    try {
      const store = await this.getStore();
      const entry = await store.get<CacheEntry<T>>(key);

      if (!entry) {
        return null;
      }

      // Check if cache has expired
      if (Date.now() > entry.metadata.expiresAt) {
        // Delete expired cache
        await store.delete(key);
        return null;
      }

      return entry;
    } catch (error) {
      console.error(`Failed to get cache for key ${key}:`, error);
      return null;
    }
  }

  /**
   * Set cached data for a key
   *
   * @param key - Cache key
   * @param data - Data to cache
   * @param metadata - Cache metadata (partial, will be merged with defaults)
   */
  async set<T>(
    key: string,
    data: T,
    metadata: Partial<CacheMetadata> & {
      workspaceId: string;
      cacheType: keyof typeof CACHE_TTL;
    }
  ): Promise<void> {
    try {
      const store = await this.getStore();
      const now = Date.now();
      const ttl = CACHE_TTL[metadata.cacheType];

      const fullMetadata: CacheMetadata = {
        workspaceId: metadata.workspaceId,
        databaseType: metadata.databaseType ?? "sqlite",
        createdAt: now,
        expiresAt: now + ttl,
        fingerprint: metadata.fingerprint,
        isStale: false,
        cacheType: metadata.cacheType,
      };

      const entry: CacheEntry<T> = {
        data,
        metadata: fullMetadata,
      };

      await store.set(key, entry);
      await store.save();
    } catch (error) {
      console.error(`Failed to set cache for key ${key}:`, error);
      throw error;
    }
  }

  /**
   * Invalidate (delete) a cache entry
   *
   * @param key - Cache key to invalidate
   */
  async invalidate(key: string): Promise<void> {
    try {
      const store = await this.getStore();
      await store.delete(key);
      await store.save();
    } catch (error) {
      console.error(`Failed to invalidate cache for key ${key}:`, error);
      throw error;
    }
  }

  /**
   * Mark a cache entry as stale (without deleting)
   *
   * @param key - Cache key to mark as stale
   */
  async markStale(key: string): Promise<void> {
    try {
      const store = await this.getStore();
      const entry = await store.get<CacheEntry<unknown>>(key);

      if (entry) {
        entry.metadata.isStale = true;
        await store.set(key, entry);
        await store.save();
      }
    } catch (error) {
      console.error(`Failed to mark cache as stale for key ${key}:`, error);
    }
  }

  /**
   * Check if a cache entry is stale
   *
   * @param key - Cache key to check
   * @returns True if stale or expired, false otherwise
   */
  async isStale(key: string): Promise<boolean> {
    try {
      const entry = await this.get<unknown>(key);

      if (!entry) {
        return true; // No cache = stale
      }

      return entry.metadata.isStale;
    } catch (error) {
      console.error(`Failed to check staleness for key ${key}:`, error);
      return true;
    }
  }

  /**
   * Clear all cache entries for a workspace
   *
   * @param workspaceId - Workspace ID to clear cache for
   */
  async clearWorkspace(workspaceId: string): Promise<void> {
    try {
      const store = await this.getStore();
      const keys = await store.keys();

      // Find all keys that start with workspaceId
      const keysToDelete = keys.filter((key) =>
        key.startsWith(`${workspaceId}:`)
      );

      for (const key of keysToDelete) {
        await store.delete(key);
      }

      await store.save();
    } catch (error) {
      console.error(
        `Failed to clear cache for workspace ${workspaceId}:`,
        error
      );
      throw error;
    }
  }

  /**
   * Clear all cache entries (all workspaces)
   */
  async clearAll(): Promise<void> {
    try {
      const store = await this.getStore();
      await store.clear();
      await store.save();
    } catch (error) {
      console.error("Failed to clear all cache:", error);
      throw error;
    }
  }

  /**
   * Get all cache keys for a workspace
   *
   * @param workspaceId - Workspace ID
   * @returns Array of cache keys
   */
  async getWorkspaceKeys(workspaceId: string): Promise<string[]> {
    try {
      const store = await this.getStore();
      const keys = await store.keys();

      return keys.filter((key) => key.startsWith(`${workspaceId}:`));
    } catch (error) {
      console.error(
        `Failed to get cache keys for workspace ${workspaceId}:`,
        error
      );
      return [];
    }
  }

  /**
   * Get cache size statistics for a workspace
   *
   * @param workspaceId - Workspace ID
   * @returns Object with cache statistics
   */
  async getCacheStats(workspaceId: string): Promise<{
    keyCount: number;
    totalSize: number;
    entriesByType: Record<string, number>;
  }> {
    try {
      const keys = await this.getWorkspaceKeys(workspaceId);
      const store = await this.getStore();
      const entriesByType: Record<string, number> = {};
      let totalSize = 0;

      for (const key of keys) {
        const entry = await store.get<CacheEntry<unknown>>(key);
        if (entry) {
          const cacheType = entry.metadata.cacheType;
          entriesByType[cacheType] = (entriesByType[cacheType] || 0) + 1;
          totalSize += JSON.stringify(entry).length;
        }
      }

      return {
        keyCount: keys.length,
        totalSize,
        entriesByType,
      };
    } catch (error) {
      console.error(
        `Failed to get cache stats for workspace ${workspaceId}:`,
        error
      );
      return {
        keyCount: 0,
        totalSize: 0,
        entriesByType: {},
      };
    }
  }

  /**
   * Invalidate cache entries by type for a workspace
   *
   * @param workspaceId - Workspace ID
   * @param cacheType - Cache type to invalidate
   */
  async invalidateByType(
    workspaceId: string,
    cacheType: keyof typeof CACHE_TTL
  ): Promise<void> {
    try {
      const keys = await this.getWorkspaceKeys(workspaceId);
      const store = await this.getStore();

      // Find keys that match the cache type
      const keysToDelete = keys.filter((key) => {
        const parsed = this.parseKey(key);
        return parsed?.cacheType === cacheType;
      });

      for (const key of keysToDelete) {
        await store.delete(key);
      }

      await store.save();
    } catch (error) {
      console.error(
        `Failed to invalidate cache type ${cacheType} for workspace ${workspaceId}:`,
        error
      );
      throw error;
    }
  }

  /**
   * Update cache fingerprint for change detection
   *
   * @param key - Cache key
   * @param fingerprint - New fingerprint hash
   */
  async updateFingerprint(key: string, fingerprint: string): Promise<void> {
    try {
      const store = await this.getStore();
      const entry = await store.get<CacheEntry<unknown>>(key);

      if (entry) {
        entry.metadata.fingerprint = fingerprint;
        await store.set(key, entry);
        await store.save();
      }
    } catch (error) {
      console.error(
        `Failed to update fingerprint for cache key ${key}:`,
        error
      );
    }
  }

  /**
   * Check if fingerprint has changed (schema has changed)
   *
   * @param key - Cache key
   * @param newFingerprint - New fingerprint to compare
   * @returns True if fingerprint changed, false otherwise
   */
  async hasFingerprintChanged(
    key: string,
    newFingerprint: string
  ): Promise<boolean> {
    try {
      const entry = await this.get<unknown>(key);

      if (!entry) {
        return true; // No cache = changed
      }

      return entry.metadata.fingerprint !== newFingerprint;
    } catch (error) {
      console.error(
        `Failed to check fingerprint change for key ${key}:`,
        error
      );
      return true;
    }
  }
}

/**
 * Singleton instance of the cache service
 */
export const cacheService = new CacheService();
