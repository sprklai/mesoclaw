/**
 * AI Provider Definitions
 *
 * This file imports provider definitions from models.json, which serves as the
 * single source of truth for all AI provider configurations.
 *
 * To add/update providers or models:
 * 1. Edit models.json at the project root
 * 2. The changes will be reflected automatically in both frontend and backend
 */

import { MODELS } from "./ai-models";

export interface ProviderConfig {
  id: string;
  name: string;
  baseUrl: string;
  requiresApiKey: boolean;
  models: string[];
}

/**
 * Standard AI providers with their base URLs and model lists
 *
 * This is dynamically generated from models.json to ensure consistency
 * across the entire application.
 *
 * Note: All providers use OpenAI-compatible protocol.
 */
export const STANDARD_PROVIDERS: Record<string, ProviderConfig> =
  Object.fromEntries(
    Object.entries(MODELS).map(([id, provider]) => [
      id,
      {
        id,
        name: provider.name,
        baseUrl: provider.baseUrl,
        requiresApiKey: provider.requiresApiKey,
        models: provider.models.map((m) => m.id),
      },
    ])
  );

/**
 * Get models for a specific provider
 */
export function getModelsForProvider(providerId: string): string[] {
  return STANDARD_PROVIDERS[providerId]?.models || [];
}

/**
 * Check if provider is local (doesn't require API key)
 */
export function isLocalProvider(providerId: string): boolean {
  const provider = STANDARD_PROVIDERS[providerId];
  return provider ? !provider.requiresApiKey : false;
}

/**
 * Get base URL for a provider
 */
export function getProviderBaseUrl(providerId: string): string {
  return STANDARD_PROVIDERS[providerId]?.baseUrl || "";
}

/**
 * Get provider configuration by ID
 */
export function getProviderConfig(
  providerId: string
): ProviderConfig | undefined {
  return STANDARD_PROVIDERS[providerId];
}
