/**
 * Agent configuration store for managing multi-agent system.
 *
 * Provides state management for:
 * - Agent configurations (CRUD operations)
 * - Workspace files (SOUL.md, AGENTS.md, etc.)
 * - Session history
 * - Execution monitoring
 *
 * Uses Tauri IPC commands for persistence.
 */
import { create } from "zustand";

import { extractErrorMessage } from "@/lib/error-utils";
import type {
  AgentConfig,
  AgentSessionSummary,
  AgentRun,
  CreateAgentRequest,
  UpdateAgentRequest,
  WorkspaceFile,
  WorkspaceFileType,
} from "@/lib/agent-config";
import { DEFAULT_AGENT_CONFIG } from "@/lib/agent-config";

// ─── Store State Types ─────────────────────────────────────────────────────

interface AgentConfigState {
  // Agent configurations
  agents: AgentConfig[];
  selectedAgentId: string | null;
  isLoadingAgents: boolean;
  agentsError: string | null;

  // Workspace files
  workspaceFiles: Map<string, WorkspaceFile>;
  isLoadingWorkspace: boolean;
  workspaceError: string | null;

  // Session history
  sessions: AgentSessionSummary[];
  isLoadingSessions: boolean;
  sessionsError: string | null;

  // Active runs (for execution monitoring)
  activeRuns: AgentRun[];
  runsError: string | null;

  // ── Agent CRUD Actions ──

  /** Load all agent configurations from backend */
  loadAgents: () => Promise<void>;

  /** Get a single agent by ID */
  getAgent: (id: string) => AgentConfig | undefined;

  /** Create a new agent configuration */
  createAgent: (request: CreateAgentRequest) => Promise<AgentConfig>;

  /** Update an existing agent configuration */
  updateAgent: (request: UpdateAgentRequest) => Promise<void>;

  /** Delete an agent configuration */
  deleteAgent: (id: string) => Promise<void>;

  /** Duplicate an agent configuration */
  duplicateAgent: (id: string) => Promise<AgentConfig>;

  /** Set the selected agent for editing */
  selectAgent: (id: string | null) => void;

  /** Toggle agent enabled status */
  toggleAgentEnabled: (id: string) => Promise<void>;

  // ── Workspace File Actions ──

  /** Load workspace files for an agent */
  loadWorkspaceFiles: (agentId: string) => Promise<void>;

  /** Get a specific workspace file */
  getWorkspaceFile: (agentId: string, type: WorkspaceFileType) => WorkspaceFile | undefined;

  /** Update a workspace file */
  updateWorkspaceFile: (agentId: string, type: WorkspaceFileType, content: string) => Promise<void>;

  // ── Session History Actions ──

  /** Load session history for an agent */
  loadSessionHistory: (agentId: string) => Promise<void>;

  /** Load all recent sessions or sessions for a specific agent */
  loadRecentSessions: (agentId?: string) => Promise<void>;

  /** Clear session history for an agent */
  clearSessionHistory: (agentId: string) => Promise<void>;

  // ── Execution Monitoring Actions ──

  /** Load active runs */
  loadActiveRuns: () => Promise<void>;

  /** Get run details */
  getRunDetails: (runId: string) => Promise<AgentRun>;

  /** Cancel a running session */
  cancelRun: (runId: string) => Promise<void>;

  // ── Utility Actions ──

  /** Clear all errors */
  clearErrors: () => void;

  /** Reset store state */
  reset: () => void;
}

// ─── Initial State ─────────────────────────────────────────────────────────

const initialState = {
  agents: [],
  selectedAgentId: null,
  isLoadingAgents: false,
  agentsError: null,

  workspaceFiles: new Map<string, WorkspaceFile>(),
  isLoadingWorkspace: false,
  workspaceError: null,

  sessions: [],
  isLoadingSessions: false,
  sessionsError: null,

  activeRuns: [],
  runsError: null,
};

// ─── Store Implementation ───────────────────────────────────────────────────

export const useAgentConfigStore = create<AgentConfigState>((set, get) => ({
  ...initialState,

  // ── Agent CRUD Actions ──

  loadAgents: async () => {
    set({ isLoadingAgents: true, agentsError: null });
    try {
      // ## TODO: Wire to backend command when available
      // const agents = await invoke<AgentConfig[]>("list_agents_command");
      // For now, use mock data
      const agents: AgentConfig[] = [];
      set({ agents, isLoadingAgents: false });
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ agentsError: message, isLoadingAgents: false });
    }
  },

  getAgent: (id: string) => {
    return get().agents.find((a) => a.id === id);
  },

  createAgent: async (request: CreateAgentRequest) => {
    set({ agentsError: null });
    try {
      const now = Date.now();
      // ## TODO: Wire to backend command when available
      // const agent = await invoke<AgentConfig>("create_agent_command", { request });
      // For now, create locally
      const agent: AgentConfig = {
        id: crypto.randomUUID(),
        name: request.name,
        role: request.role,
        systemPrompt: request.systemPrompt,
        providerId: request.providerId,
        modelId: request.modelId,
        temperature: request.temperature ?? DEFAULT_AGENT_CONFIG.temperature ?? 0.7,
        maxTokens: request.maxTokens ?? DEFAULT_AGENT_CONFIG.maxTokens ?? 4096,
        maxIterations: request.maxIterations ?? DEFAULT_AGENT_CONFIG.maxIterations ?? 20,
        maxHistory: request.maxHistory ?? DEFAULT_AGENT_CONFIG.maxHistory ?? 50,
        isEnabled: true,
        createdAt: now,
        updatedAt: now,
      };

      set((state) => ({
        agents: [...state.agents, agent],
      }));

      return agent;
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ agentsError: message });
      throw new Error(message);
    }
  },

  updateAgent: async (request: UpdateAgentRequest) => {
    set({ agentsError: null });
    try {
      // ## TODO: Wire to backend command when available
      // await invoke("update_agent_command", { request });
      // For now, update locally
      set((state) => ({
        agents: state.agents.map((a) =>
          a.id === request.id
            ? {
                ...a,
                ...request,
                updatedAt: Date.now(),
              }
            : a
        ),
      }));
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ agentsError: message });
      throw new Error(message);
    }
  },

  deleteAgent: async (id: string) => {
    set({ agentsError: null });
    try {
      // ## TODO: Wire to backend command when available
      // await invoke("delete_agent_command", { id });
      // For now, delete locally
      set((state) => ({
        agents: state.agents.filter((a) => a.id !== id),
        selectedAgentId: state.selectedAgentId === id ? null : state.selectedAgentId,
      }));
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ agentsError: message });
      throw new Error(message);
    }
  },

  duplicateAgent: async (id: string) => {
    const agent = get().getAgent(id);
    if (!agent) {
      throw new Error("Agent not found");
    }

    const now = Date.now();
    const newAgent: AgentConfig = {
      ...agent,
      id: crypto.randomUUID(),
      name: `${agent.name} (Copy)`,
      createdAt: now,
      updatedAt: now,
    };

    set((state) => ({
      agents: [...state.agents, newAgent],
    }));

    return newAgent;
  },

  selectAgent: (id: string | null) => {
    set({ selectedAgentId: id });
  },

  toggleAgentEnabled: async (id: string) => {
    const agent = get().getAgent(id);
    if (!agent) {
      throw new Error("Agent not found");
    }

    await get().updateAgent({ id, isEnabled: !agent.isEnabled });
  },

  // ── Workspace File Actions ──

  loadWorkspaceFiles: async (agentId: string) => {
    set({ isLoadingWorkspace: true, workspaceError: null });
    try {
      // ## TODO: Wire to backend command when available
      // const files = await invoke<WorkspaceFile[]>("load_workspace_files_command", { agentId });
      // For now, use empty files
      const files: WorkspaceFile[] = [];
      const fileMap = new Map<string, WorkspaceFile>();
      for (const file of files) {
        fileMap.set(`${agentId}:${file.type}`, file);
      }
      set((state) => ({
        workspaceFiles: new Map([...state.workspaceFiles, ...fileMap]),
        isLoadingWorkspace: false,
      }));
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ workspaceError: message, isLoadingWorkspace: false });
    }
  },

  getWorkspaceFile: (agentId: string, type: WorkspaceFileType) => {
    return get().workspaceFiles.get(`${agentId}:${type}`);
  },

  updateWorkspaceFile: async (agentId: string, type: WorkspaceFileType, content: string) => {
    set({ workspaceError: null });
    try {
      const now = Date.now();
      const file: WorkspaceFile = {
        type,
        filename: `${type.toUpperCase()}.md`,
        path: `workspace/${agentId}/${type}.md`,
        content,
        lastModified: now,
      };

      // ## TODO: Wire to backend command when available
      // await invoke("update_workspace_file_command", { agentId, type, content });

      set((state) => {
        const newFiles = new Map(state.workspaceFiles);
        newFiles.set(`${agentId}:${type}`, file);
        return { workspaceFiles: newFiles };
      });
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ workspaceError: message });
      throw new Error(message);
    }
  },

  // ── Session History Actions ──

  loadSessionHistory: async (agentId: string) => {
    set({ isLoadingSessions: true, sessionsError: null });
    try {
      // ## TODO: Wire to backend command when available
      // const sessions = await invoke<AgentSessionSummary[]>("list_agent_sessions_command", { agentId });
      const sessions: AgentSessionSummary[] = [];
      set((state) => ({
        sessions: [...state.sessions.filter((s) => s.agentId !== agentId), ...sessions],
        isLoadingSessions: false,
      }));
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ sessionsError: message, isLoadingSessions: false });
    }
  },

  loadRecentSessions: async (_agentId?: string) => {
    set({ isLoadingSessions: true, sessionsError: null });
    try {
      // ## TODO: Wire to backend command when available
      // const sessions = await invoke<AgentSessionSummary[]>("list_recent_sessions_command", { agentId: _agentId, limit: 50 });
      const sessions: AgentSessionSummary[] = [];
      set({ sessions, isLoadingSessions: false });
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ sessionsError: message, isLoadingSessions: false });
    }
  },

  clearSessionHistory: async (agentId: string) => {
    set({ sessionsError: null });
    try {
      // ## TODO: Wire to backend command when available
      // await invoke("clear_agent_sessions_command", { agentId });
      set((state) => ({
        sessions: state.sessions.filter((s) => s.agentId !== agentId),
      }));
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ sessionsError: message });
      throw new Error(message);
    }
  },

  // ── Execution Monitoring Actions ──

  loadActiveRuns: async () => {
    set({ runsError: null });
    try {
      // ## TODO: Wire to backend command when available
      // const runs = await invoke<AgentRun[]>("list_active_runs_command");
      const runs: AgentRun[] = [];
      set({ activeRuns: runs });
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ runsError: message });
    }
  },

  getRunDetails: async (runId: string) => {
    // ## TODO: Wire to backend command when available
    // return invoke<AgentRun>("get_run_details_command", { runId });
    const run = get().activeRuns.find((r) => r.id === runId);
    if (!run) {
      throw new Error("Run not found");
    }
    return run;
  },

  cancelRun: async (runId: string) => {
    set({ runsError: null });
    try {
      // ## TODO: Wire to backend command when available
      // await invoke("cancel_agent_run_command", { runId });
      set((state) => ({
        activeRuns: state.activeRuns.filter((r) => r.id !== runId),
      }));
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ runsError: message });
      throw new Error(message);
    }
  },

  // ── Utility Actions ──

  clearErrors: () => {
    set({
      agentsError: null,
      workspaceError: null,
      sessionsError: null,
      runsError: null,
    });
  },

  reset: () => {
    set(initialState);
  },
}));
