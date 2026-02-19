/**
 * LLM provider configuration store.
 *
 * Provider listing uses the gateway REST API (GET /api/v1/providers) when the
 * daemon is reachable, falling back to Tauri IPC otherwise.
 *
 * Operations that MUST remain on Tauri IPC (OS-level, no gateway equivalent):
 *   - API key read/write/delete (keychain commands: keychain_get/set/delete)
 *   - configure_llm_provider_command (writes to app SQLite DB via Diesel)
 *   - add/delete model and provider commands (writes to app SQLite DB)
 *   - loadProvidersAndModels (gateway only returns basic provider info, not models)
 */
import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

import { extractErrorMessage } from "@/lib/error-utils";
import { getGatewayClient } from "@/lib/gateway-client";
import { KeychainStorage } from "@/lib/keychain-storage";
import {
  DEFAULT_MODEL,
  DEFAULT_PROVIDER,
  type GlobalDefaultModel,
  type InitialModelSpec,
  type ProviderWithKeyStatus,
  type ProviderWithModels,
  type TestResult,
} from "@/lib/models";

/**
 * LLM provider configuration stored in the frontend.
 * API keys are stored in the OS keychain, cached in memory for all providers.
 */
export interface LLMProviderConfig {
  providerId: string;
  modelId: string;
  // apiKey is NOT stored here - it's fetched from keychain per provider
}

/**
 * API key cache - stores API keys in memory for all providers
 * This prevents re-fetching from keychain on every render
 */
interface ApiKeyCache {
  [providerId: string]: string | null;
}

function createConfig(providerId: string, modelId: string): LLMProviderConfig {
  return {
    providerId,
    modelId,
  };
}

interface LLMStore {
  // Current provider + model configuration
  config: LLMProviderConfig | null;

  // List of providers with their models (loaded from backend)
  providersWithModels: ProviderWithModels[];

  // List of providers with API key status (for settings UI)
  providersWithKeyStatus: ProviderWithKeyStatus[];

  // API key cache - stores keys in memory for all providers
  apiKeyCache: ApiKeyCache;

  // Loading states
  isLoading: boolean;
  isLoadingProviders: boolean;

  // Initialize the store (load config from backend)
  initialize: () => Promise<void>;

  // Load providers and models from backend
  loadProvidersAndModels: () => Promise<void>;

  // Load providers with key status (for settings UI)
  loadProvidersWithKeyStatus: () => Promise<void>;

  // Get a specific provider by ID
  getProviderById: (providerId: string) => Promise<ProviderWithModels | null>;

  // Test provider connection
  testProviderConnection: (
    providerId: string,
    apiKey: string
  ) => Promise<TestResult>;

  // Load API key for a specific provider (cached in memory)
  loadApiKeyForProvider: (providerId: string) => Promise<void>;

  // Load all API keys for all providers (called on mount)
  loadAllApiKeys: () => Promise<void>;

  // Save API key to keychain for a specific provider
  saveApiKeyForProvider: (providerId: string, apiKey: string) => Promise<void>;

  // Get API key for a specific provider (from cache)
  getApiKey: (providerId: string) => Promise<string>;

  // Delete API key for a provider
  deleteApiKeyForProvider: (providerId: string) => Promise<void>;

  // Check if provider has API key
  hasApiKey: (providerId: string) => Promise<boolean>;

  // Add a custom model
  addCustomModel: (
    providerId: string,
    modelId: string,
    displayName: string
  ) => Promise<void>;

  // Delete a custom model
  deleteModel: (modelId: string) => Promise<void>;

  // Seed standard AI models to database
  seedStandardModels: () => Promise<number>;

  // Reset and re-seed all standard models (clears old models first)
  resetAndSeedModels: () => Promise<number>;

  // Save provider + model configuration to backend
  saveProviderConfig: (providerId: string, modelId: string) => Promise<void>;

  // Discover Ollama models
  discoverOllamaModels: () => Promise<number>;

  // Add a new custom provider
  addCustomProvider: (
    id: string,
    name: string,
    baseUrl: string,
    requiresApiKey: boolean
  ) => Promise<void>;

  // Update provider details
  updateProvider: (providerId: string, baseUrl: string) => Promise<void>;

  // Add a user-defined provider with initial models
  addUserProvider: (
    id: string,
    name: string,
    baseUrl: string,
    requiresApiKey: boolean,
    initialModels: InitialModelSpec[],
    apiKey?: string
  ) => Promise<void>;

  // Delete a user-defined provider
  deleteUserProvider: (providerId: string) => Promise<void>;

  // Get the global default model configuration
  getGlobalDefaultModel: () => Promise<GlobalDefaultModel | null>;

  // Set the global default model configuration
  setGlobalDefaultModel: (providerId: string, modelId: string) => Promise<void>;

  // Get user-defined providers (computed)
  getUserDefinedProviders: () => ProviderWithModels[];
}

export const useLLMStore = create<LLMStore>((set, get) => ({
  config: null,
  providersWithModels: [],
  providersWithKeyStatus: [],
  apiKeyCache: {},
  isLoading: true,
  isLoadingProviders: false,

  initialize: async () => {
    try {
      const configResponse = await invoke<{
        providerId: string;
        modelId: string;
      }>("get_llm_provider_config_command");

      const newConfig = createConfig(
        configResponse.providerId,
        configResponse.modelId
      );
      set({ config: newConfig, isLoading: false });
    } catch (error) {
      console.warn(
        "[LLMStore] Failed to load config from backend, using defaults:",
        error
      );
      const newConfig = createConfig(DEFAULT_PROVIDER, DEFAULT_MODEL);
      set({ config: newConfig, isLoading: false });
    }
  },

  loadProvidersAndModels: async () => {
    set({ isLoadingProviders: true });

    try {
      const providers = await invoke<ProviderWithModels[]>(
        "list_ai_providers_command"
      );
      set({ providersWithModels: providers, isLoadingProviders: false });
    } catch (error) {
      console.error("[LLMStore] Failed to load providers and models:", error);
      set({ isLoadingProviders: false });
    }
  },

  loadProvidersWithKeyStatus: async () => {
    // Try gateway first for basic provider listing, fall back to IPC.
    const client = getGatewayClient();
    if (client) {
      try {
        const res = await client.listProviders();
        // Gateway returns basic info; enrich with hasApiKey from keychain + empty models.
        const providers: ProviderWithKeyStatus[] = await Promise.all(
          res.providers.map(async (p) => {
            let hasApiKey = false;
            if (p.requiresApiKey) {
              try {
                await KeychainStorage.getApiKey(p.id);
                hasApiKey = true;
              } catch {
                // Key not set
              }
            }
            return {
              id: p.id,
              name: p.name,
              baseUrl: "",
              requiresApiKey: p.requiresApiKey,
              isActive: p.isActive,
              isUserDefined: false,
              hasApiKey,
              models: [],
            };
          }),
        );
        set({ providersWithKeyStatus: providers });
        return;
      } catch {
        // Gateway unavailable, fall through to IPC.
      }
    }

    try {
      const providers = await invoke<ProviderWithKeyStatus[]>(
        "list_providers_with_key_status_command"
      );
      set({ providersWithKeyStatus: providers });
    } catch (error) {
      console.error(
        "[LLMStore] Failed to load providers with key status:",
        error
      );
    }
  },

  getProviderById: async (providerId: string) => {
    try {
      const provider = await invoke<ProviderWithModels>(
        "get_provider_by_id_command",
        { providerId }
      );
      return provider;
    } catch (error) {
      console.error(`[LLMStore] Failed to get provider ${providerId}:`, error);
      return null;
    }
  },

  testProviderConnection: async (providerId: string, apiKey: string) => {
    try {
      const result = await invoke<TestResult>(
        "test_provider_connection_command",
        { providerId, apiKey }
      );
      return result;
    } catch (error) {
      console.error(`[LLMStore] Failed to test provider ${providerId}:`, error);
      return {
        success: false,
        message: extractErrorMessage(error),
      };
    }
  },

  loadApiKeyForProvider: async (providerId: string) => {
    // Check if already in cache
    const cached = get().apiKeyCache[providerId];
    if (cached !== undefined) {
      return; // Already loaded or checked
    }

    try {
      const apiKey = await KeychainStorage.getApiKey(providerId);
      set((state) => ({
        apiKeyCache: {
          ...state.apiKeyCache,
          [providerId]: apiKey,
        },
      }));
    } catch {
      // Key doesn't exist - store null to mark as checked
      set((state) => ({
        apiKeyCache: {
          ...state.apiKeyCache,
          [providerId]: null,
        },
      }));
    }
  },

  loadAllApiKeys: async () => {
    const { providersWithKeyStatus } = get();

    // Load all API keys in parallel
    const keyPromises = providersWithKeyStatus.map(async (provider) => {
      if (!provider.requiresApiKey) {
        return { providerId: provider.id, apiKey: null };
      }
      try {
        const apiKey = await KeychainStorage.getApiKey(provider.id);
        return { providerId: provider.id, apiKey };
      } catch {
        return { providerId: provider.id, apiKey: null };
      }
    });

    const results = await Promise.all(keyPromises);

    // Update cache
    const cache: ApiKeyCache = {};
    for (const result of results) {
      cache[result.providerId] = result.apiKey;
    }

    set({ apiKeyCache: cache });
  },

  saveApiKeyForProvider: async (providerId: string, apiKey: string) => {
    await KeychainStorage.setApiKey(providerId, apiKey);
    // Update cache
    set((state) => ({
      apiKeyCache: {
        ...state.apiKeyCache,
        [providerId]: apiKey,
      },
    }));
  },

  getApiKey: async (providerId: string) => {
    // First check cache
    const cached = get().apiKeyCache[providerId];
    if (cached !== undefined) {
      if (cached === null) {
        throw new Error("API key not found");
      }
      return cached;
    }

    // If not in cache, try to load it
    try {
      const apiKey = await KeychainStorage.getApiKey(providerId);
      // Update cache
      set((state) => ({
        apiKeyCache: {
          ...state.apiKeyCache,
          [providerId]: apiKey,
        },
      }));
      return apiKey;
    } catch {
      throw new Error("API key not found");
    }
  },

  deleteApiKeyForProvider: async (providerId: string) => {
    await KeychainStorage.deleteApiKey(providerId);
    // Update cache
    set((state) => ({
      apiKeyCache: {
        ...state.apiKeyCache,
        [providerId]: null,
      },
    }));
  },

  hasApiKey: async (providerId: string) => {
    // Check cache first
    const cached = get().apiKeyCache[providerId];
    if (cached !== undefined) {
      return cached !== null;
    }

    // If not in cache, check keychain
    try {
      await KeychainStorage.getApiKey(providerId);
      return true;
    } catch {
      return false;
    }
  },

  addCustomModel: async (
    providerId: string,
    modelId: string,
    displayName: string
  ) => {
    await invoke("add_custom_model_command", {
      providerId,
      modelId,
      displayName,
    });

    // Reload providers and models to get the updated list
    await get().loadProvidersAndModels();
  },

  deleteModel: async (modelId: string) => {
    await invoke("delete_model_command", { modelId });

    // Reload providers and models to get the updated list
    await get().loadProvidersAndModels();
  },

  saveProviderConfig: async (providerId: string, modelId: string) => {
    await invoke("configure_llm_provider_command", {
      providerId,
      modelId,
    });
    set({ config: createConfig(providerId, modelId) });
  },

  discoverOllamaModels: async () => {
    const addedCount = await invoke<number>("discover_ollama_models_command");

    // Reload providers to get new models
    await get().loadProvidersAndModels();

    return addedCount;
  },

  addCustomProvider: async (
    id: string,
    name: string,
    baseUrl: string,
    requiresApiKey: boolean
  ) => {
    // This would need a backend command to add a provider
    console.log("Adding custom provider:", {
      id,
      name,
      baseUrl,
      requiresApiKey,
    });
    // For now, just reload providers
    await get().loadProvidersAndModels();
  },

  updateProvider: async (providerId: string, baseUrl: string) => {
    await invoke("update_provider_command", {
      providerId,
      baseUrl,
    });
    // Reload providers to get the updated provider
    await get().loadProvidersAndModels();
    await get().loadProvidersWithKeyStatus();
  },

  seedStandardModels: async () => {
    const insertedCount = await invoke<number>("seed_ai_models_command");
    // Reload providers to get the newly seeded models
    await get().loadProvidersAndModels();
    return insertedCount;
  },

  resetAndSeedModels: async () => {
    const seededCount = await invoke<number>("reset_and_seed_models_command");
    // Reload providers to get the newly seeded models
    await get().loadProvidersAndModels();
    return seededCount;
  },

  addUserProvider: async (
    id: string,
    name: string,
    baseUrl: string,
    requiresApiKey: boolean,
    initialModels: InitialModelSpec[],
    apiKey?: string
  ) => {
    // Create the provider with initial models
    await invoke("add_user_provider_command", {
      id,
      name,
      baseUrl,
      requiresApiKey,
      initialModels,
    });

    // If API key provided, save it to keychain
    if (apiKey && requiresApiKey) {
      await KeychainStorage.setApiKey(id, apiKey);
      // Update cache
      set((state) => ({
        apiKeyCache: {
          ...state.apiKeyCache,
          [id]: apiKey,
        },
      }));
    }

    // Reload providers to get the new provider
    await get().loadProvidersAndModels();
    await get().loadProvidersWithKeyStatus();
  },

  deleteUserProvider: async (providerId: string) => {
    await invoke("delete_user_provider_command", { providerId });

    // Remove from API key cache
    set((state) => {
      const newCache = { ...state.apiKeyCache };
      delete newCache[providerId];
      return { apiKeyCache: newCache };
    });

    // Reload providers
    await get().loadProvidersAndModels();
    await get().loadProvidersWithKeyStatus();
  },

  getGlobalDefaultModel: async () => {
    try {
      const result = await invoke<GlobalDefaultModel | null>(
        "get_global_default_model_command"
      );
      return result;
    } catch (error) {
      console.error("[LLMStore] Failed to get global default model:", error);
      return null;
    }
  },

  setGlobalDefaultModel: async (providerId: string, modelId: string) => {
    await invoke("set_global_default_model_command", {
      providerId,
      modelId,
    });
  },

  getUserDefinedProviders: () => {
    return get().providersWithModels.filter((p) => p.isUserDefined);
  },
}));

// Helper to get the default provider ID
export function getDefaultProvider(): string {
  return DEFAULT_PROVIDER;
}

// Helper to get the default model ID
export function getDefaultModel(): string {
  return DEFAULT_MODEL;
}

// Backward compatibility alias
export function getVercelProvider(): string {
  return DEFAULT_PROVIDER;
}
