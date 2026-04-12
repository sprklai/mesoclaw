import {
  api,
  apiDelete,
  apiGet,
  apiPost,
  apiPut,
  getToken,
  getBaseUrl,
  ZeniiApiError,
} from "$lib/api/client";

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
  pages: string[]; // slugs of pages generated from this source
}

export interface FixedIssue {
  kind: string;
  slug: string;
  action: string;
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

// M13: sequence counters — one per operation that overwrites shared state
let loadSeq = 0;
let searchSeq = 0;
let loadGraphSeq = 0;
let getPageSeq = 0;

// M3: abort controllers for long-running operations
let ingestController: AbortController | null = null;
let queryController: AbortController | null = null;
let regenerateController: AbortController | null = null;

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
  let lintFixed = $state<FixedIssue[]>([]);

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
    get lintFixed() {
      return lintFixed;
    },

    async load() {
      const seq = ++loadSeq;
      loading = true;
      try {
        // L4: GET /wiki now returns paginated response { pages, total, limit, offset }
        const data = await apiGet<{
          pages: WikiPage[];
          total: number;
          limit: number;
          offset: number;
        }>("/wiki?limit=200&offset=0");
        if (seq !== loadSeq) return;
        pages = [...new Map(data.pages.map((p) => [p.slug, p])).values()];
      } finally {
        if (seq === loadSeq) loading = false;
      }
    },

    async search(q: string) {
      const seq = ++searchSeq;
      loading = true;
      try {
        const data = await apiGet<WikiPage[]>(
          `/wiki/search?q=${encodeURIComponent(q)}`,
        );
        if (seq !== searchSeq) return;
        pages = [...new Map(data.map((p) => [p.slug, p])).values()];
      } finally {
        if (seq === searchSeq) loading = false;
      }
    },

    async getPage(slug: string): Promise<WikiPage | null> {
      const seq = ++getPageSeq;
      try {
        const data = await apiGet<WikiPage>(
          `/wiki/${encodeURIComponent(slug)}`,
        );
        if (seq !== getPageSeq) return null;
        return data;
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
      const seq = ++loadGraphSeq;
      graphLoading = true;
      try {
        const g = await apiGet<WikiGraph>("/wiki/graph");
        if (seq !== loadGraphSeq) return g;
        graph = g;
        return g;
      } finally {
        if (seq === loadGraphSeq) graphLoading = false;
      }
    },

    async ingest(
      filename: string,
      content: string,
    ): Promise<{ slug: string; page_count: number; message: string }> {
      ingestController?.abort();
      ingestController = new AbortController();
      try {
        const res = await api<{
          pages: WikiPage[];
          primary_slug: string;
          message: string;
        }>("/wiki/ingest", {
          method: "POST",
          body: JSON.stringify({ filename, content }),
          signal: ingestController.signal,
          timeout: 120_000,
        });
        return {
          slug: res.primary_slug,
          page_count: res.pages.length,
          message: res.message,
        };
      } catch (e) {
        if (e instanceof Error && e.name === "AbortError") throw e;
        throw e;
      }
    },

    /** Upload a binary file (PDF/DOCX/image) via multipart to /wiki/upload. */
    async uploadBinary(
      file: File,
    ): Promise<{ slug: string; page_count: number; message: string }> {
      ingestController?.abort();
      ingestController = new AbortController();
      const formData = new FormData();
      formData.append("file", file);
      const token = getToken();
      const baseUrl = getBaseUrl();
      const headers: Record<string, string> = {};
      if (token) headers["Authorization"] = `Bearer ${token}`;
      // Do NOT set Content-Type — browser sets multipart boundary automatically
      let response: Response;
      try {
        response = await fetch(`${baseUrl}/wiki/upload`, {
          method: "POST",
          headers,
          body: formData,
          signal: ingestController.signal,
        });
      } catch (e) {
        if (e instanceof Error && e.name === "AbortError") throw e;
        throw e;
      }
      if (!response.ok) {
        let errorCode = "ZENII_UNKNOWN";
        let details = response.statusText;
        try {
          const body = await response.json();
          errorCode = body.error_code ?? errorCode;
          details = body.message ?? body.error ?? details;
        } catch {
          /* not JSON */
        }
        throw new ZeniiApiError(response.status, errorCode, details);
      }
      const text = await response.text();
      const res = text
        ? (JSON.parse(text) as {
            pages: WikiPage[];
            primary_slug: string;
            message: string;
          })
        : { pages: [], primary_slug: "", message: "" };
      return {
        slug: res.primary_slug,
        page_count: res.pages.length,
        message: res.message,
      };
    },

    async query(
      question: string,
      save?: boolean,
      model?: string,
    ): Promise<QueryResult> {
      queryController?.abort();
      queryController = new AbortController();
      querying = true;
      try {
        return await api<QueryResult>("/wiki/query", {
          method: "POST",
          body: JSON.stringify({ question, save, model }),
          signal: queryController.signal,
          timeout: 120_000,
        });
      } catch (e) {
        if (e instanceof Error && e.name === "AbortError") throw e;
        throw e;
      } finally {
        querying = false;
      }
    },

    async lint(): Promise<LintIssue[]> {
      linting = true;
      try {
        const res = await apiPost<{
          issues: LintIssue[];
          fixed: FixedIssue[];
          summary: string;
        }>("/wiki/lint", { auto_fix: true });
        lintIssues = res.issues;
        lintFixed = res.fixed ?? [];
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

    async deleteSource(
      filename: string,
      model?: string,
    ): Promise<DeleteSourceResult> {
      const path = model
        ? `/wiki/sources/${encodeURIComponent(filename)}?model=${encodeURIComponent(model)}`
        : `/wiki/sources/${encodeURIComponent(filename)}`;
      // M9: snapshot before mutation so we can revert if the reload fails
      const prevSources = sources;
      const result = await apiDelete<DeleteSourceResult>(path);
      // M8: optimistic filter after delete — callers do NOT need to call load() again;
      // this.load() below overwrites pages with authoritative server state.
      sources = sources.filter((s) => s.filename !== filename);
      try {
        await this.load();
      } catch (e) {
        // M9: reload failed — revert optimistic filter so UI stays in sync with last known state
        sources = prevSources;
        throw e;
      }
      // Refresh graph if it was previously loaded (I6 fix)
      if (graph !== null) {
        try {
          await this.loadGraph();
        } catch {
          /* non-fatal */
        }
      }
      return result;
    },

    async regenerate(model?: string): Promise<RegenerateResult> {
      regenerateController?.abort();
      regenerateController = new AbortController();
      regenerating = true;
      try {
        const result = await api<RegenerateResult>("/wiki/regenerate", {
          method: "POST",
          body: JSON.stringify(model ? { model } : {}),
          signal: regenerateController.signal,
          timeout: 300_000,
        });
        // M8: self-refreshes pages and sources — callers must NOT call load()/fetchSources() again
        await this.load();
        await this.fetchSources();
        // Refresh graph if it was previously loaded (I6 fix)
        if (graph !== null) {
          try {
            await this.loadGraph();
          } catch {
            /* non-fatal */
          }
        }
        return result;
      } catch (e) {
        if (e instanceof Error && e.name === "AbortError") throw e;
        throw e;
      } finally {
        regenerating = false;
      }
    },

    async regenerateSource(filename: string, model?: string): Promise<void> {
      regenerating = true;
      try {
        await apiPost(
          `/wiki/sources/${encodeURIComponent(filename)}/regenerate`,
          model ? { model } : {},
          { timeout: 300_000 },
        );
        // M8: self-refreshes pages and sources — callers must NOT call load()/fetchSources() again
        await this.load();
        await this.fetchSources();
        // Refresh graph if it was previously loaded (I6 fix)
        if (graph !== null) {
          try {
            await this.loadGraph();
          } catch {
            /* non-fatal */
          }
        }
      } finally {
        regenerating = false;
      }
    },

    async fetchPrompt(): Promise<string> {
      const res = await apiGet<{ content: string }>("/wiki/prompt");
      return res.content;
    },

    async savePrompt(content: string): Promise<void> {
      await apiPut("/wiki/prompt", { content });
    },

    async deleteAllSources(): Promise<number> {
      const res = await apiDelete<{ deleted: number }>("/wiki/sources");
      sources = [];
      pages = [];
      graph = null;
      await this.load();
      return res.deleted;
    },

    async deleteAllPages(): Promise<number> {
      const res = await apiDelete<{ deleted: number }>("/wiki/pages");
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
