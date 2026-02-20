/**
 * SessionHistoryViewer - View session history with filtering and search.
 *
 * Features:
 * - Session list with status, message count, token usage
 * - Filtering by agent, status, and date range
 * - Search functionality
 * - Session details expansion
 */
import {
  ChevronRight,
  Clock,
  History,
  MessageSquare,
  Search,
  Trash2,
  X,
} from "@/lib/icons";
import type { AgentConfig, AgentSessionSummary, AgentStatus } from "@/lib/agent-config";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { Input } from "@/components/ui/input";
import { LoadingState } from "@/components/ui/loading-state";
import { Select } from "@/components/ui/select";
import { cn } from "@/lib/utils";

import { useState, useMemo } from "react";

// ─── Types ────────────────────────────────────────────────────────────────

interface SessionHistoryViewerProps {
  sessions: AgentSessionSummary[];
  agents: AgentConfig[];
  isLoading?: boolean;
  onLoadSessions: (agentId?: string) => Promise<void>;
  onClearHistory: (agentId: string) => Promise<void>;
  onViewSession: (sessionId: string) => void;
  className?: string;
}

interface SessionFilters {
  agentId: string;
  status: AgentStatus | "all";
  search: string;
}

// ─── Helper Functions ──────────────────────────────────────────────────────

function getStatusBadgeVariant(status: AgentStatus) {
  switch (status) {
    case "running":
      return "default";
    case "completed":
      return "success";
    case "error":
      return "destructive";
    case "paused":
      return "warning";
    default:
      return "secondary";
  }
}

function formatRelativeTime(timestamp: number): string {
  const now = Date.now();
  const diff = now - timestamp;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) {
    return `${days}d ago`;
  }
  if (hours > 0) {
    return `${hours}h ago`;
  }
  if (minutes > 0) {
    return `${minutes}m ago`;
  }
  return "Just now";
}

function formatDuration(startedAt: number, completedAt?: number): string {
  if (!completedAt) {
    return "In progress";
  }
  const diff = completedAt - startedAt;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);

  if (hours > 0) {
    return `${hours}h ${minutes % 60}m`;
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds % 60}s`;
  }
  return `${seconds}s`;
}

// ─── Status Options ────────────────────────────────────────────────────────

const STATUS_OPTIONS = [
  { value: "all", label: "All Status" },
  { value: "completed", label: "Completed" },
  { value: "running", label: "Running" },
  { value: "error", label: "Error" },
  { value: "paused", label: "Paused" },
  { value: "idle", label: "Idle" },
];

// ─── Component ────────────────────────────────────────────────────────────

export function SessionHistoryViewer({
  sessions,
  agents,
  isLoading = false,
  onLoadSessions,
  onClearHistory,
  onViewSession,
  className,
}: SessionHistoryViewerProps) {
  const [filters, setFilters] = useState<SessionFilters>({
    agentId: "all",
    status: "all",
    search: "",
  });
  const [expandedSessionId, setExpandedSessionId] = useState<string | null>(null);

  // Build agent options for Select
  const agentOptions = [
    { value: "all", label: "All Agents" },
    ...agents.map((a) => ({ value: a.id, label: a.name })),
  ];

  // Filter sessions
  const filteredSessions = useMemo(() => {
    return sessions.filter((session) => {
      // Filter by agent
      if (filters.agentId !== "all" && session.agentId !== filters.agentId) {
        return false;
      }

      // Filter by status
      if (filters.status !== "all" && session.status !== filters.status) {
        return false;
      }

      // Filter by search
      if (filters.search) {
        const searchLower = filters.search.toLowerCase();
        return (
          session.agentName.toLowerCase().includes(searchLower) ||
          session.finalMessage?.toLowerCase().includes(searchLower) ||
          false
        );
      }

      return true;
    });
  }, [sessions, filters]);

  // ─── Handlers ────────────────────────────────────────────────────────────

  const handleAgentFilterChange = (value: string) => {
    setFilters((prev) => ({ ...prev, agentId: value }));
    if (value !== "all") {
      onLoadSessions(value);
    } else {
      onLoadSessions();
    }
  };

  const handleStatusFilterChange = (value: string) => {
    setFilters((prev) => ({ ...prev, status: value as AgentStatus | "all" }));
  };

  const handleSearchChange = (value: string) => {
    setFilters((prev) => ({ ...prev, search: value }));
  };

  const clearFilters = () => {
    setFilters({ agentId: "all", status: "all", search: "" });
  };

  const toggleExpanded = (sessionId: string) => {
    setExpandedSessionId((prev) => (prev === sessionId ? null : sessionId));
  };

  const handleClearHistory = async (agentId: string) => {
    if (confirm("Are you sure you want to clear all session history for this agent?")) {
      await onClearHistory(agentId);
    }
  };

  // ─── Render ──────────────────────────────────────────────────────────────

  if (isLoading) {
    return <LoadingState message="Loading sessions..." className={className} />;
  }

  return (
    <div className={cn("flex flex-col h-full", className)}>
      {/* Header with filters */}
      <div className="border-b border-border px-4 py-3 space-y-3">
        <div className="flex items-center justify-between">
          <h3 className="font-medium flex items-center gap-2">
            <History className="h-4 w-4" />
            Session History
          </h3>
          {filters.agentId !== "all" && (
            <Button
              variant="ghost"
              size="sm"
              className="text-destructive hover:text-destructive"
              onClick={() => handleClearHistory(filters.agentId)}
            >
              <Trash2 className="mr-2 h-4 w-4" />
              Clear History
            </Button>
          )}
        </div>

        <div className="flex flex-col sm:flex-row gap-3">
          {/* Search */}
          <div className="relative flex-1">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search sessions..."
              value={filters.search}
              onChange={(e) => handleSearchChange(e.target.value)}
              className="pl-9"
            />
            {filters.search && (
              <Button
                variant="ghost"
                size="icon"
                className="absolute right-1 top-1/2 -translate-y-1/2 h-7 w-7"
                onClick={() => handleSearchChange("")}
              >
                <X className="h-3 w-3" />
              </Button>
            )}
          </div>

          {/* Agent filter */}
          <Select
            value={filters.agentId}
            onValueChange={handleAgentFilterChange}
            options={agentOptions}
            className="w-full sm:w-[180px]"
          />

          {/* Status filter */}
          <Select
            value={filters.status}
            onValueChange={handleStatusFilterChange}
            options={STATUS_OPTIONS}
            className="w-full sm:w-[140px]"
          />
        </div>

        {/* Active filters */}
        {(filters.agentId !== "all" ||
          filters.status !== "all" ||
          filters.search) && (
          <div className="flex items-center gap-2 text-xs">
            <span className="text-muted-foreground">Filters:</span>
            <Button
              variant="ghost"
              size="sm"
              className="h-6 text-xs"
              onClick={clearFilters}
            >
              Clear all
            </Button>
          </div>
        )}
      </div>

      {/* Session list */}
      <div className="flex-1 overflow-auto">
        {filteredSessions.length === 0 ? (
          <EmptyState
            icon={History}
            title="No sessions found"
            description={
              sessions.length === 0
                ? "No session history available yet."
                : "No sessions match your filters."
            }
            className="h-full"
          />
        ) : (
          <div className="divide-y divide-border">
            {filteredSessions.map((session) => {
              const isExpanded = expandedSessionId === session.id;
              const agent = agents.find((a) => a.id === session.agentId);

              return (
                <div
                  key={session.id}
                  className={cn(
                    "hover:bg-muted/30 transition-colors",
                    isExpanded && "bg-muted/30"
                  )}
                >
                  {/* Session row */}
                  <button
                    type="button"
                    className="w-full px-4 py-3 text-left"
                    onClick={() => toggleExpanded(session.id)}
                  >
                    <div className="flex items-start justify-between gap-4">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <Badge
                            variant={getStatusBadgeVariant(session.status)}
                            className="text-xs"
                          >
                            {session.status}
                          </Badge>
                          <span className="font-medium truncate">
                            {session.agentName}
                          </span>
                          {agent && (
                            <Badge variant="outline" className="text-xs">
                              {agent.role}
                            </Badge>
                          )}
                        </div>
                        <div className="mt-1 flex items-center gap-4 text-xs text-muted-foreground">
                          <span className="flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {formatRelativeTime(session.startedAt)}
                          </span>
                          <span className="flex items-center gap-1">
                            <MessageSquare className="h-3 w-3" />
                            {session.messageCount} messages
                          </span>
                          {session.tokenUsage && (
                            <span>{session.tokenUsage.toLocaleString()} tokens</span>
                          )}
                        </div>
                      </div>
                      <ChevronRight
                        className={cn(
                          "h-4 w-4 text-muted-foreground transition-transform",
                          isExpanded && "rotate-90"
                        )}
                      />
                    </div>
                  </button>

                  {/* Expanded details */}
                  {isExpanded && (
                    <div className="px-4 pb-3 space-y-3 border-t border-border bg-muted/20">
                      <div className="pt-3 grid grid-cols-2 sm:grid-cols-4 gap-3 text-sm">
                        <div>
                          <span className="text-muted-foreground">Started</span>
                          <p className="font-medium">
                            {new Date(session.startedAt).toLocaleString()}
                          </p>
                        </div>
                        {session.completedAt && (
                          <div>
                            <span className="text-muted-foreground">Completed</span>
                            <p className="font-medium">
                              {new Date(session.completedAt).toLocaleString()}
                            </p>
                          </div>
                        )}
                        <div>
                          <span className="text-muted-foreground">Duration</span>
                          <p className="font-medium">
                            {formatDuration(session.startedAt, session.completedAt)}
                          </p>
                        </div>
                        <div>
                          <span className="text-muted-foreground">Session ID</span>
                          <p className="font-medium font-mono text-xs truncate">
                            {session.id}
                          </p>
                        </div>
                      </div>

                      {session.finalMessage && (
                        <div>
                          <span className="text-sm text-muted-foreground">
                            Final Message
                          </span>
                          <p className="mt-1 text-sm bg-background p-3 rounded-md border border-border max-h-[200px] overflow-auto">
                            {session.finalMessage}
                          </p>
                        </div>
                      )}

                      <div className="flex justify-end gap-2">
                        <Button
                          variant="outline"
                          size="sm"
                          onClick={() => onViewSession(session.id)}
                        >
                          View Full Session
                        </Button>
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
