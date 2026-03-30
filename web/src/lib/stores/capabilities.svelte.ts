import { apiGet } from "$lib/api/client";

/**
 * Tracks which backend feature gates are active.
 * Feature-gated UI elements (e.g. channel_send node in workflow palette)
 * are hidden when their capability is absent.
 */
function createCapabilitiesStore() {
  let capabilities = $state<Record<string, boolean>>({});

  return {
    get capabilities() {
      return capabilities;
    },

    /** Fetch feature flags from backend endpoints. */
    async load() {
      try {
        await apiGet<unknown[]>("/channels");
        capabilities = { channels: true };
      } catch {
        capabilities = { channels: false };
      }
    },
  };
}

export const capabilitiesStore = createCapabilitiesStore();
