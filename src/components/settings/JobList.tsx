/**
 * JobList — table of scheduled jobs with toggle, delete, and creation form.
 */

import { useEffect } from "react";

import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { CronBuilder } from "./CronBuilder";
import { useSchedulerStore } from "@/stores/schedulerStore";
import type { ScheduledJob } from "@/stores/schedulerStore";

// ─── Helpers ──────────────────────────────────────────────────────────────────

function scheduleLabel(job: ScheduledJob): string {
  const s = job.schedule;
  if (s.type === "interval") {
    const secs = s.secs;
    if (secs < 60) return `every ${secs}s`;
    if (secs < 3600) return `every ${Math.round(secs / 60)}m`;
    return `every ${Math.round(secs / 3600)}h`;
  }
  return s.expr;
}

function payloadLabel(job: ScheduledJob): string {
  const p = job.payload;
  if (p.type === "heartbeat") return "Heartbeat";
  if (p.type === "agent_turn")
    return `Agent: ${p.prompt.slice(0, 40)}${p.prompt.length > 40 ? "…" : ""}`;
  if (p.type === "notify") return `Notify: ${p.message.slice(0, 30)}`;
  return (p as { type: string }).type;
}

function formatNextRun(iso: string | null): string {
  if (!iso) return "—";
  try {
    return new Date(iso).toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

// ─── CreateJobForm ────────────────────────────────────────────────────────────

function CreateJobForm() {
  const form = useSchedulerStore((s) => s.form);
  const submitting = useSchedulerStore((s) => s.submitting);
  const error = useSchedulerStore((s) => s.error);
  const updateForm = useSchedulerStore((s) => s.updateForm);
  const submitForm = useSchedulerStore((s) => s.submitForm);
  const closeForm = useSchedulerStore((s) => s.closeForm);

  return (
    <div className="rounded-md border bg-muted/20 p-4 flex flex-col gap-4">
      <p className="font-semibold text-sm">New Scheduled Job</p>

      {/* Name */}
      <div className="flex flex-col gap-1">
        <label className="text-xs text-muted-foreground">Name</label>
        <Input
          value={form.name}
          onChange={(e) => updateForm({ name: e.target.value })}
          placeholder="My heartbeat job"
        />
      </div>

      {/* Schedule type */}
      <div className="flex items-center gap-4">
        <label className="text-xs text-muted-foreground">Schedule</label>
        <label className="flex items-center gap-1.5 text-xs cursor-pointer">
          <input
            type="radio"
            checked={form.scheduleType === "interval"}
            onChange={() => updateForm({ scheduleType: "interval" })}
          />
          Interval
        </label>
        <label className="flex items-center gap-1.5 text-xs cursor-pointer">
          <input
            type="radio"
            checked={form.scheduleType === "cron"}
            onChange={() => updateForm({ scheduleType: "cron" })}
          />
          Cron
        </label>
      </div>

      {form.scheduleType === "interval" ? (
        <div className="flex items-center gap-2">
          <label className="text-xs text-muted-foreground w-20">Every (secs)</label>
          <Input
            type="number"
            min={1}
            value={form.intervalSecs}
            onChange={(e) => updateForm({ intervalSecs: Number(e.target.value) })}
            className="w-32"
          />
        </div>
      ) : (
        <CronBuilder
          value={form.cronExpr}
          onChange={(expr) => updateForm({ cronExpr: expr })}
        />
      )}

      {/* Payload type */}
      <div className="flex flex-col gap-1">
        <label className="text-xs text-muted-foreground">Action</label>
        <select
          value={form.payloadType}
          onChange={(e) =>
            updateForm({
              payloadType: e.target.value as "heartbeat" | "agent_turn" | "notify",
            })
          }
          className="rounded-md border bg-background text-foreground px-2 py-1 text-xs focus:outline-none focus-visible:ring-2 focus-visible:ring-ring w-fit"
        >
          <option value="heartbeat">Run Heartbeat checklist</option>
          <option value="agent_turn">Agent Turn (custom prompt)</option>
          <option value="notify">Publish notification</option>
        </select>
      </div>

      {form.payloadType === "agent_turn" && (
        <div className="flex flex-col gap-1">
          <label className="text-xs text-muted-foreground">Prompt</label>
          <Input
            value={form.prompt}
            onChange={(e) => updateForm({ prompt: e.target.value })}
            placeholder="Summarise today's work…"
          />
        </div>
      )}

      {form.payloadType === "notify" && (
        <div className="flex flex-col gap-1">
          <label className="text-xs text-muted-foreground">Message</label>
          <Input
            value={form.notifyMessage}
            onChange={(e) => updateForm({ notifyMessage: e.target.value })}
            placeholder="Reminder: check progress"
          />
        </div>
      )}

      {error && <p className="text-xs text-destructive">{error}</p>}

      <div className="flex gap-2 justify-end">
        <Button variant="ghost" size="sm" onClick={closeForm}>
          Cancel
        </Button>
        <Button
          variant="default"
          size="sm"
          disabled={!form.name.trim() || submitting}
          onClick={submitForm}
        >
          {submitting ? "Creating…" : "Create Job"}
        </Button>
      </div>
    </div>
  );
}

// ─── JobRow ───────────────────────────────────────────────────────────────────

function JobRow({ job }: { job: ScheduledJob }) {
  const toggleJob = useSchedulerStore((s) => s.toggleJob);
  const deleteJob = useSchedulerStore((s) => s.deleteJob);

  return (
    <tr className="border-b last:border-0 text-sm hover:bg-muted/30">
      <td className="py-2 px-3 font-medium">{job.name}</td>
      <td className="py-2 px-3 font-mono text-xs text-muted-foreground">
        {scheduleLabel(job)}
      </td>
      <td className="py-2 px-3 text-xs text-muted-foreground">
        {payloadLabel(job)}
      </td>
      <td className="py-2 px-3 text-xs text-muted-foreground">
        {formatNextRun(job.nextRun)}
      </td>
      <td className="py-2 px-3">
        {job.errorCount > 0 && (
          <Badge variant="destructive" className="text-xs">
            {job.errorCount} err
          </Badge>
        )}
      </td>
      <td className="py-2 px-3">
        <Switch
          checked={job.enabled}
          onCheckedChange={(v) => toggleJob(job.id, v)}
          aria-label={job.enabled ? "Disable job" : "Enable job"}
        />
      </td>
      <td className="py-2 px-3">
        <Button
          variant="ghost"
          size="sm"
          className="text-destructive hover:text-destructive"
          onClick={() => deleteJob(job.id)}
        >
          Delete
        </Button>
      </td>
    </tr>
  );
}

// ─── JobList ─────────────────────────────────────────────────────────────────

interface JobListProps {
  className?: string;
}

export function JobList({ className }: JobListProps) {
  const jobs = useSchedulerStore((s) => s.jobs);
  const loading = useSchedulerStore((s) => s.loading);
  const error = useSchedulerStore((s) => s.error);
  const formOpen = useSchedulerStore((s) => s.formOpen);
  const loadJobs = useSchedulerStore((s) => s.loadJobs);
  const openForm = useSchedulerStore((s) => s.openForm);

  useEffect(() => {
    loadJobs();
  }, [loadJobs]);

  return (
    <div className={cn("flex flex-col gap-4", className)}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <p className="text-sm font-semibold">Scheduled Jobs</p>
        <Button variant="outline" size="sm" onClick={openForm} disabled={formOpen}>
          + Add Job
        </Button>
      </div>

      {/* Creation form */}
      {formOpen && <CreateJobForm />}

      {/* Error */}
      {error && !formOpen && (
        <p className="text-xs text-destructive">{error}</p>
      )}

      {/* Loading */}
      {loading && (
        <p className="text-xs text-muted-foreground animate-pulse">Loading jobs…</p>
      )}

      {/* Empty */}
      {!loading && jobs.length === 0 && (
        <p className="text-center text-sm text-muted-foreground py-8">
          No scheduled jobs yet.
        </p>
      )}

      {/* Table */}
      {jobs.length > 0 && (
        <div className="overflow-x-auto rounded-md border">
          <table className="w-full">
            <thead>
              <tr className="border-b bg-muted/30 text-xs text-muted-foreground">
                <th className="py-2 px-3 text-left font-medium">Name</th>
                <th className="py-2 px-3 text-left font-medium">Schedule</th>
                <th className="py-2 px-3 text-left font-medium">Action</th>
                <th className="py-2 px-3 text-left font-medium">Next run</th>
                <th className="py-2 px-3 text-left font-medium">Errors</th>
                <th className="py-2 px-3 text-left font-medium">Active</th>
                <th className="py-2 px-3" />
              </tr>
            </thead>
            <tbody>
              {jobs.map((job) => (
                <JobRow key={job.id} job={job} />
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
