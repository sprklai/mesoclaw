/**
 * AgentProgress — compact panel showing the agent loop's live status.
 *
 * Displays:
 *  • Session status badge
 *  • Iteration count (increments on each tool call)
 *  • Cancel button (only while running or awaiting approval)
 *  • Expandable execution log (most recent ToolExecution entries)
 */

import { useState } from "react";

import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Collapsible, CollapsibleContent } from "@/components/ui/collapsible";
import { ToolExecutionStatus } from "./ToolExecutionStatus";
import { useAgentStore } from "@/stores/agentStore";

import type { AgentSessionStatus } from "@/stores/agentStore";

// ─── Helpers ──────────────────────────────────────────────────────────────────

const STATUS_LABEL: Record<AgentSessionStatus, string> = {
  idle: "Idle",
  running: "Running",
  awaiting_approval: "Awaiting Approval",
  complete: "Done",
  error: "Error",
};

const STATUS_VARIANT: Record<
  AgentSessionStatus,
  "default" | "secondary" | "destructive" | "outline" | "success"
> = {
  idle: "outline",
  running: "secondary",
  awaiting_approval: "default",
  complete: "success",
  error: "destructive",
};

// ─── AgentProgress ────────────────────────────────────────────────────────────

interface AgentProgressProps {
  className?: string;
  /** Maximum number of tool executions to show in the log (default 10). */
  maxVisible?: number;
}

export function AgentProgress({ className, maxVisible = 10 }: AgentProgressProps) {
  const session = useAgentStore((s) => s.session);
  const executions = useAgentStore((s) => s.executions);
  const cancelSession = useAgentStore((s) => s.cancelSession);

  const [logOpen, setLogOpen] = useState(false);

  if (!session) return null;

  const { status, iterationCount } = session;
  const isActive = status === "running" || status === "awaiting_approval";
  const visible = executions.slice(0, maxVisible);

  return (
    <div
      className={cn(
        "rounded-lg border bg-card p-3 text-sm shadow-sm",
        className
      )}
    >
      {/* ── Status row ─────────────────────────────────────────── */}
      <div className="flex items-center gap-3">
        {/* Status badge */}
        <Badge variant={STATUS_VARIANT[status]}>{STATUS_LABEL[status]}</Badge>

        {/* Spinner while running */}
        {status === "running" && (
          <span
            aria-label="Running"
            className="inline-block h-3 w-3 animate-spin rounded-full border-2 border-muted border-t-primary"
          />
        )}

        {/* Iteration counter */}
        <span className="text-muted-foreground">
          {iterationCount === 0
            ? "no iterations yet"
            : `${iterationCount} iteration${iterationCount === 1 ? "" : "s"}`}
        </span>

        {/* Spacer */}
        <span className="flex-1" />

        {/* Cancel */}
        {isActive && (
          <Button
            variant="destructive"
            size="sm"
            onClick={() => cancelSession()}
          >
            Cancel
          </Button>
        )}

        {/* Execution log toggle */}
        {executions.length > 0 && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => setLogOpen((v) => !v)}
          >
            {logOpen ? "▾ hide log" : "▸ show log"}
          </Button>
        )}
      </div>

      {/* ── Final message ──────────────────────────────────────── */}
      {status === "complete" && session.finalMessage && (
        <p className="mt-2 text-xs text-muted-foreground leading-relaxed">
          {session.finalMessage}
        </p>
      )}

      {/* ── Execution log ──────────────────────────────────────── */}
      {executions.length > 0 && (
        <Collapsible open={logOpen} onOpenChange={setLogOpen}>
          <CollapsibleContent>
            <div className="mt-3 flex flex-col gap-1.5">
              {visible.map((ex) => (
                <ToolExecutionStatus key={ex.id} execution={ex} />
              ))}
              {executions.length > maxVisible && (
                <p className="text-center text-xs text-muted-foreground">
                  … {executions.length - maxVisible} more
                </p>
              )}
            </div>
          </CollapsibleContent>
        </Collapsible>
      )}
    </div>
  );
}
