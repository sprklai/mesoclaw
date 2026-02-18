/**
 * Zustand store for the Memory Search UI.
 *
 * State: search query, results, loading flag, selected entry,
 * daily date list, selected date, and that day's raw content.
 *
 * Note: memory commands are stubbed in Phase 3 follow-up (see todo.md).
 * This store handles errors gracefully and shows empty states.
 */

import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

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

  search: async (query, limit = 20) => {
    if (!query.trim()) {
      set({ results: [], searchError: null, selectedEntry: null });
      return;
    }
    set({ searching: true, searchError: null });
    try {
      const raw = await invoke<MemoryEntry[]>("search_memory_command", {
        query,
        limit,
      });
      set({ results: raw, searching: false, selectedEntry: null });
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

  loadDates: async () => {
    set({ datesLoading: true });
    try {
      const dates = await invoke<string[]>("list_daily_memory_dates_command");
      // Sort descending (most recent first)
      dates.sort((a, b) => b.localeCompare(a));
      set({ availableDates: dates, datesLoading: false });
    } catch {
      // Command may not be wired yet — silently show empty list
      set({ availableDates: [], datesLoading: false });
    }
  },

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
