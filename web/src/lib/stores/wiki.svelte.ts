import { apiGet, apiPost } from "$lib/api/client";

export interface WikiPage {
  slug: string;
  title: string;
  page_type: string;
  tags: string[];
  sources: string[];
  updated: string;
  tldr: string;
  body: string;
  wikilinks: string[];
}

export interface WikiNode {
  id: string;
  label: string;
  page_type: string;
}

export interface WikiEdge {
  from: string;
  to: string;
}

export interface WikiGraph {
  nodes: WikiNode[];
  edges: WikiEdge[];
}

function createWikiStore() {
  let pages = $state<WikiPage[]>([]);
  let loading = $state(false);
  let syncing = $state(false);
  let graph = $state<WikiGraph | null>(null);
  let graphLoading = $state(false);

  return {
    get pages() {
      return pages;
    },
    get loading() {
      return loading;
    },
    get syncing() {
      return syncing;
    },
    get graph() {
      return graph;
    },
    get graphLoading() {
      return graphLoading;
    },

    async load() {
      loading = true;
      try {
        pages = await apiGet<WikiPage[]>("/wiki");
      } finally {
        loading = false;
      }
    },

    async search(q: string) {
      loading = true;
      try {
        pages = await apiGet<WikiPage[]>(
          `/wiki/search?q=${encodeURIComponent(q)}`,
        );
      } finally {
        loading = false;
      }
    },

    async getPage(slug: string): Promise<WikiPage | null> {
      try {
        return await apiGet<WikiPage>(`/wiki/${encodeURIComponent(slug)}`);
      } catch {
        return null;
      }
    },

    async sync(): Promise<number> {
      syncing = true;
      try {
        const res = await apiPost<{ synced: number }>("/wiki/sync");
        return res.synced;
      } finally {
        syncing = false;
      }
    },

    async loadGraph(): Promise<WikiGraph> {
      graphLoading = true;
      try {
        const g = await apiGet<WikiGraph>("/wiki/graph");
        graph = g;
        return g;
      } finally {
        graphLoading = false;
      }
    },

    async ingest(
      filename: string,
      content: string,
    ): Promise<{ slug: string; status: string }> {
      return apiPost("/wiki/ingest", { filename, content });
    },

    clear() {
      pages = [];
      graph = null;
    },
  };
}

export const wikiStore = createWikiStore();
