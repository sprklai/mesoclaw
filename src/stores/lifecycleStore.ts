/**
 * Lifecycle Management Store
 *
 * Manages state for the resource lifecycle management system.
 * Tracks resources, health status, and user intervention requests.
 */

import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { UnlistenFn } from "@tauri-apps/api/event";

// ─── Types ───────────────────────────────────────────────────────────────────

export interface ResourceStatus {
  id: string;
  resourceType: string;
  state: "idle" | "running" | "stuck" | "recovering" | "completed" | "failed";
  createdAt: string;
  recoveryAttempts: number;
  escalationTier: number;
  progress?: number;
  substate?: string;
}

export interface UserInterventionRequest {
  id: string;
  resourceId: string;
  resourceType: string;
  failureContext: {
    error: string;
    recoveryAttempts: number;
    runningDurationSecs: number;
    lastState: string;
    failedAt: string;
  };
  attemptedTiers: number[];
  options: InterventionOption[];
  createdAt: string;
}

export interface InterventionOption {
  id: string;
  label: string;
  description: string;
  destructive: boolean;
}

export interface SupervisorStats {
  totalResources: number;
  idle: number;
  running: number;
  stuck: number;
  recovering: number;
  completed: number;
  failed: number;
  healthy: number;
  degraded: number;
  isMonitoring: boolean;
}

export interface StateTransition {
  resourceId: string;
  fromState: string;
  toState: string;
  timestamp: string;
  reason: string;
}

interface LifecycleState {
  // Resource tracking
  resources: ResourceStatus[];
  selectedResourceId: string | null;
  resourceHistory: StateTransition[];

  // Health stats
  stats: SupervisorStats | null;

  // User interventions
  pendingInterventions: UserInterventionRequest[];
  activeIntervention: UserInterventionRequest | null;

  // UI state
  isLoading: boolean;
  error: string | null;
  isMonitoring: boolean;

  // Event listeners
  _listeners: UnlistenFn[];
}

interface LifecycleActions {
  // Resource operations
  fetchAllResources: () => Promise<void>;
  fetchResourcesByType: (type: string) => Promise<void>;
  fetchStuckResources: () => Promise<void>;
  fetchResourceStatus: (resourceId: string) => Promise<ResourceStatus | null>;
  fetchResourceHistory: (resourceId: string) => Promise<void>;
  selectResource: (resourceId: string | null) => void;

  // Resource control
  retryResource: (resourceId: string) => Promise<void>;
  stopResource: (resourceId: string) => Promise<void>;
  killResource: (resourceId: string) => Promise<void>;
  recordHeartbeat: (resourceId: string) => Promise<void>;
  updateProgress: (resourceId: string, progress: number, substate: string) => Promise<void>;
  spawnResource: (type: string, config: Record<string, unknown>) => Promise<string>;

  // Stats
  fetchStats: () => Promise<void>;
  checkMonitoring: () => Promise<void>;

  // Interventions
  fetchPendingInterventions: () => Promise<void>;
  resolveIntervention: (
    requestId: string,
    selectedOption: string,
    additionalData?: Record<string, unknown>
  ) => Promise<void>;
  setActiveIntervention: (request: UserInterventionRequest | null) => void;

  // Event handling
  setupEventListeners: () => Promise<void>;
  cleanupEventListeners: () => void;

  // State management
  setError: (error: string | null) => void;
  clearError: () => void;
  reset: () => void;
}

const initialState: LifecycleState = {
  resources: [],
  selectedResourceId: null,
  resourceHistory: [],
  stats: null,
  pendingInterventions: [],
  activeIntervention: null,
  isLoading: false,
  error: null,
  isMonitoring: false,
  _listeners: [],
};

export const useLifecycleStore = create<LifecycleState & LifecycleActions>((set, get) => ({
  ...initialState,

  // ─── Resource Operations ───────────────────────────────────────────────────

  fetchAllResources: async () => {
    set({ isLoading: true, error: null });
    try {
      const resources = await invoke<ResourceStatus[]>("get_all_resources_command");
      set({ resources, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  fetchResourcesByType: async (type: string) => {
    set({ isLoading: true, error: null });
    try {
      const resources = await invoke<ResourceStatus[]>("get_resources_by_type_command", {
        resourceType: type,
      });
      set({ resources, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  fetchStuckResources: async () => {
    set({ isLoading: true, error: null });
    try {
      const resources = await invoke<ResourceStatus[]>("get_stuck_resources_command");
      set({ resources, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  fetchResourceStatus: async (resourceId: string) => {
    try {
      const status = await invoke<ResourceStatus>("get_resource_status_command", {
        resourceId,
      });
      return status;
    } catch (error) {
      set({ error: String(error) });
      return null;
    }
  },

  fetchResourceHistory: async (resourceId: string) => {
    try {
      const history = await invoke<StateTransition[]>("get_resource_history_command", {
        resourceId,
      });
      set({ resourceHistory: history });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  selectResource: (resourceId: string | null) => {
    set({ selectedResourceId: resourceId });
    if (resourceId) {
      get().fetchResourceHistory(resourceId);
    } else {
      set({ resourceHistory: [] });
    }
  },

  // ─── Resource Control ──────────────────────────────────────────────────────

  retryResource: async (resourceId: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke("retry_resource_command", { resourceId });
      await get().fetchAllResources();
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  stopResource: async (resourceId: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke("stop_resource_command", { resourceId });
      await get().fetchAllResources();
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  killResource: async (resourceId: string) => {
    set({ isLoading: true, error: null });
    try {
      await invoke("kill_resource_command", { resourceId });
      await get().fetchAllResources();
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  recordHeartbeat: async (resourceId: string) => {
    try {
      await invoke("record_resource_heartbeat_command", { resourceId });
    } catch (error) {
      console.error("Failed to record heartbeat:", error);
    }
  },

  updateProgress: async (resourceId: string, progress: number, substate: string) => {
    try {
      await invoke("update_resource_progress_command", { resourceId, progress, substate });
    } catch (error) {
      console.error("Failed to update progress:", error);
    }
  },

  spawnResource: async (type: string, config: Record<string, unknown>) => {
    set({ isLoading: true, error: null });
    try {
      const resourceId = await invoke<string>("spawn_resource_command", {
        resourceType: type,
        config,
      });
      await get().fetchAllResources();
      set({ isLoading: false });
      return resourceId;
    } catch (error) {
      set({ error: String(error), isLoading: false });
      throw error;
    }
  },

  // ─── Stats ─────────────────────────────────────────────────────────────────

  fetchStats: async () => {
    try {
      const stats = await invoke<SupervisorStats>("get_supervisor_stats_command");
      set({ stats, isMonitoring: stats.isMonitoring });
    } catch (error) {
      console.error("Failed to fetch stats:", error);
    }
  },

  checkMonitoring: async () => {
    try {
      const isMonitoring = await invoke<boolean>("is_lifecycle_monitoring_command");
      set({ isMonitoring });
    } catch (error) {
      console.error("Failed to check monitoring:", error);
    }
  },

  // ─── Interventions ─────────────────────────────────────────────────────────

  fetchPendingInterventions: async () => {
    try {
      const interventions = await invoke<UserInterventionRequest[]>(
        "get_pending_interventions_command"
      );
      set({ pendingInterventions: interventions });
    } catch (error) {
      console.error("Failed to fetch interventions:", error);
    }
  },

  resolveIntervention: async (
    requestId: string,
    selectedOption: string,
    additionalData?: Record<string, unknown>
  ) => {
    set({ isLoading: true, error: null });
    try {
      await invoke("resolve_intervention_command", {
        requestId,
        selectedOption,
        additionalData,
      });
      await get().fetchPendingInterventions();
      set({ activeIntervention: null, isLoading: false });
    } catch (error) {
      set({ error: String(error), isLoading: false });
    }
  },

  setActiveIntervention: (request: UserInterventionRequest | null) => {
    set({ activeIntervention: request });
  },

  // ─── Event Handling ────────────────────────────────────────────────────────

  setupEventListeners: async () => {
    const listeners = get()._listeners;

    // Clean up existing listeners
    listeners.forEach((unlisten) => unlisten());

    // Note: In a real implementation, we'd subscribe to lifecycle events
    // emitted via the Tauri event system. For now, we'll use polling.
    // The backend would emit events like:
    // - "lifecycle:resource_started"
    // - "lifecycle:resource_stuck"
    // - "lifecycle:resource_recovered"
    // - "lifecycle:user_intervention_needed"

    // Set up periodic refresh
    const refreshInterval = setInterval(() => {
      get().fetchAllResources();
      get().fetchStats();
      get().fetchPendingInterventions();
    }, 5000);

    // Store the interval cleanup function
    set({
      _listeners: [() => clearInterval(refreshInterval)],
    });
  },

  cleanupEventListeners: () => {
    const listeners = get()._listeners;
    listeners.forEach((unlisten) => unlisten());
    set({ _listeners: [] });
  },

  // ─── State Management ──────────────────────────────────────────────────────

  setError: (error: string | null) => set({ error }),
  clearError: () => set({ error: null }),
  reset: () => {
    get().cleanupEventListeners();
    set(initialState);
  },
}));
