import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

import { useSettings } from "@/stores/settings";
import { useLLMStore } from "@/stores/llm";

/**
 * Component to initialize all Zustand stores on app mount
 */
export function StoreInitializer() {
  const initialize = useSettings((state) => state.initialize);
  const initializeLLM = useLLMStore((state) => state.initialize);

  useEffect(() => {
    const initializeStores = async () => {
      try {
        // Initialize settings
        await initialize();
        // Initialize LLM store
        await initializeLLM();

        // Close splash screen after stores are initialized
        await invoke("close_splashscreen");
      } catch (err) {
        console.error("Failed to initialize:", err);
        // Close splash screen even on error
        invoke("close_splashscreen").catch(console.error);
      }
    };

    initializeStores();
  }, [initialize, initializeLLM]);

  return null;
}
