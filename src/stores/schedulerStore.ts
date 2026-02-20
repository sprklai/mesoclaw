/**
 * Zustand store for the Scheduler UI.
 *
 * Mirrors the Rust ScheduledJob / JobExecution types and wraps the scheduler
 * IPC commands wired to real Tauri backend commands.
 */

import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

import { extractErrorMessage } from "@/lib/error-utils";
import { withStoreLoading } from "@/lib/store-utils";

// ─── Types (mirror Rust serde output) ────────────────────────────────────────

export type ScheduleKind =
  | { type: "interval"; secs: number }
  | { type: "cron"; expr: string };

export type JobPayloadKind =
  | { type: "heartbeat" }
  | { type: "agent_turn"; prompt: string }
  | { type: "notify"; message: string };

export interface ScheduledJob {
  id: string;
  name: string;
  schedule: ScheduleKind;
  sessionTarget: "main" | "isolated";
  payload: JobPayloadKind;
  enabled: boolean;
  errorCount: number;
  nextRun: string | null; // ISO 8601
}

export interface JobExecution {
  jobId: string;
  startedAt: string;
  finishedAt: string | null;
  status: "success" | "failed" | "stuck" | "skipped";
  output: string | null;
  errorMessage: string | null;
}

// ─── Creation form ────────────────────────────────────────────────────────────

export interface JobCreationForm {
  name: string;
  scheduleType: "interval" | "cron";
  intervalSecs: number;
  cronExpr: string;
  payloadType: "heartbeat" | "agent_turn" | "notify";
  prompt: string;
  notifyMessage: string;
}

const DEFAULT_FORM: JobCreationForm = {
  name: "",
  scheduleType: "interval",
  intervalSecs: 3600,
  cronExpr: "0 * * * *",
  payloadType: "heartbeat",
  prompt: "",
  notifyMessage: "",
};

// ─── Store ────────────────────────────────────────────────────────────────────

interface SchedulerState {
  jobs: ScheduledJob[];
  loading: boolean;
  error: string | null;

  history: Record<string, JobExecution[]>; // jobId → executions
  historyLoading: string | null; // jobId being fetched

  form: JobCreationForm;
  formOpen: boolean;
  submitting: boolean;

  loadJobs: () => Promise<void>;
  loadHistory: (jobId: string) => Promise<void>;
  toggleJob: (jobId: string, enabled: boolean) => Promise<void>;
  deleteJob: (jobId: string) => Promise<void>;
  submitForm: () => Promise<void>;
  updateForm: (patch: Partial<JobCreationForm>) => void;
  openForm: () => void;
  closeForm: () => void;
}

export const useSchedulerStore = create<SchedulerState>((set, get) => ({
  jobs: [],
  loading: false,
  error: null,
  history: {},
  historyLoading: null,
  form: { ...DEFAULT_FORM },
  formOpen: false,
  submitting: false,

  loadJobs: async () => {
    await withStoreLoading(
      set,
      async () => {
        const jobs = await invoke<ScheduledJob[]>("list_jobs_command");
        set({ jobs });
        return jobs;
      },
      {
        loadingKey: "loading",
        onError: () => set({ jobs: [] }),
      }
    );
  },

  loadHistory: async (jobId) => {
    set({ historyLoading: jobId });
    try {
      const executions = await invoke<JobExecution[]>("job_history_command", {
        jobId,
      });
      set((s) => ({
        history: { ...s.history, [jobId]: executions },
        historyLoading: null,
      }));
    } catch {
      set({ historyLoading: null });
    }
  },

  toggleJob: async (jobId, enabled) => {
    try {
      await invoke("toggle_job_command", { jobId, enabled });
      set((s) => ({
        jobs: s.jobs.map((j) =>
          j.id === jobId ? { ...j, enabled } : j
        ),
      }));
    } catch (err) {
      set({ error: extractErrorMessage(err) });
    }
  },

  deleteJob: async (jobId) => {
    try {
      await invoke("delete_job_command", { jobId });
      set((s) => ({ jobs: s.jobs.filter((j) => j.id !== jobId) }));
    } catch (err) {
      set({ error: extractErrorMessage(err) });
    }
  },

  submitForm: async () => {
    const { form } = get();

    const schedule: ScheduleKind =
      form.scheduleType === "interval"
        ? { type: "interval", secs: form.intervalSecs }
        : { type: "cron", expr: form.cronExpr };

    const payload: JobPayloadKind =
      form.payloadType === "heartbeat"
        ? { type: "heartbeat" }
        : form.payloadType === "agent_turn"
          ? { type: "agent_turn", prompt: form.prompt }
          : { type: "notify", message: form.notifyMessage };

    await withStoreLoading(
      set,
      async () => {
        await invoke("create_job_command", {
          name: form.name,
          scheduleJson: schedule,
          payloadJson: payload,
        });
        await get().loadJobs();
        set({ formOpen: false, form: { ...DEFAULT_FORM } });
      },
      { loadingKey: "submitting" }
    );
  },

  updateForm: (patch) =>
    set((s) => ({ form: { ...s.form, ...patch } })),
  openForm: () => set({ formOpen: true, form: { ...DEFAULT_FORM } }),
  closeForm: () => set({ formOpen: false }),
}));
