import { useCallback } from "react";

import { KeychainStorage } from "@/lib/keychain-storage";
import { getVerbosity } from "@/lib/settings";
import { getVercelProvider, useLLMStore } from "@/stores/llm";

/**
 * Hook to fetch AI explanation configuration for a workspace.
 *
 * Returns a memoized callback that fetches:
 * - API key from the OS keychain
 * - Model configuration (global from LLM store)
 * - Current verbosity setting from app settings
 *
 * @param workspaceId - The workspace ID to get configuration for
 * @returns A callback function that returns the explanation configuration
 */
export function useExplanationConfig(_workspaceId: string) {
  const config = useLLMStore((state) => state.config);

  return useCallback(async () => {
    const providerId = getVercelProvider();
    const apiKey = await KeychainStorage.getApiKey(providerId);
    const model = config?.modelId || "openai/gpt-4o";
    const verbosity = await getVerbosity();
    return { apiKey, model, verbosity };
  }, [config]);
}
