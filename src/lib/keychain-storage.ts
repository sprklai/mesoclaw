import { invoke } from "@tauri-apps/api/core";
import { APP_IDENTITY } from "@/config/app-identity";

/**
 * Fast, OS-native secure storage using system keychain
 * - macOS: Keychain
 * - Linux: Secret Service
 * - Windows: Credential Manager
 *
 * Access time: <10ms (vs 40 seconds with Stronghold)
 */

const SERVICE_NAME = APP_IDENTITY.keychainService;

export class KeychainStorage {
  /**
   * Set a secret in the OS keychain
   * @param key - Unique identifier for the secret
   * @param value - The secret value to store
   */
  static async set(key: string, value: string): Promise<void> {
    try {
      await invoke("keychain_set", {
        service: SERVICE_NAME,
        key,
        value,
      });
    } catch (error) {
      console.error(`[KeychainStorage] Failed to set key ${key}:`, error);
      throw new Error(
        `Failed to save to keychain: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  /**
   * Get a secret from the OS keychain
   * @param key - Unique identifier for the secret
   * @returns The secret value
   * @throws Error if key not found
   */
  static async get(key: string): Promise<string> {
    try {
      const value = await invoke<string>("keychain_get", {
        service: SERVICE_NAME,
        key,
      });
      return value;
    } catch (error) {
      // Extract error message from various error formats
      let errorMessage = "";

      if (error instanceof Error) {
        errorMessage = error.message;
      } else if (typeof error === "object" && error !== null) {
        // Tauri errors are objects with a message property
        const errObj = error as Record<string, unknown>;
        errorMessage = (errObj.message as string) || JSON.stringify(error);
      } else if (typeof error === "string") {
        errorMessage = error;
      } else {
        errorMessage = String(error);
      }

      // Check if this is an expected "no entry" error (happens on first run)
      const isNoEntryError =
        errorMessage.includes("No entry") ||
        errorMessage.includes("No matching entry") ||
        errorMessage.includes("No error");

      if (isNoEntryError) {
        console.warn(
          `[KeychainStorage] Key not found (expected on first run): ${key}`
        );
      } else {
        console.error(`[KeychainStorage] Failed to get key ${key}:`, error);
      }
      throw error;
    }
  }

  /**
   * Delete a secret from the OS keychain
   * @param key - Unique identifier for the secret
   */
  static async delete(key: string): Promise<void> {
    try {
      await invoke("keychain_delete", {
        service: SERVICE_NAME,
        key,
      });
    } catch (error) {
      console.error(`[KeychainStorage] Failed to delete key ${key}:`, error);
      throw new Error(
        `Failed to delete from keychain: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  /**
   * Check if a secret exists in the OS keychain
   * @param key - Unique identifier for the secret
   * @returns true if key exists, false otherwise
   */
  static async exists(key: string): Promise<boolean> {
    try {
      const exists = await invoke<boolean>("keychain_exists", {
        service: SERVICE_NAME,
        key,
      });
      return exists;
    } catch {
      // exists() failing is not a critical error - just return false
      return false;
    }
  }

  // Convenience methods for API keys
  static async setApiKey(provider: string, apiKey: string): Promise<void> {
    await this.set(`api_key:${provider}`, apiKey);
  }

  static async getApiKey(provider: string): Promise<string> {
    return await this.get(`api_key:${provider}`);
  }

  static async deleteApiKey(provider: string): Promise<void> {
    await this.delete(`api_key:${provider}`);
  }

  static async hasApiKey(provider: string): Promise<boolean> {
    return await this.exists(`api_key:${provider}`);
  }
}
