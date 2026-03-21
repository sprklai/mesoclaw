import { apiGet, apiPost, apiPut, apiDelete } from "$lib/api/client";

export interface ModelInfo {
  id: string;
  provider_id: string;
  model_id: string;
  display_name: string;
  context_limit: number | null;
  is_custom: boolean;
  is_active: boolean;
}

export interface ProviderWithKeyStatus {
  id: string;
  name: string;
  base_url: string;
  requires_api_key: boolean;
  is_active: boolean;
  is_user_defined: boolean;
  models: ModelInfo[];
  has_api_key: boolean;
}

export interface ConfiguredModel {
  value: string; // "provider_id:model_id"
  label: string; // "Provider / Model"
}

export interface DefaultModel {
  provider_id: string;
  model_id: string;
}

const SELECTED_MODEL_KEY = "zenii:selectedModel";

function createProvidersStore() {
  let providers = $state<ProviderWithKeyStatus[]>([]);
  let loading = $state(false);
  let selectedModel = $state(
    typeof localStorage !== "undefined"
      ? (localStorage.getItem(SELECTED_MODEL_KEY) ?? "")
      : "",
  );
  let defaultModel = $state<DefaultModel | null>(null);

  return {
    get providers() {
      return providers;
    },
    get loading() {
      return loading;
    },
    get selectedModel() {
      return selectedModel;
    },
    set selectedModel(value: string) {
      selectedModel = value;
      if (typeof localStorage !== "undefined") {
        if (value) {
          localStorage.setItem(SELECTED_MODEL_KEY, value);
        } else {
          localStorage.removeItem(SELECTED_MODEL_KEY);
        }
      }
    },
    get defaultModel() {
      return defaultModel;
    },
    get configuredModels(): ConfiguredModel[] {
      return providers
        .filter((p) => p.has_api_key && p.models.length > 0)
        .flatMap((p) =>
          p.models.map((m) => ({
            value: `${p.id}:${m.model_id}`,
            label: `${p.name} / ${m.display_name}`,
          })),
        );
    },

    get hasUsableModel(): boolean {
      return providers.some((p) => p.has_api_key && p.models.length > 0);
    },

    async load() {
      loading = true;
      try {
        providers = await apiGet<ProviderWithKeyStatus[]>(
          "/providers/with-key-status",
        );
        const models = this.configuredModels;
        // Only clear selection if it references a model that no longer exists
        if (selectedModel && !models.some((m) => m.value === selectedModel)) {
          this.selectedModel = "";
        }
      } catch (e) {
        console.error("[ProvidersStore] Failed to load providers:", e);
        providers = [];
      } finally {
        loading = false;
      }
    },

    async loadDefault() {
      try {
        const result = await apiGet<DefaultModel | null>("/providers/default");
        defaultModel = result;
        if (result && !selectedModel) {
          const defaultValue = `${result.provider_id}:${result.model_id}`;
          // Only set if the default model is actually available (has API key)
          if (this.configuredModels.some((m) => m.value === defaultValue)) {
            this.selectedModel = defaultValue;
          }
        }
      } catch {
        defaultModel = null;
      }
      // Final fallback: first available model if still nothing selected
      if (!selectedModel && this.configuredModels.length > 0) {
        this.selectedModel = this.configuredModels[0].value;
      }
    },

    async setDefault(providerId: string, modelId: string) {
      await apiPut("/providers/default", {
        provider_id: providerId,
        model_id: modelId,
      });
      defaultModel = { provider_id: providerId, model_id: modelId };
    },

    async setApiKey(providerId: string, value: string) {
      await apiPost("/credentials", {
        key: `api_key:${providerId}`,
        value,
      });
      await this.load();
    },

    async removeApiKey(providerId: string) {
      await apiDelete(`/credentials/api_key:${providerId}`);
      await this.load();
    },

    async addModel(providerId: string, modelId: string) {
      await apiPost(`/providers/${providerId}/models`, {
        model_id: modelId,
        display_name: modelId,
      });
      await this.load();
    },

    async deleteModel(providerId: string, modelId: string) {
      await apiDelete(
        `/providers/${encodeURIComponent(providerId)}/models/${encodeURIComponent(modelId)}`,
      );
      await this.load();
    },

    async addProvider(
      id: string,
      name: string,
      baseUrl: string,
      requiresApiKey: boolean,
    ) {
      await apiPost("/providers", {
        id,
        name,
        base_url: baseUrl,
        requires_api_key: requiresApiKey,
        models: [],
      });
      await this.load();
    },

    async deleteProvider(id: string) {
      await apiDelete(`/providers/${id}`);
      await this.load();
    },

    async testConnection(
      providerId: string,
    ): Promise<{ success: boolean; message: string; latency_ms?: number }> {
      return apiPost(`/providers/${providerId}/test`, {});
    },
  };
}

export const providersStore = createProvidersStore();
