import { apiDelete, apiGet, apiPost, apiPut } from "$lib/api/client";

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

export interface LintIssue {
  kind: string;
  page_slug: string;
  detail: string;
  fix: string;
}

export interface QueryResult {
  answer: string;
  citations: string[];
  saved_page: WikiPage | null;
}

export interface SourceRecord {
  filename: string;
  hash: string;
  active: boolean;
  last_run_id: string | null;
}

export interface DeleteSourceResult {
  filename: string;
  deleted_pages: string[];
  rebuilt_pages: string[];
  message: string;
}

export interface RegenerateResult {
  sources_processed: number;
  pages_generated: number;
  message: string;
}

function createWikiStore() {
  let pages = $state<WikiPage[]>([]);
  let loading = $state(false);
  let syncing = $state(false);
  let graph = $state<WikiGraph | null>(null);
  let graphLoading = $state(false);
  let querying = $state(false);
  let linting = $state(false);
  let lintIssues = $state<LintIssue[] | null>(null);
  let sources = $state<SourceRecord[]>([]);
  let sourcesLoading = $state(false);
  let regenerating = $state(false);

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
    get querying() {
      return querying;
    },
    get linting() {
      return linting;
    },
    get lintIssues() {
      return lintIssues;
    },
    get sources() {
      return sources;
    },
    get sourcesLoading() {
      return sourcesLoading;
    },
    get regenerating() {
      return regenerating;
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
    ): Promise<{ slug: string; page_count: number; message: string }> {
      const res = await apiPost<{
        pages: WikiPage[];
        primary_slug: string;
        message: string;
      }>("/wiki/ingest", { filename, content }, { timeout: 120_000 });
      return { slug: res.primary_slug, page_count: res.pages.length, message: res.message };
    },

    async query(
      question: string,
      save?: boolean,
      model?: string,
    ): Promise<QueryResult> {
      querying = true;
      try {
        return await apiPost<QueryResult>(
          "/wiki/query",
          { question, save, model },
          { timeout: 120_000 },
        );
      } finally {
        querying = false;
      }
    },

    async lint(): Promise<LintIssue[]> {
      linting = true;
      try {
        const res = await apiPost<{ issues: LintIssue[]; summary: string }>(
          "/wiki/lint",
        );
        lintIssues = res.issues;
        return res.issues;
      } finally {
        linting = false;
      }
    },

    async fetchSources() {
      sourcesLoading = true;
      try {
        sources = await apiGet<SourceRecord[]>("/wiki/sources");
      } finally {
        sourcesLoading = false;
      }
    },

    async deleteSource(filename: string, model?: string): Promise<DeleteSourceResult> {
      const path = model
        ? `/wiki/sources/${encodeURIComponent(filename)}?model=${encodeURIComponent(model)}`
        : `/wiki/sources/${encodeURIComponent(filename)}`;
      const result = await apiDelete<DeleteSourceResult>(path);
      // Refresh sources and pages after deletion
      sources = sources.filter((s) => s.filename !== filename);
      await this.load();
      return result;
    },

    async regenerate(model?: string): Promise<RegenerateResult> {
      regenerating = true;
      try {
        const result = await apiPost<RegenerateResult>(
          "/wiki/regenerate",
          model ? { model } : {},
          { timeout: 300_000 },
        );
        // Refresh everything after regeneration
        await this.load();
        await this.fetchSources();
        return result;
      } finally {
        regenerating = false;
      }
    },

    async fetchPrompt(): Promise<string> {
      const res = await apiGet<{ content: string }>('/wiki/prompt');
      return res.content;
    },

    async savePrompt(content: string): Promise<void> {
      await apiPut('/wiki/prompt', { content });
    },

    async deleteAllSources(): Promise<number> {
      const res = await apiDelete<{ deleted: number }>('/wiki/sources');
      sources = [];
      return res.deleted;
    },

    async deleteAllPages(): Promise<number> {
      const res = await apiDelete<{ deleted: number }>('/wiki/pages');
      pages = [];
      graph = null;
      return res.deleted;
    },

    async fetchWikiDir(): Promise<string> {
      const res = await apiGet<{ path: string }>("/wiki/dir");
      return res.path;
    },

    clear() {
      pages = [];
      graph = null;
      lintIssues = null;
    },
  };
}

export const wikiStore = createWikiStore();
