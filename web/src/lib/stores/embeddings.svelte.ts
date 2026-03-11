import { apiGet, apiPost } from "$lib/api/client";

export interface EmbeddingStatus {
  provider: string;
  model: string;
  dimensions: number;
  model_available: boolean;
}

export interface EmbedTestResult {
  success: boolean;
  dimensions?: number;
  latency_ms: number;
  error?: string;
}

function createEmbeddingsStore() {
  let status = $state<EmbeddingStatus>({
    provider: "none",
    model: "",
    dimensions: 0,
    model_available: true,
  });
  let loading = $state(false);

  return {
    get status() {
      return status;
    },
    get loading() {
      return loading;
    },

    async loadStatus() {
      loading = true;
      try {
        status = await apiGet<EmbeddingStatus>("/embeddings/status");
      } finally {
        loading = false;
      }
    },

    async refreshStatus() {
      status = await apiGet<EmbeddingStatus>("/embeddings/status");
    },

    async test(): Promise<EmbedTestResult> {
      return apiPost<EmbedTestResult>("/embeddings/test", {});
    },

    async download(): Promise<{ status: string; model: string }> {
      return apiPost("/embeddings/download", {});
    },

    async reindex(): Promise<{ status: string }> {
      return apiPost("/embeddings/reindex", {});
    },
  };
}

export const embeddingsStore = createEmbeddingsStore();
