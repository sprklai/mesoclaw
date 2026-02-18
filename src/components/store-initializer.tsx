import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

import { useSettings } from "@/stores/settings";
import { useLLMStore } from "@/stores/llm";
import { useGatewayStore } from "@/stores/gatewayStore";

/**
 * Component to initialize all Zustand stores on app mount.
 *
 * Initialization order:
 * 1. Settings + LLM config (blocking — required before splash closes)
 * 2. Gateway connection check (non-blocking — runs in background)
 */
export function StoreInitializer() {
  const initialize = useSettings((state) => state.initialize);
  const initializeLLM = useLLMStore((state) => state.initialize);
  const checkGateway = useGatewayStore((state) => state.checkConnection);

  useEffect(() => {
    const initializeStores = async () => {
      try {
        // Initialize settings and LLM config (required before UI renders).
        await initialize();
        await initializeLLM();

        // Close splash screen after critical stores are ready.
        await invoke("close_splashscreen");
      } catch (err) {
        console.error("Failed to initialize:", err);
        // Close splash screen even on error.
        invoke("close_splashscreen").catch(console.error);
      }

      // Non-blocking: probe the gateway in the background.
      // If the daemon is not running this is a no-op (sets connected: false).
      checkGateway().catch(() => {/* silently ignored */});
    };

    initializeStores();
  }, [initialize, initializeLLM, checkGateway]);

  return null;
}
