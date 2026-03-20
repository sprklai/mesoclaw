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

  return {
    get active() {
      return delegation !== null;
    },
    get delegation() {
      return delegation;
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
      // No-op: keep tree visible until stream finishes
    },

    clear() {
      delegation = null;
    },
  };
}

export const delegationStore = createDelegationStore();
