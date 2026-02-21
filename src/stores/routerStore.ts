/**
 * Router configuration store.
 *
 * Manages the MesoClaw routing system for intelligent model selection.
 * Uses Tauri IPC to communicate with the backend router service.
 */
import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

import { extractErrorMessage } from "@/lib/error-utils";

/**
 * Routing profile types
 */
export type RoutingProfile = "eco" | "balanced" | "premium";

/**
 * Task types for routing
 */
export type TaskType =
  | "code"
  | "general"
  | "fast"
  | "creative"
  | "analysis"
  | "other";

/**
 * Model modality types
 */
export type ModelModality =
  | "text"
  | "image"
  | "image_generation"
  | "audio_transcription"
  | "audio_generation"
  | "video"
  | "embedding";

/**
 * Model capabilities
 */
export interface ModelCapabilities {
  toolCalling: boolean;
  streaming: boolean;
  structuredOutput: boolean;
  systemPrompt: boolean;
  maxOutputTokens: number | null;
}

/**
 * Discovered model from provider APIs
 */
export interface DiscoveredModel {
  id: string;
  displayName: string;
  providerId: string;
  modelId: string;
  costTier: "low" | "medium" | "high";
  contextLimit: number | null;
  modalities: ModelModality[];
  capabilities: ModelCapabilities | null;
  discoveredAt: string;
  isActive: boolean;
}

/**
 * Router configuration from backend
 */
export interface RouterConfig {
  activeProfile: RoutingProfile;
  customRoutes: Record<string, string> | null;
  taskOverrides: Record<string, string> | null;
  lastDiscovery: string | null;
}

/**
 * Router store state
 */
interface RouterStore {
  // Current router configuration
  config: RouterConfig | null;

  // Discovered models cache
  models: DiscoveredModel[];

  // Loading states
  isLoading: boolean;
  isLoadingModels: boolean;
  isDiscovering: boolean;

  // Error state
  error: string | null;

  // Initialize the store (load config and models)
  initialize: () => Promise<void>;

  // Load router configuration from backend
  loadConfig: () => Promise<void>;

  // Load discovered models
  loadModels: () => Promise<void>;

  // Set the active routing profile
  setProfile: (profile: RoutingProfile) => Promise<void>;

  // Set a task override
  setTaskOverride: (task: TaskType, modelId: string) => Promise<void>;

  // Clear a task override
  clearTaskOverride: (task: TaskType) => Promise<void>;

  // Discover models from a provider
  discoverModels: (
    providerId: string,
    baseUrl: string,
    apiKey?: string
  ) => Promise<number>;

  // Route a message to get the best model
  routeMessage: (message: string) => Promise<string | null>;

  // Route a message with modality requirements
  routeMessageWithModalities: (
    message: string,
    modalities: ModelModality[]
  ) => Promise<string | null>;

  // Get available models for current profile
  getAvailableModels: () => Promise<string[]>;

  // Check if a provider is available
  isProviderAvailable: (
    providerId: string,
    baseUrl: string,
    apiKey?: string
  ) => Promise<boolean>;

  // Reload models from database
  reloadModels: () => Promise<number>;

  // Get model count
  getModelCount: () => Promise<number>;

  // Get models by provider
  getModelsByProvider: (providerId: string) => DiscoveredModel[];

  // Get models by cost tier
  getModelsByCostTier: (tier: "low" | "medium" | "high") => DiscoveredModel[];

  // Get models by modality
  getModelsByModality: (modality: ModelModality) => DiscoveredModel[];

  // Clear error
  clearError: () => void;
}

/**
 * Default router configuration
 */
function createDefaultConfig(): RouterConfig {
  return {
    activeProfile: "balanced",
    customRoutes: null,
    taskOverrides: null,
    lastDiscovery: null,
  };
}

export const useRouterStore = create<RouterStore>((set, get) => ({
  config: null,
  models: [],
  isLoading: true,
  isLoadingModels: false,
  isDiscovering: false,
  error: null,

  initialize: async () => {
    set({ isLoading: true, error: null });

    try {
      // Initialize router from database in backend
      await invoke("initialize_router");

      // Load config and models in parallel
      await Promise.all([get().loadConfig(), get().loadModels()]);
    } catch (error) {
      console.error("[RouterStore] Failed to initialize:", error);
      set({
        config: createDefaultConfig(),
        isLoading: false,
        error: extractErrorMessage(error),
      });
    }
  },

  loadConfig: async () => {
    try {
      const config = await invoke<RouterConfig>("get_router_config");
      set({ config, error: null });
    } catch (error) {
      console.error("[RouterStore] Failed to load config:", error);
      set({ config: createDefaultConfig(), error: extractErrorMessage(error) });
    }
  },

  loadModels: async () => {
    set({ isLoadingModels: true });

    try {
      const models = await invoke<DiscoveredModel[]>("get_discovered_models");
      set({ models, isLoadingModels: false, error: null });
    } catch (error) {
      console.error("[RouterStore] Failed to load models:", error);
      set({ models: [], isLoadingModels: false, error: extractErrorMessage(error) });
    }
  },

  setProfile: async (profile: RoutingProfile) => {
    try {
      await invoke("set_router_profile", { profile });
      set((state) => ({
        config: state.config ? { ...state.config, activeProfile: profile } : null,
        error: null,
      }));
    } catch (error) {
      console.error("[RouterStore] Failed to set profile:", error);
      set({ error: extractErrorMessage(error) });
      throw error;
    }
  },

  setTaskOverride: async (task: TaskType, modelId: string) => {
    try {
      await invoke("set_task_override", { task, modelId });
      // Reload config to get updated overrides
      await get().loadConfig();
    } catch (error) {
      console.error("[RouterStore] Failed to set task override:", error);
      set({ error: extractErrorMessage(error) });
      throw error;
    }
  },

  clearTaskOverride: async (task: TaskType) => {
    try {
      await invoke("clear_task_override", { task });
      // Reload config to get updated overrides
      await get().loadConfig();
    } catch (error) {
      console.error("[RouterStore] Failed to clear task override:", error);
      set({ error: extractErrorMessage(error) });
      throw error;
    }
  },

  discoverModels: async (providerId: string, baseUrl: string, apiKey?: string) => {
    set({ isDiscovering: true, error: null });

    try {
      const count = await invoke<number>("discover_models", {
        providerId,
        baseUrl,
        apiKey: apiKey ?? null,
      });

      // Reload models after discovery
      await get().loadModels();

      set({ isDiscovering: false });
      return count;
    } catch (error) {
      console.error("[RouterStore] Failed to discover models:", error);
      set({ isDiscovering: false, error: extractErrorMessage(error) });
      throw error;
    }
  },

  routeMessage: async (message: string) => {
    try {
      const modelId = await invoke<string | null>("route_message", { message });
      return modelId;
    } catch (error) {
      console.error("[RouterStore] Failed to route message:", error);
      set({ error: extractErrorMessage(error) });
      return null;
    }
  },

  routeMessageWithModalities: async (message: string, modalities: ModelModality[]) => {
    try {
      const modelId = await invoke<string | null>("route_message_with_modalities", {
        message,
        modalities,
      });
      return modelId;
    } catch (error) {
      console.error("[RouterStore] Failed to route message with modalities:", error);
      set({ error: extractErrorMessage(error) });
      return null;
    }
  },

  getAvailableModels: async () => {
    try {
      const models = await invoke<string[]>("get_available_models");
      return models;
    } catch (error) {
      console.error("[RouterStore] Failed to get available models:", error);
      return [];
    }
  },

  isProviderAvailable: async (providerId: string, baseUrl: string, apiKey?: string) => {
    try {
      const available = await invoke<boolean>("is_provider_available", {
        providerId,
        baseUrl,
        apiKey: apiKey ?? null,
      });
      return available;
    } catch (error) {
      console.error("[RouterStore] Failed to check provider availability:", error);
      return false;
    }
  },

  reloadModels: async () => {
    try {
      const count = await invoke<number>("reload_models");
      await get().loadModels();
      return count;
    } catch (error) {
      console.error("[RouterStore] Failed to reload models:", error);
      set({ error: extractErrorMessage(error) });
      return 0;
    }
  },

  getModelCount: async () => {
    try {
      const count = await invoke<number>("get_model_count");
      return count;
    } catch (error) {
      console.error("[RouterStore] Failed to get model count:", error);
      return 0;
    }
  },

  getModelsByProvider: (providerId: string) => {
    return get().models.filter((m) => m.providerId === providerId);
  },

  getModelsByCostTier: (tier: "low" | "medium" | "high") => {
    return get().models.filter((m) => m.costTier === tier && m.isActive);
  },

  getModelsByModality: (modality: ModelModality) => {
    return get().models.filter(
      (m) => m.isActive && m.modalities.includes(modality)
    );
  },

  clearError: () => {
    set({ error: null });
  },
}));

/**
 * Helper to get the display name for a routing profile
 */
export function getProfileDisplayName(profile: RoutingProfile): string {
  switch (profile) {
    case "eco":
      return "Eco (Cost-Effective)";
    case "balanced":
      return "Balanced";
    case "premium":
      return "Premium (Best Quality)";
    default:
      return profile;
  }
}

/**
 * Helper to get the display name for a task type
 */
export function getTaskDisplayName(task: TaskType): string {
  switch (task) {
    case "code":
      return "Code & Programming";
    case "general":
      return "General Conversation";
    case "fast":
      return "Quick Responses";
    case "creative":
      return "Creative Writing";
    case "analysis":
      return "Analysis & Research";
    case "other":
      return "Other Tasks";
    default:
      return task;
  }
}

/**
 * Helper to get the display name for a modality
 */
export function getModalityDisplayName(modality: ModelModality): string {
  switch (modality) {
    case "text":
      return "Text";
    case "image":
      return "Image Understanding";
    case "image_generation":
      return "Image Generation";
    case "audio_transcription":
      return "Audio Transcription";
    case "audio_generation":
      return "Audio Generation";
    case "video":
      return "Video";
    case "embedding":
      return "Embeddings";
    default:
      return modality;
  }
}
