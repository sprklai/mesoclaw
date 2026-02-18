/**
 * Zustand store for the agent loop UI state.
 *
 * Tracks the current session, per-tool execution status, the approval queue,
 * and iteration count.  Subscribes to Tauri `app-event` events emitted by the
 * Rust backend's TauriBridge.
 */

import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

// ─── Types ────────────────────────────────────────────────────────────────────

export type ToolStatus = "running" | "success" | "error";

export interface ToolExecution {
  /** Unique key: `${tool_name}-${timestamp}` */
  id: string;
  toolName: string;
  /** JSON args from the backend (may be any object or primitive). */
  args: Record<string, unknown>;
  status: ToolStatus;
  result: string | null;
  startedAt: number;
  finishedAt: number | null;
}

export interface ApprovalRequest {
  actionId: string;
  toolName: string;
  description: string;
  riskLevel: string;
}

export type AgentSessionStatus =
  | "idle"
  | "running"
  | "awaiting_approval"
  | "complete"
  | "error";

export interface AgentSession {
  sessionId: string;
  status: AgentSessionStatus;
  iterationCount: number;
  startedAt: number;
  completedAt: number | null;
  finalMessage: string | null;
}

// ─── AppEvent payloads (mirrors Rust AppEvent enum) ───────────────────────────

interface AgentToolStartPayload {
  type: "agent_tool_start";
  tool_name: string;
  args: Record<string, unknown>;
}

interface AgentToolResultPayload {
  type: "agent_tool_result";
  tool_name: string;
  result: string;
  success: boolean;
}

interface AgentCompletePayload {
  type: "agent_complete";
  session_id: string;
  message: string;
}

interface ApprovalNeededPayload {
  type: "approval_needed";
  action_id: string;
  tool_name: string;
  description: string;
  risk_level: string;
}

type AppEventPayload =
  | AgentToolStartPayload
  | AgentToolResultPayload
  | AgentCompletePayload
  | ApprovalNeededPayload
  | { type: string };

// ─── Store ────────────────────────────────────────────────────────────────────

const MAX_EXECUTIONS = 50;

interface AgentState {
  session: AgentSession | null;
  executions: ToolExecution[];
  approvalQueue: ApprovalRequest[];

  /** Unlisten function returned by `listen()`. */
  _unlisten: (() => void) | null;

  // ── Actions ──
  startListening: () => Promise<void>;
  stopListening: () => void;

  /** Begin a new session (resets execution log). */
  startSession: (sessionId: string) => void;

  /** Cancel the current session via Tauri command. */
  cancelSession: () => Promise<void>;

  /** Respond to an approval request. */
  respondToApproval: (
    actionId: string,
    approved: boolean,
    allowAlways?: boolean
  ) => Promise<void>;

  clearSession: () => void;
}

export const useAgentStore = create<AgentState>((set, get) => ({
  session: null,
  executions: [],
  approvalQueue: [],
  _unlisten: null,

  startListening: async () => {
    const existing = get()._unlisten;
    if (existing) return; // already subscribed

    const unlisten = await listen<AppEventPayload>("app-event", (event) => {
      const payload = event.payload;

      switch (payload.type) {
        case "agent_tool_start": {
          const p = payload as AgentToolStartPayload;
          const execution: ToolExecution = {
            id: `${p.tool_name}-${Date.now()}`,
            toolName: p.tool_name,
            args: p.args ?? {},
            status: "running",
            result: null,
            startedAt: Date.now(),
            finishedAt: null,
          };
          set((s) => ({
            executions: [execution, ...s.executions].slice(0, MAX_EXECUTIONS),
            session: s.session
              ? {
                  ...s.session,
                  iterationCount: s.session.iterationCount + 1,
                  status: "running",
                }
              : null,
          }));
          break;
        }

        case "agent_tool_result": {
          const p = payload as AgentToolResultPayload;
          set((s) => ({
            executions: s.executions.map((ex) =>
              ex.toolName === p.tool_name && ex.status === "running"
                ? {
                    ...ex,
                    status: p.success ? "success" : "error",
                    result: p.result,
                    finishedAt: Date.now(),
                  }
                : ex
            ),
          }));
          break;
        }

        case "agent_complete": {
          const p = payload as AgentCompletePayload;
          set((s) => ({
            session: s.session
              ? {
                  ...s.session,
                  status: "complete",
                  completedAt: Date.now(),
                  finalMessage: p.message,
                }
              : {
                  sessionId: p.session_id,
                  status: "complete",
                  iterationCount: 0,
                  startedAt: Date.now(),
                  completedAt: Date.now(),
                  finalMessage: p.message,
                },
          }));
          break;
        }

        case "approval_needed": {
          const p = payload as ApprovalNeededPayload;
          const req: ApprovalRequest = {
            actionId: p.action_id,
            toolName: p.tool_name,
            description: p.description,
            riskLevel: p.risk_level,
          };
          set((s) => ({
            approvalQueue: [...s.approvalQueue, req],
            session: s.session
              ? { ...s.session, status: "awaiting_approval" }
              : null,
          }));
          break;
        }

        default:
          break;
      }
    });

    set({ _unlisten: unlisten });
  },

  stopListening: () => {
    get()._unlisten?.();
    set({ _unlisten: null });
  },

  startSession: (sessionId: string) => {
    set({
      session: {
        sessionId,
        status: "running",
        iterationCount: 0,
        startedAt: Date.now(),
        completedAt: null,
        finalMessage: null,
      },
      executions: [],
      approvalQueue: [],
    });
  },

  cancelSession: async () => {
    const { session } = get();
    if (!session) return;
    try {
      await invoke("cancel_agent_command", { sessionId: session.sessionId });
    } catch {
      // Best-effort; the backend may already have stopped.
    }
    set((s) => ({
      session: s.session
        ? { ...s.session, status: "complete", completedAt: Date.now() }
        : null,
    }));
  },

  respondToApproval: async (
    actionId: string,
    approved: boolean,
    _allowAlways = false
  ) => {
    try {
      await invoke("respond_to_approval_command", { actionId, approved });
    } catch {
      // Best-effort.
    }
    set((s) => ({
      approvalQueue: s.approvalQueue.filter((r) => r.actionId !== actionId),
      session:
        s.session && s.approvalQueue.length <= 1
          ? { ...s.session, status: "running" }
          : s.session,
    }));
  },

  clearSession: () => {
    set({ session: null, executions: [], approvalQueue: [] });
  },
}));
