import { apiGet, apiPut } from "$lib/api/client";

function createConfigStore() {
  let config = $state<Record<string, unknown>>({});
  let loading = $state(false);
  let error = $state<string | null>(null);

  return {
    get config() {
      return config;
    },
    get loading() {
      return loading;
    },
    get error() {
      return error;
    },

    async load() {
      loading = true;
      error = null;
      try {
        config = await apiGet<Record<string, unknown>>("/config");
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        error = `Failed to load config. Is the daemon running? (${msg})`;
        console.error("configStore.load failed:", e);
      } finally {
        loading = false;
      }
    },

    async update(partial: Record<string, unknown>) {
      const result = await apiPut<{
        status: string;
        fields: Record<string, unknown>;
      }>("/config", partial);
      config = { ...config, ...partial };
      return result;
    },

    get(key: string): unknown {
      return config[key];
    },
  };
}

export const configStore = createConfigStore();
