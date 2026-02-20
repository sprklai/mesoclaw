/**
 * Zustand store for Activity Dashboard.
 *
 * Manages real-time, planned, and recent activities with auto-refresh.
 */

import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

import {
  type Activity,
  type PlannedJob,
  DEFAULT_ACTIVITY_CONFIG,
} from "@/types/activity";

// ─── Types from backend (mirrors Rust types) ───────────────────────────────

interface BackendActivity {
  id: string;
  source: "agent" | "scheduler" | "system" | "channel";
  title: string;
  status:
    | "running"
    | "awaiting"
    | "pending"
    | "paused"
    | "success"
    | "error"
    | "cancelled"
    | "terminated"
    | "stuck"
    | "skipped";
  startedAt: string;
  completedAt?: string;
  linkTo?: string;
}

// ─── Store State ────────────────────────────────────────────────────────────

interface ActivityState {
  // Real-time activities (running/awaiting/paused)
  realtime: Activity[];
  // Planned scheduled jobs
  planned: PlannedJob[];
  // Recently completed activities
  recent: Activity[];

  // Loading states
  loading: boolean;
  error: string | null;

  // Config
  refreshIntervalMs: number;
  rollingWindowMs: number;

  // Auto-refresh handle
  refreshHandle: ReturnType<typeof setInterval> | null;

  // Actions
  loadAll: () => Promise<void>;
  startAutoRefresh: () => void;
  stopAutoRefresh: () => void;
  setRefreshInterval: (ms: number) => void;
  setRollingWindow: (ms: number) => void;
}

// ─── Constants ──────────────────────────────────────────────────────────────

const ACTIVE_STATUSES = new Set(["running", "awaiting", "paused", "pending"]);
const TERMINAL_STATUSES = new Set([
  "success",
  "error",
  "cancelled",
  "terminated",
  "stuck",
  "skipped",
]);

// ─── Helper Functions ───────────────────────────────────────────────────────

function mapBackendActivity(a: BackendActivity): Activity {
  return {
    id: a.id,
    source: a.source,
    title: a.title,
    status: a.status,
    startedAt: a.startedAt,
    completedAt: a.completedAt,
    linkTo: a.linkTo,
  };
}

function categorizeActivities(
  activities: Activity[],
  rollingWindowMs: number
): { realtime: Activity[]; recent: Activity[] } {
  const now = Date.now();
  const cutoff = now - rollingWindowMs;

  const realtime: Activity[] = [];
  const recent: Activity[] = [];

  for (const activity of activities) {
    // Filter by rolling window
    const startTime = new Date(activity.startedAt).getTime();
    if (startTime < cutoff) continue;

    if (ACTIVE_STATUSES.has(activity.status)) {
      realtime.push(activity);
    } else if (TERMINAL_STATUSES.has(activity.status)) {
      recent.push(activity);
    }
  }

  // Sort realtime by start time (newest first)
  realtime.sort(
    (a, b) =>
      new Date(b.startedAt).getTime() - new Date(a.startedAt).getTime()
  );

  // Sort recent by completed time (newest first), then by start time
  recent.sort((a, b) => {
    const aTime = a.completedAt
      ? new Date(a.completedAt).getTime()
      : new Date(a.startedAt).getTime();
    const bTime = b.completedAt
      ? new Date(b.completedAt).getTime()
      : new Date(b.startedAt).getTime();
    return bTime - aTime;
  });

  return { realtime, recent };
}

// ─── Store ──────────────────────────────────────────────────────────────────

export const useActivityStore = create<ActivityState>((set, get) => ({
  realtime: [],
  planned: [],
  recent: [],
  loading: false,
  error: null,
  refreshIntervalMs: DEFAULT_ACTIVITY_CONFIG.refreshIntervalMs,
  rollingWindowMs: DEFAULT_ACTIVITY_CONFIG.rollingWindowMs,
  refreshHandle: null,

  loadAll: async () => {
    set({ loading: true, error: null });
    try {
      // Fetch activities from backend
      const backendActivities = await invoke<BackendActivity[]>(
        "get_recent_activity_command",
        {
          withinMs: get().rollingWindowMs,
        }
      );

      // Map and categorize
      const activities = backendActivities.map(mapBackendActivity);
      const { realtime, recent } = categorizeActivities(
        activities,
        get().rollingWindowMs
      );

      // For now, planned jobs come from scheduler store (already loaded elsewhere)
      // We'll just use the activities for now
      set({ realtime, recent, planned: [], loading: false });
    } catch (err) {
      set({ error: String(err), loading: false });
    }
  },

  startAutoRefresh: () => {
    const { refreshHandle, refreshIntervalMs, loadAll } = get();

    // Clear existing interval if any
    if (refreshHandle) {
      clearInterval(refreshHandle);
    }

    // Load immediately
    loadAll();

    // Set up periodic refresh
    const handle = setInterval(() => {
      loadAll();
    }, refreshIntervalMs);

    set({ refreshHandle: handle });
  },

  stopAutoRefresh: () => {
    const { refreshHandle } = get();
    if (refreshHandle) {
      clearInterval(refreshHandle);
      set({ refreshHandle: null });
    }
  },

  setRefreshInterval: (ms: number) => {
    const { refreshHandle, startAutoRefresh } = get();
    set({ refreshIntervalMs: ms });
    // Restart auto-refresh with new interval if active
    if (refreshHandle) {
      startAutoRefresh();
    }
  },

  setRollingWindow: (ms: number) => {
    set({ rollingWindowMs: ms });
    // Reload with new window
    get().loadAll();
  },
}));
