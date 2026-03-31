import * as m from "$lib/paraglide/messages";
import {
  apiGet,
  apiGetText,
  apiPost,
  apiPut,
  apiDelete,
} from "$lib/api/client";

export interface WorkflowStep {
  name: string;
  type: string;
  depends_on: string[];
  tool?: string;
  args?: Record<string, unknown>;
  prompt?: string;
  model?: string;
  seconds?: number;
  expression?: string;
  if_true?: string;
  if_false?: string;
  steps?: string[];
  timeout_secs?: number;
  retry?: { max_retries: number; retry_delay_ms: number };
  failure_policy?: string | { Fallback: { step: string } };
}

export interface NodePosition {
  x: number;
  y: number;
}

export interface Workflow {
  id: string;
  name: string;
  description: string;
  schedule: string | null;
  steps: WorkflowStep[];
  layout?: Record<string, NodePosition>;
  created_at: string;
  updated_at: string;
}

export interface StepOutput {
  step_name: string;
  output: string;
  success: boolean;
  duration_ms: number;
  error: string | null;
}

export interface WorkflowRun {
  id: string;
  workflow_id: string;
  status: string;
  step_results: StepOutput[];
  started_at: string;
  completed_at: string | null;
  error: string | null;
}

export interface WorkflowRunProgress {
  runId: string;
  completedSteps: { stepName: string; success: boolean }[];
  startedAt: number;
}

const WORKFLOW_SAFETY_TIMEOUT_MS = 10 * 60 * 1000;

function createWorkflowsStore() {
  let workflows = $state<Workflow[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let runningWorkflows = $state<Map<string, WorkflowRunProgress>>(new Map());
  const timeouts = new Map<string, ReturnType<typeof setTimeout>>();

  return {
    get workflows() {
      return workflows;
    },
    get loading() {
      return loading;
    },
    get error() {
      return error;
    },

    isRunning(workflowId: string): boolean {
      return runningWorkflows.has(workflowId);
    },

    getProgress(workflowId: string): WorkflowRunProgress | undefined {
      return runningWorkflows.get(workflowId);
    },

    setRunning(workflowId: string, runId: string) {
      const next = new Map(runningWorkflows);
      next.set(workflowId, {
        runId,
        completedSteps: [],
        startedAt: Date.now(),
      });
      runningWorkflows = next;

      // Safety timeout: clear running state after 5 minutes
      const existing = timeouts.get(workflowId);
      if (existing) clearTimeout(existing);
      timeouts.set(
        workflowId,
        setTimeout(() => {
          this.setCompleted(workflowId, runId, "timeout");
        }, WORKFLOW_SAFETY_TIMEOUT_MS),
      );
    },

    stepCompleted(
      workflowId: string,
      _runId: string,
      stepName: string,
      success: boolean,
    ) {
      const progress = runningWorkflows.get(workflowId);
      if (!progress) return;
      const next = new Map(runningWorkflows);
      next.set(workflowId, {
        ...progress,
        completedSteps: [...progress.completedSteps, { stepName, success }],
      });
      runningWorkflows = next;
    },

    setCompleted(workflowId: string, _runId: string, _status: string) {
      const next = new Map(runningWorkflows);
      next.delete(workflowId);
      runningWorkflows = next;

      const timeout = timeouts.get(workflowId);
      if (timeout) {
        clearTimeout(timeout);
        timeouts.delete(workflowId);
      }
    },

    async cancel(workflowId: string) {
      // Save running state before optimistic removal
      const savedProgress = runningWorkflows.get(workflowId);
      const savedTimeout = timeouts.get(workflowId);

      if (!savedProgress) {
        error = m.workflows_cancel_no_active_run();
        return;
      }

      // Optimistic remove
      const next = new Map(runningWorkflows);
      next.delete(workflowId);
      runningWorkflows = next;

      if (savedTimeout) {
        clearTimeout(savedTimeout);
        timeouts.delete(workflowId);
      }

      try {
        await apiPost(
          `/workflows/${encodeURIComponent(workflowId)}/runs/${encodeURIComponent(savedProgress.runId)}/cancel`,
          {},
        );
      } catch (e: unknown) {
        // Restore running state on failure
        const restored = new Map(runningWorkflows);
        restored.set(workflowId, savedProgress);
        runningWorkflows = restored;

        if (savedTimeout) {
          timeouts.set(
            workflowId,
            setTimeout(() => {
              this.setCompleted(workflowId, savedProgress.runId, "timeout");
            }, WORKFLOW_SAFETY_TIMEOUT_MS),
          );
        }
        error = e instanceof Error ? e.message : "Failed to cancel workflow";
      }
    },

    async load() {
      loading = true;
      error = null;
      try {
        workflows = await apiGet<Workflow[]>("/workflows").catch(
          (e: unknown) => {
            error = e instanceof Error ? e.message : "Failed to load workflows";
            return [] as Workflow[];
          },
        );
      } finally {
        loading = false;
      }
    },

    async create(tomlContent: string): Promise<Workflow> {
      const result = await apiPost<Workflow>("/workflows", {
        toml_content: tomlContent,
      });
      await this.load();
      return result;
    },

    async update(id: string, tomlContent: string): Promise<Workflow> {
      const result = await apiPut<Workflow>(
        `/workflows/${encodeURIComponent(id)}`,
        {
          toml_content: tomlContent,
        },
      );
      await this.load();
      return result;
    },

    async getRawToml(id: string): Promise<string> {
      return apiGetText(`/workflows/${encodeURIComponent(id)}/raw`);
    },

    async remove(id: string): Promise<void> {
      await apiDelete(`/workflows/${encodeURIComponent(id)}`);
      await this.load();
    },

    async run(id: string): Promise<void> {
      await apiPost(`/workflows/${encodeURIComponent(id)}/run`, {});
    },

    async history(id: string): Promise<WorkflowRun[]> {
      return apiGet<WorkflowRun[]>(
        `/workflows/${encodeURIComponent(id)}/history`,
      );
    },
  };
}

export const workflowsStore = createWorkflowsStore();
