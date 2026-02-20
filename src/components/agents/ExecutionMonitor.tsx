/**
 * ExecutionMonitor - Real-time execution monitor for multi-agent sessions.
 *
 * Features:
 * - Active runs display with status
 * - Message flow visualization
 * - Tool call status tracking
 * - Cancel functionality
 */
import {
  AlertCircle,
  CheckCircle2,
  Clock,
  Loader2,
  Pause,
  Square,
  XCircle,
  Zap,
} from "@/lib/icons";
import type { AgentRun, ToolCallRecord } from "@/lib/agent-config";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { LoadingState } from "@/components/ui/loading-state";
import { cn } from "@/lib/utils";

import { useState, useEffect } from "react";

// ─── Types ────────────────────────────────────────────────────────────────

interface ExecutionMonitorProps {
  activeRuns: AgentRun[];
  isLoading?: boolean;
  onLoadRuns: () => Promise<void>;
  onCancelRun: (runId: string) => Promise<void>;
  onViewRun: (runId: string) => void;
  className?: string;
}

// ─── Helper Functions ──────────────────────────────────────────────────────

function getRunStatusIcon(status: AgentRun["status"]) {
  switch (status) {
    case "running":
      return Loader2;
    case "completed":
      return CheckCircle2;
    case "error":
      return XCircle;
    case "paused":
      return Pause;
    default:
      return Clock;
  }
}

function getToolStatusIcon(status: ToolCallRecord["status"]) {
  switch (status) {
    case "running":
      return Loader2;
    case "success":
      return CheckCircle2;
    case "error":
      return XCircle;
    case "pending":
      return Clock;
    default:
      return AlertCircle;
  }
}

function getToolStatusColor(status: ToolCallRecord["status"]) {
  switch (status) {
    case "running":
      return "text-primary";
    case "success":
      return "text-green-500";
    case "error":
      return "text-destructive";
    case "pending":
      return "text-muted-foreground";
    default:
      return "text-muted-foreground";
  }
}

// ─── Component ────────────────────────────────────────────────────────────

export function ExecutionMonitor({
  activeRuns,
  isLoading = false,
  onLoadRuns,
  onCancelRun,
  onViewRun,
  className,
}: ExecutionMonitorProps) {
  const [expandedRunId, setExpandedRunId] = useState<string | null>(null);
  const [autoRefresh, setAutoRefresh] = useState(true);

  // Auto-refresh active runs
  useEffect(() => {
    if (!autoRefresh || activeRuns.length === 0) return;

    const interval = setInterval(() => {
      onLoadRuns();
    }, 5000);

    return () => clearInterval(interval);
  }, [autoRefresh, activeRuns.length, onLoadRuns]);

  // ─── Handlers ────────────────────────────────────────────────────────────

  const toggleExpanded = (runId: string) => {
    setExpandedRunId((prev) => (prev === runId ? null : runId));
  };

  const handleCancel = async (runId: string) => {
    if (confirm("Are you sure you want to cancel this run?")) {
      await onCancelRun(runId);
    }
  };

  // ─── Render ──────────────────────────────────────────────────────────────

  if (isLoading) {
    return <LoadingState message="Loading active runs..." className={className} />;
  }

  return (
    <div className={cn("flex flex-col h-full", className)}>
      {/* Header */}
      <div className="border-b border-border px-4 py-3">
        <div className="flex items-center justify-between">
          <h3 className="font-medium flex items-center gap-2">
            <Zap className="h-4 w-4" />
            Execution Monitor
            {activeRuns.length > 0 && (
              <Badge variant="default" className="ml-2">
                {activeRuns.length} active
              </Badge>
            )}
          </h3>
          <label className="flex items-center gap-2 text-sm">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
              className="rounded border-input"
            />
            Auto-refresh
          </label>
        </div>
      </div>

      {/* Runs list */}
      <div className="flex-1 overflow-auto">
        {activeRuns.length === 0 ? (
          <EmptyState
            icon={Zap}
            title="No active runs"
            description="Start an agent session to see real-time execution status."
            className="h-full"
          />
        ) : (
          <div className="divide-y divide-border">
            {activeRuns.map((run) => {
              const isExpanded = expandedRunId === run.id;
              const StatusIcon = getRunStatusIcon(run.status);
              const isRunning = run.status === "running";

              return (
                <div
                  key={run.id}
                  className={cn(
                    "hover:bg-muted/30 transition-colors",
                    isExpanded && "bg-muted/30"
                  )}
                >
                  {/* Run row */}
                  <button
                    type="button"
                    className="w-full px-4 py-3 text-left"
                    onClick={() => toggleExpanded(run.id)}
                  >
                    <div className="flex items-start justify-between gap-4">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <StatusIcon
                            className={cn(
                              "h-4 w-4",
                              isRunning && "animate-spin text-primary",
                              run.status === "completed" && "text-green-500",
                              run.status === "error" && "text-destructive"
                            )}
                          />
                          <span className="font-medium truncate">
                            {run.agentName}
                          </span>
                          <Badge
                            variant={run.status === "running" ? "default" : "secondary"}
                            className="text-xs"
                          >
                            {run.status}
                          </Badge>
                        </div>
                        <div className="mt-1 flex items-center gap-4 text-xs text-muted-foreground">
                          <span className="flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {new Date(run.startedAt).toLocaleTimeString()}
                          </span>
                          <span className="flex items-center gap-1">
                            Iteration {run.iterations}
                          </span>
                          {run.currentTool && (
                            <span className="flex items-center gap-1">
                              <Zap className="h-3 w-3" />
                              {run.currentTool}
                            </span>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        {isRunning && (
                          <Button
                            variant="ghost"
                            size="sm"
                            className="text-destructive hover:text-destructive"
                            onClick={(e) => {
                              e.stopPropagation();
                              handleCancel(run.id);
                            }}
                          >
                            <Square className="h-4 w-4" />
                          </Button>
                        )}
                      </div>
                    </div>
                  </button>

                  {/* Expanded details */}
                  {isExpanded && (
                    <div className="px-4 pb-3 space-y-3 border-t border-border bg-muted/20">
                      {/* Run details */}
                      <div className="pt-3 grid grid-cols-3 gap-3 text-sm">
                        <div>
                          <span className="text-muted-foreground">Session ID</span>
                          <p className="font-medium font-mono text-xs truncate">
                            {run.sessionId}
                          </p>
                        </div>
                        <div>
                          <span className="text-muted-foreground">Run ID</span>
                          <p className="font-medium font-mono text-xs truncate">
                            {run.id}
                          </p>
                        </div>
                        <div>
                          <span className="text-muted-foreground">Tool Calls</span>
                          <p className="font-medium">{run.toolCalls.length}</p>
                        </div>
                      </div>

                      {/* Tool calls timeline */}
                      {run.toolCalls.length > 0 && (
                        <div>
                          <span className="text-sm text-muted-foreground">
                            Tool Call Timeline
                          </span>
                          <div className="mt-2 space-y-2">
                            {run.toolCalls.slice(-10).map((toolCall) => {
                              const ToolIcon = getToolStatusIcon(toolCall.status);
                              const isToolRunning = toolCall.status === "running";

                              return (
                                <div
                                  key={toolCall.id}
                                  className="flex items-start gap-3 p-2 bg-background rounded-md border border-border text-sm"
                                >
                                  <ToolIcon
                                    className={cn(
                                      "h-4 w-4 mt-0.5 shrink-0",
                                      getToolStatusColor(toolCall.status),
                                      isToolRunning && "animate-spin"
                                    )}
                                  />
                                  <div className="flex-1 min-w-0">
                                    <div className="flex items-center gap-2">
                                      <span className="font-medium">
                                        {toolCall.toolName}
                                      </span>
                                      <Badge
                                        variant="secondary"
                                        className="text-xs"
                                      >
                                        {toolCall.status}
                                      </Badge>
                                    </div>
                                    {Object.keys(toolCall.args).length > 0 && (
                                      <pre className="mt-1 text-xs text-muted-foreground overflow-auto max-h-[60px]">
                                        {JSON.stringify(toolCall.args, null, 2)}
                                      </pre>
                                    )}
                                    {toolCall.result && (
                                      <div className="mt-2 p-2 bg-muted/50 rounded text-xs max-h-[100px] overflow-auto">
                                        {toolCall.result}
                                      </div>
                                    )}
                                  </div>
                                  <span className="text-xs text-muted-foreground shrink-0">
                                    {new Date(toolCall.startedAt).toLocaleTimeString()}
                                  </span>
                                </div>
                              );
                            })}
                          </div>
                        </div>
                      )}

                      {/* Actions */}
                      <div className="flex justify-end gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => onViewRun(run.id)}
                        >
                          View Full Session
                        </Button>
                        {isRunning && (
                          <Button
                            variant="destructive"
                            size="sm"
                            onClick={() => handleCancel(run.id)}
                          >
                            Cancel Run
                          </Button>
                        )}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}
