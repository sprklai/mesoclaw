/**
 * Centralized AI model definitions for the application.
 * All model configurations should be imported from this file.
 */

/**
 * AI Provider configuration from backend.
 * Backend uses #[serde(rename_all = "camelCase")] for AIProviderData.
 */
export interface AIProvider {
  id: string;
  name: string;
  baseUrl: string;
  requiresApiKey: boolean;
  isActive: boolean;
  isUserDefined: boolean;
}

/**
 * AI Model configuration from backend.
 * Backend uses #[serde(rename_all = "camelCase")] for AIModelData.
 */
export interface AIModel {
  id: string;
  providerId: string;
  modelId: string; // Actual ID for API calls
  displayName: string;
  contextLimit?: number;
  isCustom: boolean;
  isActive: boolean;
}

/**
 * Provider with its associated models (loaded from backend).
 * Backend uses #[serde(rename_all = "camelCase")] for ProviderWithModels.
 */
export interface ProviderWithModels extends AIProvider {
  models: AIModel[];
}

/**
 * Initial model specification for creating user-defined providers.
 */
export interface InitialModelSpec {
  modelId: string;
  displayName?: string;
}

/**
 * Global default model configuration.
 */
export interface GlobalDefaultModel {
  providerId: string;
  modelId: string;
}

/**
 * Provider with API key status for settings UI.
 */
export interface ProviderWithKeyStatus extends AIProvider {
  hasApiKey: boolean;
  models: AIModel[];
}

/**
 * Test result for provider connection testing.
 */
export interface TestResult {
  success: boolean;
  latencyMs?: number;
  error?: string;
  model?: string;
}

/**
 * Legacy ModelOption interface for backward compatibility
 * @deprecated Use ProviderWithModels instead
 */
export interface ModelOption {
  value: string;
  label: string;
  provider: string;
}

/**
 * Default provider ID
 */
export const DEFAULT_PROVIDER = "vercel-ai-gateway";

/**
 * Default model ID
 * Note: This is the model ID, not the full provider/model format
 */
export const DEFAULT_MODEL = "google/gemini-3-flash";

/**
 * Legacy available models (static, for fallback only)
 * @deprecated Use providersWithModels from backend instead
 */
export const AVAILABLE_MODELS: ModelOption[] = [
  { value: "openai/gpt-4o", label: "GPT-4o", provider: "openai" },
  { value: "openai/gpt-4", label: "GPT-4", provider: "openai" },
  { value: "openai/gpt-3.5-turbo", label: "GPT-3.5 Turbo", provider: "openai" },
  {
    value: "anthropic/claude-opus-4.5",
    label: "Claude Opus 4.5",
    provider: "anthropic",
  },
  {
    value: "anthropic/claude-sonnet-4.5",
    label: "Claude Sonnet 4.5",
    provider: "anthropic",
  },
  {
    value: "anthropic/claude-haiku-4.5",
    label: "Claude Haiku 4.5",
    provider: "anthropic",
  },
  {
    value: "google/gemini-3-flash",
    label: "Gemini 3 Flash",
    provider: "google",
  },
  { value: "xai/grok-code-fast-1", label: "Grok Code Fast", provider: "xai" },
];

/**
 * Map of model IDs to short display names for UI.
 */
export const MODEL_SHORT_NAMES: Record<string, string> = {
  "anthropic/claude-sonnet-4.5": "Claude Sonnet",
  "anthropic/claude-haiku-4.5": "Claude Haiku",
  "anthropic/claude-opus-4.5": "Claude Opus",
  "openai/gpt-4o": "GPT-4o",
  "openai/gpt-4": "GPT-4",
  "openai/gpt-3.5-turbo": "GPT-3.5 Turbo",
  "google/gemini-3-flash": "Gemini 3 Flash",
  "xai/grok-code-fast-1": "Grok Code Fast",
};

/**
 * Provider display names (legacy, for model ID parsing)
 */
export const PROVIDER_NAMES: Record<string, string> = {
  openai: "OpenAI",
  anthropic: "Anthropic",
  google: "Google",
  xai: "xAI",
};

/**
 * Get short display name for a model ID.
 * @param modelId - The model ID (can include provider prefix like "anthropic/claude-3.5-sonnet")
 * @returns Short display name
 */
export function getModelShortName(modelId: string): string {
  return MODEL_SHORT_NAMES[modelId] || modelId.split("/").pop() || modelId;
}

/**
 * Get provider display name from model ID.
 * @param modelId - The model ID (can include provider prefix like "anthropic/claude-3.5-sonnet")
 * @returns Provider display name
 */
export function getProviderName(modelId: string): string {
  const provider = modelId.split("/")[0];
  return (
    PROVIDER_NAMES[provider] ||
    provider.charAt(0).toUpperCase() + provider.slice(1)
  );
}

/**
 * Convert AIModel to ModelOption (for backward compatibility)
 * @param model - The AIModel from backend
 * @returns ModelOption for legacy components
 */
export function aiModelToOption(model: AIModel): ModelOption {
  return {
    value: model.modelId,
    label: model.displayName,
    provider: model.providerId,
  };
}

/**
 * Find a model by ID in providers list
 * @param providers - List of providers with models
 * @param modelId - The model ID to find
 * @returns The AIModel if found, undefined otherwise
 */
export function findModelById(
  providers: ProviderWithModels[],
  modelId: string
): AIModel | undefined {
  for (const provider of providers) {
    const model = provider.models.find((m) => m.modelId === modelId);
    if (model) {
      return model;
    }
  }
  return undefined;
}

/**
 * Get all models from all providers as a flat list
 * @param providers - List of providers with models
 * @returns Flat list of all AIModel entries
 */
export function getAllModels(providers: ProviderWithModels[]): AIModel[] {
  return providers.flatMap((provider) => provider.models);
}

/**
 * Get models for a specific provider
 * @param providers - List of providers with models
 * @param providerId - The provider ID to filter by
 * @returns List of AIModel entries for the provider
 */
export function getModelsByProvider(
  providers: ProviderWithModels[],
  providerId: string
): AIModel[] {
  const provider = providers.find((p) => p.id === providerId);
  return provider?.models || [];
}

/**
 * Parse provider and model ID from a combined string
 * Handles both formats: "providerId/modelId" and just "modelId"
 * @param combined - Combined string or just model ID
 * @returns Object with providerId and modelId
 */
export function parseProviderModel(combined: string): {
  providerId: string;
  modelId: string;
} {
  const parts = combined.split("/");
  if (parts.length >= 2 && isKnownProvider(parts[0])) {
    return { providerId: parts[0], modelId: parts.slice(1).join("/") };
  }
  // Default to vercel-ai-gateway for legacy format
  return { providerId: DEFAULT_PROVIDER, modelId: combined };
}

/**
 * Check if a string is a known provider ID
 * @param id - The ID to check
 * @returns True if it's a known provider
 */
function isKnownProvider(id: string): boolean {
  return ["vercel-ai-gateway", "openrouter"].includes(id);
}
