/**
 * AI Model Lists
 *
 * This file imports model definitions from models.json, which serves as the
 * single source of truth for all AI model configurations.
 *
 * To add/update models:
 * 1. Edit models.json at the project root
 * 2. The changes will be reflected automatically in both frontend and backend
 *
 * Note: For custom providers, models are managed via the settings UI and
 * stored in the database.
 *
 * Ollama models are discovered dynamically at runtime.
 */

import modelDefinitions from "../../models.json";

export type ModelDefinition = {
  id: string;
  displayName: string;
  contextLimit: number;
};

export type ProviderDefinition = {
  name: string;
  baseUrl: string;
  requiresApiKey: boolean;
  models: ModelDefinition[];
};

export type ModelRegistry = {
  [providerId: string]: ProviderDefinition;
};

/**
 * Type guard to check if imported value is valid ModelRegistry
 */
function isValidModelRegistry(obj: unknown): obj is ModelRegistry {
  if (typeof obj !== "object" || obj === null) {
    return false;
  }

  const registry = obj as Record<string, unknown>;
  for (const providerId in registry) {
    const provider = registry[providerId];
    if (
      typeof provider !== "object" ||
      provider === null ||
      !("name" in provider) ||
      !("baseUrl" in provider) ||
      !("requiresApiKey" in provider) ||
      !("models" in provider) ||
      !Array.isArray(provider.models)
    ) {
      return false;
    }
  }

  return true;
}

/**
 * Validate and export model definitions
 */
if (!isValidModelRegistry(modelDefinitions)) {
  throw new Error("Invalid model definitions in models.json");
}

/**
 * Base model definitions from models.json
 */
const BASE_MODELS: ModelRegistry = modelDefinitions;

/**
 * Provider categories
 */
export const AI_GATEWAY_PROVIDERS = [
  "vercel-ai-gateway",
  "openrouter",
] as const;
export const AI_PROVIDERS = ["openai", "anthropic", "gemini"] as const;
export const LOCAL_PROVIDERS = ["ollama"] as const;

/**
 * All provider IDs
 */
export const ALL_PROVIDER_IDS = [
  ...AI_GATEWAY_PROVIDERS,
  ...AI_PROVIDERS,
  ...LOCAL_PROVIDERS,
] as const;

/**
 * Runtime Ollama models (updated dynamically via discovery)
 */
let ollamaModels: ModelDefinition[] = [];

/**
 * Get the current model definitions (including dynamic Ollama models)
 */
export function getModels(): ModelRegistry {
  return {
    ...BASE_MODELS,
    ollama: {
      ...BASE_MODELS.ollama,
      models:
        ollamaModels.length > 0 ? ollamaModels : BASE_MODELS.ollama.models,
    },
  };
}

/**
 * Export MODELS as a getter that returns the current state
 * This ensures Ollama models are always up-to-date
 */
export const MODELS = new Proxy({} as ModelRegistry, {
  get(_target, prop: string) {
    const models = getModels();
    return models[prop as keyof typeof models];
  },
  ownKeys() {
    return Object.keys(getModels());
  },
  getOwnPropertyDescriptor() {
    return {
      enumerable: true,
      configurable: true,
    };
  },
}) as ModelRegistry;

/**
 * Update Ollama models dynamically
 * Called after Ollama model discovery
 */
export function updateOllamaModels(models: ModelDefinition[]): void {
  ollamaModels = models;
  console.log(
    `[ai-models] Updated Ollama models: ${models.length} models available`
  );
}

/**
 * Get current Ollama models
 */
export function getOllamaModels(): ModelDefinition[] {
  return ollamaModels;
}

/**
 * Check if Ollama has any models
 */
export function hasOllamaModels(): boolean {
  return ollamaModels.length > 0;
}

/**
 * OpenAI Models
 */
export const OPENAI_MODELS: string[] = BASE_MODELS.openai.models.map(
  (m) => m.id
);

/**
 * Anthropic/Claude Models
 */
export const ANTHROPIC_MODELS: string[] = BASE_MODELS.anthropic.models.map(
  (m) => m.id
);

/**
 * Google Gemini Models
 */
export const GEMINI_MODELS: string[] = BASE_MODELS.gemini.models.map(
  (m) => m.id
);

/**
 * OpenRouter Models
 */
export const OPENROUTER_MODELS: string[] = BASE_MODELS.openrouter.models.map(
  (m) => m.id
);

/**
 * Vercel AI Gateway Models
 */
export const VERCEL_GATEWAY_MODELS: string[] = BASE_MODELS[
  "vercel-ai-gateway"
].models.map((m) => m.id);

/**
 * Ollama Models (dynamic)
 */
export const OLLAMA_MODELS: string[] = []; // Populated dynamically

/**
 * Get display name for a model
 */
export function getModelDisplayName(
  providerId: string,
  modelId: string
): string {
  const provider = getModels()[providerId];
  if (!provider) return modelId;

  const model = provider.models.find((m) => m.id === modelId);
  return model?.displayName || modelId;
}

/**
 * Get context limit for a model
 */
export function getModelContextLimit(
  providerId: string,
  modelId: string
): number {
  const provider = getModels()[providerId];
  if (!provider) return 128000;

  const model = provider.models.find((m) => m.id === modelId);
  return model?.contextLimit || 128000;
}

/**
 * Re-export for backward compatibility
 */
export default MODELS;
