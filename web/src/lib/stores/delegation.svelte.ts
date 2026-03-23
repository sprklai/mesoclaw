export interface AgentState {
  id: string;
  description: string;
  toolUses: number;
  tokensUsed: number;
  currentActivity: string;
  status: "pending" | "running" | "completed" | "failed";
  durationMs?: number;
}

export interface DelegationState {
  delegationId: string;
  agents: AgentState[];
  startedAt: number;
}

function createDelegationStore() {
  let delegation = $state<DelegationState | null>(null);
  let aggregating = $state(false);

  return {
    get active() {
      return delegation !== null;
    },
    get delegation() {
      return delegation;
    },
    get aggregating() {
      return aggregating;
    },

    startDelegation(
      delegationId: string,
      agents: Array<{ id: string; description: string }>,
    ) {
      delegation = {
        delegationId,
        agents: agents.map((a) => ({
          id: a.id,
          description: a.description,
          toolUses: 0,
          tokensUsed: 0,
          currentActivity: "",
          status: "pending" as const,
        })),
        startedAt: Date.now(),
      };
    },

    updateAgent(
      agentId: string,
      toolUses: number,
      tokensUsed: number,
      activity: string,
    ) {
      if (!delegation) return;
      delegation = {
        ...delegation,
        agents: delegation.agents.map((a) =>
          a.id === agentId
            ? {
                ...a,
                toolUses,
                tokensUsed,
                currentActivity: activity,
                status: "running" as const,
              }
            : a,
        ),
      };
    },

    completeAgent(
      agentId: string,
      status: string,
      durationMs: number,
      toolUses: number,
      tokensUsed: number,
    ) {
      if (!delegation) return;
      const agentStatus: "completed" | "failed" =
        status === "completed" ? "completed" : "failed";
      delegation = {
        ...delegation,
        agents: delegation.agents.map((a) =>
          a.id === agentId
            ? {
                ...a,
                status: agentStatus,
                durationMs,
                toolUses,
                tokensUsed,
              }
            : a,
        ),
      };
    },

    completeDelegation() {
      aggregating = true;
    },

    clear() {
      delegation = null;
      aggregating = false;
    },
  };
}

export const delegationStore = createDelegationStore();

/** Build a DelegationRecord from live delegation state for use as a fallback. */
export function buildDelegationRecord(
  state: DelegationState,
): import("$lib/stores/messages.svelte").DelegationRecord {
  return {
    delegation_id: state.delegationId,
    total_duration_ms: Date.now() - state.startedAt,
    total_tokens: state.agents.reduce((sum, a) => sum + a.tokensUsed, 0),
    agents: state.agents.map((a) => ({
      id: a.id,
      description: a.description,
      status:
        a.status === "completed"
          ? "Completed"
          : a.status === "failed"
            ? "Failed"
            : "Running",
      tool_uses: a.toolUses,
      tokens_used: a.tokensUsed,
      duration_ms: a.durationMs ?? 0,
    })),
  };
}
