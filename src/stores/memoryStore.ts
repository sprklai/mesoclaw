/**
 * Zustand store for the Memory Search UI.
 *
 * State: search query, results, loading flag, selected entry,
 * daily date list, selected date, and that day's raw content.
 *
 * Memory search and listing are routed through the gateway REST API so the
 * GUI and CLI share the same data path (GAP-2).  Daily memory retrieval falls
 * back to Tauri IPC (`get_daily_memory_command`) because the gateway does not
 * yet expose a per-date endpoint.
 */

import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import { gateway, type GatewayMemoryEntry } from "@/lib/gateway";

// ─── Types ────────────────────────────────────────────────────────────────────

export interface MemoryEntry {
  id: string;
  key: string;
  content: string;
  category: string;
  score: number;
  created_at: string;
  updated_at: string;
}

function toMemoryEntry(e: GatewayMemoryEntry): MemoryEntry {
  return {
    id: e.id,
    key: e.key,
    content: e.content,
    // MemoryCategory is serialised as a lowercase string by the Rust backend.
    category: typeof e.category === "string" ? e.category : String(e.category),
    score: e.score,
    created_at: e.created_at,
    updated_at: e.updated_at,
  };
}

// ─── Store ────────────────────────────────────────────────────────────────────

interface MemoryState {
  // ── Search
  query: string;
  results: MemoryEntry[];
  searching: boolean;
  searchError: string | null;
  selectedEntry: MemoryEntry | null;

  // ── Daily timeline
  availableDates: string[];          // "YYYY-MM-DD" sorted descending
  datesLoading: boolean;
  selectedDate: string | null;
  dailyContent: string | null;
  dailyLoading: boolean;

  // ── Actions
  setQuery: (query: string) => void;
  search: (query: string, limit?: number) => Promise<void>;
  selectEntry: (entry: MemoryEntry | null) => void;
  clearSearch: () => void;

  loadDates: () => Promise<void>;
  selectDate: (date: string) => Promise<void>;
}

export const useMemoryStore = create<MemoryState>((set, _get) => ({
  query: "",
  results: [],
  searching: false,
  searchError: null,
  selectedEntry: null,

  availableDates: [],
  datesLoading: false,
  selectedDate: null,
  dailyContent: null,
  dailyLoading: false,

  setQuery: (query) => set({ query }),

  /**
   * Search memory via the gateway REST API (`GET /api/v1/memory/search?q=...`).
   *
   * Previously used Tauri IPC `search_memory_command`.  Both read from the
   * same `InMemoryStore` instance; the gateway path also works for the CLI.
   */
  search: async (query, limit = 20) => {
    if (!query.trim()) {
      set({ results: [], searchError: null, selectedEntry: null });
      return;
    }
    set({ searching: true, searchError: null });
    try {
      const { entries } = await gateway.searchMemory(query, limit);
      set({
        results: entries.map(toMemoryEntry),
        searching: false,
        selectedEntry: null,
      });
    } catch (err) {
      set({
        results: [],
        searching: false,
        searchError: err instanceof Error ? err.message : String(err),
      });
    }
  },

  selectEntry: (entry) => set({ selectedEntry: entry }),

  clearSearch: () =>
    set({ query: "", results: [], searchError: null, selectedEntry: null }),

  /**
   * Load the list of available daily-memory dates.
   *
   * Derives the date list from all memory entries that have category "daily"
   * via the gateway `GET /api/v1/memory` endpoint.  This replaces the
   * non-existent `list_daily_memory_dates_command` Tauri IPC call (which
   * always failed silently).
   */
  loadDates: async () => {
    set({ datesLoading: true });
    try {
      const { entries } = await gateway.listMemory();
      const dailyDates = entries
        .filter((e) => e.category === "daily")
        .map((e) => {
          // Daily entries use keys like "daily:YYYY-MM-DD".
          const match = /(\d{4}-\d{2}-\d{2})/.exec(e.key);
          return match ? match[1] : null;
        })
        .filter((d): d is string => d !== null);

      // Deduplicate and sort descending (most recent first).
      const unique = [...new Set(dailyDates)];
      unique.sort((a, b) => b.localeCompare(a));
      set({ availableDates: unique, datesLoading: false });
    } catch {
      // Gateway not reachable — show empty list without surfacing an error.
      set({ availableDates: [], datesLoading: false });
    }
  },

  /**
   * Load the raw daily-memory content for `date`.
   *
   * Uses Tauri IPC `get_daily_memory_command` because the gateway does not
   * yet expose a per-date daily-memory endpoint.
   *
   * ## TODO: Migrate to gateway once `GET /api/v1/memory/daily/{date}` is
   *   available.  Track in todo.md.
   */
  selectDate: async (date) => {
    set({ selectedDate: date, dailyLoading: true, dailyContent: null });
    try {
      const content = await invoke<string | null>("get_daily_memory_command", {
        date,
      });
      set({ dailyContent: content ?? null, dailyLoading: false });
    } catch (err) {
      set({
        dailyContent: null,
        dailyLoading: false,
      });
      void err;
    }
  },
}));
