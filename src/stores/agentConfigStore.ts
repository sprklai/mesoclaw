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
import { invoke } from "@tauri-apps/api/core";

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

// ─── Backend Types (snake_case from Rust) ─────────────────────────────────────

interface BackendAgent {
  id: string;
  name: string;
  description: string | null;
  system_prompt: string;
  model_id: string;
  provider_id: string;
  temperature: number;
  max_tokens: number | null;
  tools_enabled: number;
  memory_enabled: number;
  workspace_path: string | null;
  is_active: number;
  created_at: string;
  updated_at: string;
}

interface BackendCreateAgentRequest {
  name: string;
  description?: string | null;
  system_prompt: string;
  model_id: string;
  provider_id: string;
  temperature?: number;
  max_tokens?: number | null;
  tools_enabled?: boolean;
  memory_enabled?: boolean;
}

interface BackendUpdateAgentRequest {
  id: string;
  name?: string;
  description?: string | null;
  system_prompt?: string;
  model_id?: string;
  provider_id?: string;
  temperature?: number;
  max_tokens?: number | null;
  tools_enabled?: boolean;
  memory_enabled?: boolean;
  workspace_path?: string | null;
  is_active?: boolean;
}

interface BackendSession {
  id: string;
  agent_id: string;
  name: string;
  status: string;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

interface BackendRun {
  id: string;
  session_id: string;
  agent_id: string;
  parent_run_id: string | null;
  status: string;
  input_message: string;
  output_message: string | null;
  error_message: string | null;
  tokens_used: number | null;
  duration_ms: number | null;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
}

// ─── Type Conversion Utilities ────────────────────────────────────────────────

function backendAgentToFrontend(agent: BackendAgent): AgentConfig {
  return {
    id: agent.id,
    name: agent.name,
    role: agent.description ?? "",
    systemPrompt: agent.system_prompt,
    providerId: agent.provider_id,
    modelId: agent.model_id,
    temperature: agent.temperature,
    maxTokens: agent.max_tokens ?? 4096,
    maxIterations: 20, // Default, not stored in backend
    maxHistory: 50, // Default, not stored in backend
    isEnabled: agent.is_active === 1,
    createdAt: new Date(agent.created_at).getTime(),
    updatedAt: new Date(agent.updated_at).getTime(),
  };
}

function frontendCreateToBackend(request: CreateAgentRequest): BackendCreateAgentRequest {
  return {
    name: request.name,
    description: request.role,
    system_prompt: request.systemPrompt,
    model_id: request.modelId,
    provider_id: request.providerId,
    temperature: request.temperature,
    max_tokens: request.maxTokens,
    tools_enabled: true,
    memory_enabled: true,
  };
}

function frontendUpdateToBackend(request: UpdateAgentRequest): BackendUpdateAgentRequest {
  return {
    id: request.id,
    name: request.name,
    description: request.role,
    system_prompt: request.systemPrompt,
    model_id: request.modelId,
    provider_id: request.providerId,
    temperature: request.temperature,
    max_tokens: request.maxTokens,
    is_active: request.isEnabled,
  };
}

function backendSessionToFrontend(session: BackendSession): AgentSessionSummary {
  return {
    id: session.id,
    agentId: session.agent_id,
    agentName: session.name,
    status: session.status as AgentSessionSummary["status"],
    startedAt: new Date(session.created_at).getTime(),
    completedAt: session.completed_at ? new Date(session.completed_at).getTime() : undefined,
    messageCount: 0, // Not tracked in backend model
  };
}

function backendRunToFrontend(run: BackendRun): AgentRun {
  return {
    id: run.id,
    sessionId: run.session_id,
    agentId: run.agent_id,
    agentName: "", // Not in backend model
    status: run.status as AgentRun["status"],
    startedAt: run.started_at ? new Date(run.started_at).getTime() : Date.now(),
    completedAt: run.completed_at ? new Date(run.completed_at).getTime() : undefined,
    iterations: 0, // Not tracked in backend model
    toolCalls: [], // Not tracked in backend model
  };
}

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
      const backendAgents = await invoke<BackendAgent[]>("list_db_agents_command");
      const agents = backendAgents.map(backendAgentToFrontend);
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
      const backendRequest = frontendCreateToBackend(request);
      const backendAgent = await invoke<BackendAgent>("create_db_agent_command", {
        request: backendRequest,
      });
      const agent = backendAgentToFrontend(backendAgent);

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
      const backendRequest = frontendUpdateToBackend(request);
      const backendAgent = await invoke<BackendAgent>("update_db_agent_command", {
        request: backendRequest,
      });
      const updatedAgent = backendAgentToFrontend(backendAgent);

      set((state) => ({
        agents: state.agents.map((a) => (a.id === request.id ? updatedAgent : a)),
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
      await invoke("delete_db_agent_command", { id });
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

    // Create a new agent based on the existing one
    const newAgent = await get().createAgent({
      name: `${agent.name} (Copy)`,
      role: agent.role,
      systemPrompt: agent.systemPrompt,
      providerId: agent.providerId,
      modelId: agent.modelId,
      temperature: agent.temperature,
      maxTokens: agent.maxTokens,
      maxIterations: agent.maxIterations,
      maxHistory: agent.maxHistory,
    });

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
      const backendSessions = await invoke<BackendSession[]>("list_db_agent_sessions_command", {
        agentId: agentId,
      });
      const sessions = backendSessions.map(backendSessionToFrontend);
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
      // Note: The backend command doesn't support filtering by agentId currently
      // If agentId is provided, we filter client-side after fetching
      const backendSessions = await invoke<BackendSession[]>("list_recent_db_sessions_command", {
        limit: 50,
      });
      let sessions = backendSessions.map(backendSessionToFrontend);

      // Filter by agentId if provided
      if (_agentId) {
        sessions = sessions.filter((s) => s.agentId === _agentId);
      }

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
      const backendRuns = await invoke<BackendRun[]>("list_active_db_runs_command");
      const runs = backendRuns.map(backendRunToFrontend);
      set({ activeRuns: runs });
    } catch (error) {
      const message = extractErrorMessage(error);
      set({ runsError: message });
    }
  },

  getRunDetails: async (runId: string) => {
    const backendRun = await invoke<BackendRun>("get_db_run_details_command", { runId: runId });
    return backendRunToFrontend(backendRun);
  },

  cancelRun: async (runId: string) => {
    set({ runsError: null });
    try {
      await invoke("cancel_db_run_command", { runId: runId });
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
