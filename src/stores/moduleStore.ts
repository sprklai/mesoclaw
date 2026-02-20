/**
 * Zustand store for the Module Management UI.
 *
 * Mirrors the Rust ModuleManifest / ModuleEntry types and wraps the module
 * IPC commands wired to real Tauri backend commands.
 */

import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

// ─── Types (mirror Rust serde output) ────────────────────────────────────────

export type ModuleType = "tool" | "service" | "mcp";
export type RuntimeType = "native" | "docker" | "podman";

/** Lifecycle status tracked on the frontend. */
export type ModuleStatus = "stopped" | "starting" | "running" | "error";

export interface ModuleInfo {
  id: string;
  name: string;
  version: string;
  description: string;
  /** Serialised from Rust `ModuleType` via `#[serde(rename = "type")]`. */
  type: ModuleType;
}

export interface RuntimeConfig {
  /** Serialised from Rust `RuntimeType` via `#[serde(rename = "type")]`. */
  type: RuntimeType;
  command: string;
  args: string[];
  env: Record<string, string>;
  timeout_secs: number | null;
}

export interface SecurityConfig {
  allow_network: boolean;
  allow_filesystem: boolean;
  max_memory_mb: number;
}

export interface ModuleManifest {
  module: ModuleInfo;
  runtime: RuntimeConfig;
  security: SecurityConfig;
}

/** Enriched entry returned by `list_modules_command`. */
export interface ModuleEntry {
  manifest: ModuleManifest;
  status: ModuleStatus;
  healthy: boolean | null;
  errorMessage: string | null;
}

// ─── Scaffold form ────────────────────────────────────────────────────────────

export interface ScaffoldForm {
  name: string;
  moduleType: ModuleType;
  runtimeType: RuntimeType;
  command: string;
  description: string;
}

const DEFAULT_SCAFFOLD: ScaffoldForm = {
  name: "",
  moduleType: "tool",
  runtimeType: "native",
  command: "",
  description: "",
};

// ─── Store ────────────────────────────────────────────────────────────────────

interface ModuleState {
  modules: ModuleEntry[];
  loading: boolean;
  error: string | null;

  selectedId: string | null;

  scaffoldOpen: boolean;
  scaffoldForm: ScaffoldForm;
  scaffolding: boolean;

  loadModules: () => Promise<void>;
  startModule: (moduleId: string) => Promise<void>;
  stopModule: (moduleId: string) => Promise<void>;
  selectModule: (moduleId: string | null) => void;

  openScaffold: () => void;
  closeScaffold: () => void;
  updateScaffoldForm: (patch: Partial<ScaffoldForm>) => void;
  submitScaffold: () => Promise<void>;
}

export const useModuleStore = create<ModuleState>((set, get) => ({
  modules: [],
  loading: false,
  error: null,
  selectedId: null,
  scaffoldOpen: false,
  scaffoldForm: { ...DEFAULT_SCAFFOLD },
  scaffolding: false,

  loadModules: async () => {
    set({ loading: true, error: null });
    try {
      const modules = await invoke<ModuleEntry[]>("list_modules_command");
      set({ modules, loading: false });
    } catch (err) {
      set({
        modules: [],
        loading: false,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },

  startModule: async (moduleId) => {
    try {
      await invoke("start_module_command", { moduleId });
      set((s) => ({
        modules: s.modules.map((m) =>
          m.manifest.module.id === moduleId
            ? { ...m, status: "starting" as ModuleStatus }
            : m
        ),
      }));
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  stopModule: async (moduleId) => {
    try {
      await invoke("stop_module_command", { moduleId });
      set((s) => ({
        modules: s.modules.map((m) =>
          m.manifest.module.id === moduleId
            ? { ...m, status: "stopped" as ModuleStatus }
            : m
        ),
      }));
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  selectModule: (moduleId) => set({ selectedId: moduleId }),

  openScaffold: () =>
    set({ scaffoldOpen: true, scaffoldForm: { ...DEFAULT_SCAFFOLD } }),
  closeScaffold: () => set({ scaffoldOpen: false }),
  updateScaffoldForm: (patch) =>
    set((s) => ({ scaffoldForm: { ...s.scaffoldForm, ...patch } })),

  submitScaffold: async () => {
    const { scaffoldForm } = get();
    set({ scaffolding: true, error: null });
    try {
      await invoke("create_module_command", {
        name: scaffoldForm.name,
        moduleType: scaffoldForm.moduleType,
        runtimeType: scaffoldForm.runtimeType,
        command: scaffoldForm.command,
        description: scaffoldForm.description,
      });
      await get().loadModules();
      set({
        scaffoldOpen: false,
        scaffoldForm: { ...DEFAULT_SCAFFOLD },
        scaffolding: false,
      });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        scaffolding: false,
      });
    }
  },
}));
