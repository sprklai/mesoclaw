import { appDataDir } from "@tauri-apps/api/path";
import { Client, Stronghold } from "@tauri-apps/plugin-stronghold";

import type {
  SecretResponse,
  SecretCategoryType,
} from "../types/secure-storage";

let strongholdInstance: Stronghold | null = null;
let clientInstance: Client | null = null;
let initPromise: Promise<{
  stronghold: Stronghold;
  client: Client;
}> | null = null;

async function initStronghold(): Promise<{
  stronghold: Stronghold;
  client: Client;
}> {
  // Return existing promise if initialization is in progress
  if (initPromise) {
    return initPromise;
  }

  // Return cached instances if already initialized
  if (strongholdInstance && clientInstance) {
    return { stronghold: strongholdInstance, client: clientInstance };
  }

  // Create initialization promise
  initPromise = (async () => {
    const totalStart = performance.now();
    try {
      const appDataStart = performance.now();
      const appData = await appDataDir();
      console.log(
        `[SecureStorage] appDataDir() took ${(performance.now() - appDataStart).toFixed(2)}ms`
      );

      const vaultPath = `${appData}/secrets.hold`;
      const vaultPassword = "aiboilerplate-secure-vault";

      console.log("[SecureStorage] Initializing Stronghold at:", vaultPath);

      const loadStart = performance.now();
      strongholdInstance = await Stronghold.load(vaultPath, vaultPassword);
      console.log(
        `[SecureStorage] Stronghold.load() took ${(performance.now() - loadStart).toFixed(2)}ms`
      );

      const clientName = "aiboilerplate-secrets";
      try {
        const loadClientStart = performance.now();
        clientInstance = await strongholdInstance.loadClient(clientName);
        console.log(
          `[SecureStorage] loadClient() took ${(performance.now() - loadClientStart).toFixed(2)}ms`
        );
      } catch {
        const createClientStart = performance.now();
        clientInstance = await strongholdInstance.createClient(clientName);
        console.log(
          `[SecureStorage] createClient() took ${(performance.now() - createClientStart).toFixed(2)}ms`
        );

        const saveStart = performance.now();
        await strongholdInstance.save();
        console.log(
          `[SecureStorage] save() took ${(performance.now() - saveStart).toFixed(2)}ms`
        );
      }

      console.log(
        `[SecureStorage] Total initialization took ${(performance.now() - totalStart).toFixed(2)}ms`
      );
      return { stronghold: strongholdInstance, client: clientInstance };
    } catch (error) {
      console.error("[SecureStorage] Initialization failed:", error);
      strongholdInstance = null;
      clientInstance = null;
      initPromise = null;
      throw new Error(
        `Failed to initialize Stronghold: ${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      // Clear the promise once done (whether success or failure)
      initPromise = null;
    }
  })();

  return initPromise;
}

async function insertRecord(
  store: any,
  key: string,
  value: string
): Promise<void> {
  const data = Array.from(new TextEncoder().encode(value));
  await store.insert(key, data);
}

async function getRecord(store: any, key: string): Promise<string> {
  const data = await store.get(key);
  if (!data) {
    throw new Error(`Key not found: ${key}`);
  }
  return new TextDecoder().decode(new Uint8Array(data));
}

function buildKey(category: string, key: string): string {
  return `${category}:${key}`;
}

export class SecureStorage {
  static async setSecret(
    key: string,
    value: string,
    category: SecretCategoryType,
    metadata?: Record<string, string>
  ): Promise<void> {
    const fullKey = buildKey(category, key);
    console.log(`[SecureStorage] Setting secret: ${fullKey}`);

    try {
      const { stronghold, client } = await initStronghold();
      const store = client.getStore();

      const entry: SecretResponse = {
        key,
        value,
        category,
        metadata,
      };

      // Insert the record
      await insertRecord(store, fullKey, JSON.stringify(entry));
      console.log(`[SecureStorage] Record inserted: ${fullKey}`);

      // Update metadata
      await this.addKeyToMetadata(store, category, key);
      console.log(`[SecureStorage] Metadata updated: ${fullKey}`);

      // Immediately save to ensure persistence
      // Per Stronghold docs: save() must be called after making changes
      await stronghold.save();
      console.log(
        `[SecureStorage] Vault saved successfully after setting: ${fullKey}`
      );
    } catch (error) {
      console.error(`[SecureStorage] Failed to set secret ${fullKey}:`, error);
      throw new Error(
        `Failed to save secret: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  static async getSecret(
    key: string,
    category: SecretCategoryType
  ): Promise<SecretResponse> {
    const fullKey = buildKey(category, key);
    console.log(`[SecureStorage] Getting secret: ${fullKey}`);

    try {
      const { client } = await initStronghold();
      const store = client.getStore();

      const data = await getRecord(store, fullKey);
      console.log(`[SecureStorage] Secret retrieved successfully: ${fullKey}`);
      return JSON.parse(data) as SecretResponse;
    } catch (error) {
      console.error(`[SecureStorage] Failed to get secret ${fullKey}:`, error);
      throw error;
    }
  }

  static async updateSecret(
    key: string,
    value: string,
    category: SecretCategoryType,
    metadata?: Record<string, string>
  ): Promise<void> {
    await this.setSecret(key, value, category, metadata);
  }

  static async deleteSecret(
    key: string,
    category: SecretCategoryType
  ): Promise<void> {
    const fullKey = buildKey(category, key);
    console.log(`[SecureStorage] Deleting secret: ${fullKey}`);

    try {
      const { stronghold, client } = await initStronghold();
      const store = client.getStore();

      await store.remove(fullKey);
      await this.removeKeyFromMetadata(store, category, key);
      await stronghold.save();
      console.log(`[SecureStorage] Secret deleted successfully: ${fullKey}`);
    } catch (error) {
      console.error(
        `[SecureStorage] Failed to delete secret ${fullKey}:`,
        error
      );
      throw new Error(
        `Failed to delete secret: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  static async listKeys(category: SecretCategoryType): Promise<string[]> {
    const { client } = await initStronghold();
    const store = client.getStore();

    const metadataKey = `__metadata__:${category}`;

    try {
      const data = await store.get(metadataKey);
      if (!data) {
        return [];
      }
      const metadataStr = new TextDecoder().decode(new Uint8Array(data));
      const metadata = JSON.parse(metadataStr);
      return metadata.keys || [];
    } catch {
      return [];
    }
  }

  private static async addKeyToMetadata(
    store: any,
    category: string,
    key: string
  ): Promise<void> {
    const metadataKey = `__metadata__:${category}`;
    let keys: string[] = [];

    try {
      const data = await store.get(metadataKey);
      if (data) {
        const metadataStr = new TextDecoder().decode(new Uint8Array(data));
        const metadata = JSON.parse(metadataStr);
        keys = metadata.keys || [];
      }
    } catch {
      keys = [];
    }

    if (!keys.includes(key)) {
      keys.push(key);
      const metadata = { keys };
      const metadataData = Array.from(
        new TextEncoder().encode(JSON.stringify(metadata))
      );
      try {
        await store.insert(metadataKey, metadataData);
        console.log(`[SecureStorage] Added key to metadata: ${metadataKey}`);
      } catch (error) {
        console.error(
          `[SecureStorage] Failed to add key to metadata ${metadataKey}:`,
          error
        );
        throw error;
      }
    }
  }

  private static async removeKeyFromMetadata(
    store: any,
    category: string,
    key: string
  ): Promise<void> {
    const metadataKey = `__metadata__:${category}`;

    try {
      const data = await store.get(metadataKey);
      if (data) {
        const metadataStr = new TextDecoder().decode(new Uint8Array(data));
        const metadata = JSON.parse(metadataStr);
        const keys = (metadata.keys || []).filter((k: string) => k !== key);
        const newMetadata = { keys };
        const metadataData = Array.from(
          new TextEncoder().encode(JSON.stringify(newMetadata))
        );
        await store.insert(metadataKey, metadataData);
      }
    } catch {
      // Ignore errors
    }
  }

  static async hasSecret(
    key: string,
    category: SecretCategoryType
  ): Promise<boolean> {
    try {
      await this.getSecret(key, category);
      return true;
    } catch {
      return false;
    }
  }

  static async setApiKey(provider: string, apiKey: string): Promise<void> {
    await this.setSecret(provider, apiKey, "api_key", { provider });
  }

  static async getApiKey(provider: string): Promise<string> {
    const secret = await this.getSecret(provider, "api_key");
    return secret.value;
  }

  static async deleteApiKey(provider: string): Promise<void> {
    await this.deleteSecret(provider, "api_key");
  }

  static async listApiKeyProviders(): Promise<string[]> {
    return await this.listKeys("api_key");
  }
}

export const apiKeyHelpers = {
  async setOpenAIKey(apiKey: string): Promise<void> {
    await SecureStorage.setApiKey("openai", apiKey);
  },

  async getOpenAIKey(): Promise<string | null> {
    try {
      return await SecureStorage.getApiKey("openai");
    } catch {
      return null;
    }
  },

  async setAnthropicKey(apiKey: string): Promise<void> {
    await SecureStorage.setApiKey("anthropic", apiKey);
  },

  async getAnthropicKey(): Promise<string | null> {
    try {
      return await SecureStorage.getApiKey("anthropic");
    } catch {
      return null;
    }
  },

  async setGeminiKey(apiKey: string): Promise<void> {
    await SecureStorage.setApiKey("gemini", apiKey);
  },

  async getGeminiKey(): Promise<string | null> {
    try {
      return await SecureStorage.getApiKey("gemini");
    } catch {
      return null;
    }
  },

  async setGroqKey(apiKey: string): Promise<void> {
    await SecureStorage.setApiKey("groq", apiKey);
  },

  async getGroqKey(): Promise<string | null> {
    try {
      return await SecureStorage.getApiKey("groq");
    } catch {
      return null;
    }
  },

  async setOllamaKey(apiKey: string): Promise<void> {
    await SecureStorage.setApiKey("ollama", apiKey);
  },

  async getOllamaKey(): Promise<string | null> {
    try {
      return await SecureStorage.getApiKey("ollama");
    } catch {
      return null;
    }
  },

  async deleteProvider(provider: string): Promise<void> {
    await SecureStorage.deleteApiKey(provider);
  },

  async hasProvider(provider: string): Promise<boolean> {
    try {
      await SecureStorage.getApiKey(provider);
      return true;
    } catch {
      return false;
    }
  },

  async getAllProviders(): Promise<string[]> {
    return await SecureStorage.listApiKeyProviders();
  },
};

// No eager initialization - let Stronghold initialize lazily when first needed
// This prevents blocking the splash screen for 38+ seconds on first run
