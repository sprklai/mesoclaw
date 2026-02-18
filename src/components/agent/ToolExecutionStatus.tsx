/**
 * ToolExecutionStatus — shows a single tool call with its arguments,
 * a spinner while running, and an expandable result when finished.
 */

import { useState } from "react";

import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible";

import type { ToolExecution } from "@/stores/agentStore";

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Truncate a JSON argument object to a compact one-line representation. */
function truncateArgs(args: Record<string, unknown>, maxLen = 120): string {
  try {
    const raw = JSON.stringify(args);
    if (raw.length <= maxLen) return raw;
    return `${raw.slice(0, maxLen)}…`;
  } catch {
    return "(invalid args)";
  }
}

/** Duration since `startedAt` in seconds. */
function elapsed(startedAt: number, finishedAt: number | null): string {
  const ms = (finishedAt ?? Date.now()) - startedAt;
  return `${(ms / 1000).toFixed(1)}s`;
}

// ─── StatusIcon ───────────────────────────────────────────────────────────────

function StatusIcon({ status }: { status: ToolExecution["status"] }) {
  if (status === "running") {
    return (
      <span
        aria-label="Running"
        className="inline-block h-3 w-3 animate-spin rounded-full border-2 border-muted border-t-primary"
      />
    );
  }
  if (status === "success") {
    return (
      <span aria-label="Success" className="text-green-500 text-sm leading-none">
        ✓
      </span>
    );
  }
  return (
    <span aria-label="Error" className="text-destructive text-sm leading-none">
      ✗
    </span>
  );
}

// ─── ToolExecutionStatus ──────────────────────────────────────────────────────

interface ToolExecutionStatusProps {
  execution: ToolExecution;
  className?: string;
}

export function ToolExecutionStatus({ execution, className }: ToolExecutionStatusProps) {
  const [open, setOpen] = useState(false);
  const hasResult = execution.result !== null;

  return (
    <div
      className={cn(
        "rounded-md border bg-muted/30 px-3 py-2 text-xs font-mono",
        execution.status === "error" && "border-destructive/40 bg-destructive/5",
        execution.status === "success" && "border-green-500/20",
        className
      )}
    >
      {/* Header row */}
      <div className="flex items-center gap-2">
        <StatusIcon status={execution.status} />

        <span className="font-semibold text-foreground">{execution.toolName}</span>

        <span className="flex-1 truncate text-muted-foreground">
          {truncateArgs(execution.args)}
        </span>

        <Badge
          variant={
            execution.status === "running"
              ? "secondary"
              : execution.status === "success"
                ? "success"
                : "destructive"
          }
          className="shrink-0"
        >
          {elapsed(execution.startedAt, execution.finishedAt)}
        </Badge>
      </div>

      {/* Expandable result */}
      {hasResult && (
        <Collapsible open={open} onOpenChange={setOpen}>
          <CollapsibleTrigger className="mt-1 text-muted-foreground hover:text-foreground underline-offset-2 hover:underline focus:outline-none">
            {open ? "▾ hide result" : "▸ show result"}
          </CollapsibleTrigger>
          <CollapsibleContent>
            <pre className="mt-2 max-h-40 overflow-y-auto whitespace-pre-wrap break-words text-[10px] text-foreground/80">
              {execution.result}
            </pre>
          </CollapsibleContent>
        </Collapsible>
      )}
    </div>
  );
}
