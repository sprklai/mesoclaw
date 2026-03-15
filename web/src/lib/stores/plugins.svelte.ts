import { apiGet, apiPost, apiDelete, apiPut } from "$lib/api/client";

export interface PluginListItem {
  name: string;
  version: string;
  description: string;
  enabled: boolean;
  tools_count: number;
  skills_count: number;
}

export interface PluginDetail {
  manifest: {
    plugin: {
      name: string;
      version: string;
      description: string;
      author?: string;
      license?: string;
      homepage?: string;
    };
    tools: { name: string; description: string }[];
    skills: { name: string; file: string }[];
  };
  enabled: boolean;
  installed_at: string;
  source: Record<string, unknown>;
}

export interface AvailablePlugin {
  name: string;
  version: string;
  description: string;
  author: string | null;
  tools_count: number;
  skills_count: number;
  installed: boolean;
}

function createPluginsStore() {
  let plugins = $state<PluginListItem[]>([]);
  let available = $state<AvailablePlugin[]>([]);
  let repoUrl = "";
  let loading = $state(false);
  let installing = $state(false);
  let browsing = $state(false);
  let error = $state<string | null>(null);

  return {
    get plugins() {
      return plugins;
    },
    get available() {
      return available;
    },
    get loading() {
      return loading;
    },
    get installing() {
      return installing;
    },
    get browsing() {
      return browsing;
    },
    get error() {
      return error;
    },

    async load() {
      loading = true;
      error = null;
      try {
        plugins = await apiGet<PluginListItem[]>("/plugins");
      } catch (e) {
        error = e instanceof Error ? e.message : "Failed to load plugins";
        plugins = [];
      } finally {
        loading = false;
      }
    },

    async loadAvailable() {
      browsing = true;
      error = null;
      try {
        const resp = await apiGet<{
          repo_url: string;
          plugins: AvailablePlugin[];
        }>("/plugins/available");
        available = resp.plugins;
        repoUrl = resp.repo_url;
      } catch (e) {
        error =
          e instanceof Error ? e.message : "Failed to fetch plugin catalog";
        available = [];
      } finally {
        browsing = false;
      }
    },

    async installSelected(names: string[]): Promise<boolean> {
      installing = true;
      error = null;
      try {
        for (const name of names) {
          const source = `${repoUrl}#plugins/${name}`;
          await apiPost("/plugins/install", {
            source,
            local: false,
            all: false,
          });
        }
        await this.load();
        await this.loadAvailable();
        return true;
      } catch (e) {
        error = e instanceof Error ? e.message : "Install failed";
        await this.load();
        await this.loadAvailable();
        return false;
      } finally {
        installing = false;
      }
    },

    async install(
      source: string,
      local: boolean,
      all = false,
    ): Promise<boolean> {
      installing = true;
      error = null;
      try {
        await apiPost("/plugins/install", { source, local, all });
        await this.load();
        return true;
      } catch (e) {
        error = e instanceof Error ? e.message : "Install failed";
        return false;
      } finally {
        installing = false;
      }
    },

    async remove(name: string): Promise<boolean> {
      error = null;
      try {
        await apiDelete(`/plugins/${encodeURIComponent(name)}`);
        await this.load();
        return true;
      } catch (e) {
        error = e instanceof Error ? e.message : "Remove failed";
        return false;
      }
    },

    async toggle(name: string): Promise<boolean> {
      error = null;
      try {
        await apiPut(`/plugins/${encodeURIComponent(name)}/toggle`, {});
        await this.load();
        return true;
      } catch (e) {
        error = e instanceof Error ? e.message : "Toggle failed";
        return false;
      }
    },

    async update(name: string): Promise<boolean> {
      error = null;
      try {
        await apiPost(`/plugins/${encodeURIComponent(name)}/update`, {});
        await this.load();
        return true;
      } catch (e) {
        error = e instanceof Error ? e.message : "Update failed";
        return false;
      }
    },

    async getDetail(name: string): Promise<PluginDetail | null> {
      try {
        return await apiGet<PluginDetail>(
          `/plugins/${encodeURIComponent(name)}`,
        );
      } catch {
        return null;
      }
    },
  };
}

export const pluginsStore = createPluginsStore();
