import { apiGet, apiPost, apiPut, apiDelete } from "$lib/api/client";

export interface ScheduledJob {
  id: string;
  name: string;
  schedule: { type: "interval"; secs: number } | { type: "cron"; expr: string } | { type: "human"; datetime: string };
  session_target: "main" | "isolated";
  payload:
    | { type: "heartbeat" }
    | { type: "agent_turn"; prompt: string }
    | { type: "notify"; message: string }
    | { type: "send_via_channel"; channel: string; message: string };
  enabled: boolean;
  error_count: number;
  next_run: string | null;
  active_hours: { start_hour: number; end_hour: number } | null;
  delete_after_run: boolean;
}

export interface JobExecution {
  id: string;
  job_id: string;
  status: "success" | "failed" | "stuck" | "skipped";
  started_at: string;
  completed_at: string | null;
  error: string | null;
}

export interface SchedulerStatus {
  running: boolean;
  job_count: number;
}

function createSchedulerStore() {
  let jobs = $state<ScheduledJob[]>([]);
  let loading = $state(false);
  let status = $state<SchedulerStatus>({ running: false, job_count: 0 });

  return {
    get jobs() {
      return jobs;
    },
    get loading() {
      return loading;
    },
    get status() {
      return status;
    },

    async load() {
      loading = true;
      try {
        const [jobList, schedulerStatus] = await Promise.all([
          apiGet<ScheduledJob[]>("/scheduler/jobs").catch(
            () => [] as ScheduledJob[],
          ),
          apiGet<SchedulerStatus>("/scheduler/status").catch(
            () => ({ running: false, job_count: 0 }) as SchedulerStatus,
          ),
        ]);
        jobs = jobList;
        status = schedulerStatus;
      } finally {
        loading = false;
      }
    },

    async createJob(job: Partial<ScheduledJob>): Promise<string> {
      const result = await apiPost<{ id: string }>("/scheduler/jobs", {
        id: "",
        name: job.name ?? "",
        schedule: job.schedule,
        session_target: job.session_target ?? "main",
        payload: job.payload,
        active_hours: job.active_hours ?? null,
        delete_after_run: job.delete_after_run ?? false,
      });
      await this.load();
      return result.id;
    },

    async updateJob(id: string, job: Partial<ScheduledJob>): Promise<void> {
      await apiPut(`/scheduler/jobs/${encodeURIComponent(id)}`, {
        id,
        name: job.name ?? "",
        schedule: job.schedule,
        session_target: job.session_target ?? "main",
        payload: job.payload,
        active_hours: job.active_hours ?? null,
        delete_after_run: job.delete_after_run ?? false,
      });
      await this.load();
    },

    async toggleJob(id: string): Promise<boolean> {
      const result = await apiPut<{ id: string; enabled: boolean }>(
        `/scheduler/jobs/${encodeURIComponent(id)}/toggle`,
      );
      await this.load();
      return result.enabled;
    },

    async deleteJob(id: string): Promise<void> {
      await apiDelete(`/scheduler/jobs/${encodeURIComponent(id)}`);
      await this.load();
    },

    async getHistory(id: string): Promise<JobExecution[]> {
      return apiGet<JobExecution[]>(
        `/scheduler/jobs/${encodeURIComponent(id)}/history`,
      );
    },
  };
}

export const schedulerStore = createSchedulerStore();
