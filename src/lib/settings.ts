import { invoke } from "@tauri-apps/api/core";

import type { Settings } from "@/lib/tauri/settings/types";

/**
 * Get the current explanation verbosity setting from app settings.
 *
 * @returns The verbosity level ("concise", "balanced", or "detailed")
 * @default "balanced" if settings cannot be retrieved
 */
export async function getVerbosity(): Promise<string> {
  try {
    const settings = await invoke<Settings>("get_app_settings");
    return settings.explanationVerbosity ?? "balanced";
  } catch {
    return "balanced";
  }
}
