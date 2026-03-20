import { apiGet, apiPost, apiPut, apiDelete, getToken, getBaseUrl } from "$lib/api/client";

export interface WorkflowStep {
  name: string;
  type: string;
  depends_on: string[];
  tool?: string;
  prompt?: string;
  seconds?: number;
}

export interface Workflow {
  id: string;
  name: string;
  description: string;
  schedule: string | null;
  steps: WorkflowStep[];
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

function createWorkflowsStore() {
  let workflows = $state<Workflow[]>([]);
  let loading = $state(false);

  return {
    get workflows() {
      return workflows;
    },
    get loading() {
      return loading;
    },

    async load() {
      loading = true;
      try {
        workflows = await apiGet<Workflow[]>("/workflows").catch(
          () => [] as Workflow[],
        );
      } finally {
        loading = false;
      }
    },

    async create(tomlContent: string): Promise<Workflow> {
      const result = await apiPost<Workflow>("/workflows", { toml_content: tomlContent });
      await this.load();
      return result;
    },

    async update(id: string, tomlContent: string): Promise<Workflow> {
      const result = await apiPut<Workflow>(`/workflows/${encodeURIComponent(id)}`, {
        toml_content: tomlContent,
      });
      await this.load();
      return result;
    },

    async getRawToml(id: string): Promise<string> {
      const baseUrl = getBaseUrl();
      const token = getToken();
      const headers: Record<string, string> = {};
      if (token) headers["Authorization"] = `Bearer ${token}`;
      const response = await fetch(`${baseUrl}/workflows/${encodeURIComponent(id)}/raw`, {
        headers,
      });
      if (!response.ok) throw new Error(`Failed to fetch raw TOML: ${response.statusText}`);
      return response.text();
    },

    async remove(id: string): Promise<void> {
      await apiDelete(`/workflows/${encodeURIComponent(id)}`);
      await this.load();
    },

    async run(id: string): Promise<WorkflowRun> {
      return apiPost<WorkflowRun>(
        `/workflows/${encodeURIComponent(id)}/run`,
        {},
      );
    },

    async history(id: string): Promise<WorkflowRun[]> {
      return apiGet<WorkflowRun[]>(
        `/workflows/${encodeURIComponent(id)}/history`,
      );
    },
  };
}

export const workflowsStore = createWorkflowsStore();
