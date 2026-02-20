/**
 * Zustand store for the MesoClaw identity / persona system.
 *
 * Manages the seven identity files that define the agent's personality,
 * operating instructions, and boot behaviour.
 *
 * Identity CRUD is served via the gateway REST API (/api/v1/identity/*).
 * `getSystemPrompt` still uses Tauri IPC (no gateway endpoint yet).
 */

import { create } from "zustand";

import {
  type IdentityFileInfo,
  getIdentityFile,
  getSystemPrompt,
  listIdentityFiles,
  updateIdentityFile,
} from "@/lib/tauri/identity";
import { extractErrorMessage } from "@/lib/error-utils";
import { withStoreLoading } from "@/lib/store-utils";

interface IdentityState {
  /** Metadata for all identity files */
  files: IdentityFileInfo[];
  /** In-memory cache: fileName → content */
  contentCache: Record<string, string>;
  /** Assembled system prompt (built from all files) */
  systemPrompt: string | null;
  isLoading: boolean;
  error: string | null;

  // Actions
  loadFiles: () => Promise<void>;
  getFileContent: (fileName: string) => Promise<string>;
  saveFile: (fileName: string, content: string) => Promise<void>;
  loadSystemPrompt: () => Promise<void>;
  clearError: () => void;
}

export const useIdentityStore = create<IdentityState>((set, get) => ({
  files: [],
  contentCache: {},
  systemPrompt: null,
  isLoading: false,
  error: null,

  loadFiles: async () => {
    await withStoreLoading(set, async () => {
      const files = await listIdentityFiles();
      set({ files });
      return files;
    });
  },

  getFileContent: async (fileName: string) => {
    // Return from cache if available.
    const cached = get().contentCache[fileName];
    if (cached !== undefined) {
      return cached;
    }
    const content = await getIdentityFile(fileName);
    set((state) => ({
      contentCache: { ...state.contentCache, [fileName]: content },
    }));
    return content;
  },

  saveFile: async (fileName: string, content: string) => {
    await updateIdentityFile(fileName, content);
    // Update local cache immediately.
    set((state) => ({
      contentCache: { ...state.contentCache, [fileName]: content },
    }));
  },

  loadSystemPrompt: async () => {
    try {
      const systemPrompt = await getSystemPrompt();
      set({ systemPrompt });
    } catch (err) {
      set({ error: extractErrorMessage(err) });
    }
  },

  clearError: () => set({ error: null }),
}));
