import { apiGet, apiPost, apiPut, apiDelete } from "$lib/api/client";

export interface MemoryEntry {
  key: string;
  content: string;
  category: string;
  score: number;
  created_at: string;
}

export interface UserObservation {
  id: string;
  category: string;
  key: string;
  value: string;
  confidence: number;
  created_at: string;
  updated_at: string;
}

function createMemoryStore() {
  let entries = $state<MemoryEntry[]>([]);
  let observations = $state<UserObservation[]>([]);
  let loading = $state(false);
  let loadVersion = 0;

  return {
    get entries() {
      return entries;
    },
    get observations() {
      return observations;
    },
    get loading() {
      return loading;
    },

    async loadAll() {
      const version = ++loadVersion;
      loading = true;
      try {
        const [memResult, obsResult] = await Promise.allSettled([
          apiGet<MemoryEntry[]>("/memory?q=&limit=50&offset=0"),
          apiGet<{ observations: UserObservation[] }>("/user/observations"),
        ]);
        if (version !== loadVersion) return; // Stale load from re-navigation
        entries = memResult.status === "fulfilled" ? memResult.value : [];
        observations =
          obsResult.status === "fulfilled" ? obsResult.value.observations : [];
      } finally {
        if (version === loadVersion) {
          loading = false;
        }
      }
    },

    async search(query: string, limit = 20, offset = 0) {
      loading = true;
      try {
        const params = new URLSearchParams({
          q: query,
          limit: String(limit),
          offset: String(offset),
        });
        entries = await apiGet<MemoryEntry[]>(`/memory?${params}`);
      } finally {
        loading = false;
      }
    },

    async getByKey(key: string) {
      return apiGet<MemoryEntry>(`/memory/${encodeURIComponent(key)}`);
    },

    async create(key: string, content: string, category?: string) {
      await apiPost("/memory", { key, content, category });
      entries = [
        {
          key,
          content,
          category: (category ?? "core").toLowerCase(),
          score: 1,
          created_at: new Date().toISOString().replace("T", " ").slice(0, 19),
        },
        ...entries,
      ];
    },

    async update(key: string, content: string, category?: string) {
      await apiPut(`/memory/${encodeURIComponent(key)}`, { content, category });
      entries = entries.map((e) =>
        e.key === key ? { ...e, content, category: category ?? e.category } : e,
      );
    },

    async remove(key: string) {
      await apiDelete(`/memory/${encodeURIComponent(key)}`);
      entries = entries.filter((e) => e.key !== key);
    },

    clear() {
      entries = [];
      observations = [];
    },
  };
}

export const memoryStore = createMemoryStore();
