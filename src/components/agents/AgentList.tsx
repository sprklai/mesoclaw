/**
 * AgentList component - Displays a list of configured agents with status indicators.
 *
 * Features:
 * - Agent list with name, role, model, provider, and status
 * - Actions for edit, delete, duplicate, and toggle enable/disable
 * - Status indicators (idle, running, paused, error, completed)
 * - Responsive layout with mobile-friendly design
 */
import {
  Bot,
  MoreHorizontal,
  Pencil,
  Play,
  Power,
  PowerOff,
  Square,
  Trash2,
  Copy,
} from "@/lib/icons";
import type { AgentConfig, AgentStatus } from "@/lib/agent-config";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { EmptyState } from "@/components/ui/empty-state";
import { LoadingState } from "@/components/ui/loading-state";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { cn } from "@/lib/utils";

// ─── Types ────────────────────────────────────────────────────────────────

interface AgentListProps {
  agents: AgentConfig[];
  isLoading?: boolean;
  selectedAgentId?: string | null;
  onSelectAgent: (agent: AgentConfig) => void;
  onDeleteAgent: (agent: AgentConfig) => void;
  onDuplicateAgent: (agent: AgentConfig) => void;
  onToggleEnabled: (agent: AgentConfig) => void;
  onRunAgent: (agent: AgentConfig) => void;
  onCreateAgent: () => void;
  className?: string;
}

// ─── Helper Functions ─────────────────────────────────────────────────────

function getStatusIcon(status: AgentStatus) {
  switch (status) {
    case "running":
      return Play;
    case "idle":
      return Bot;
    case "paused":
      return Square;
    case "error":
      return PowerOff;
    case "completed":
      return Power;
    default:
      return Bot;
  }
}

// ─── Component ────────────────────────────────────────────────────────────

export function AgentList({
  agents,
  isLoading = false,
  selectedAgentId,
  onSelectAgent,
  onDeleteAgent,
  onDuplicateAgent,
  onToggleEnabled,
  onRunAgent,
  onCreateAgent,
  className,
}: AgentListProps) {
  if (isLoading) {
    return <LoadingState message="Loading agents..." />;
  }

  if (agents.length === 0) {
    return (
      <EmptyState
        icon={Bot}
        title="No agents configured"
        description="Create your first agent to start building multi-agent workflows."
        action={{
          label: "Create Agent",
          onClick: onCreateAgent,
        }}
        className={className}
      />
    );
  }

  return (
    <div className={cn("space-y-4", className)}>
      {/* Desktop: Table view */}
      <div className="hidden md:block rounded-lg border border-border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-12">Status</TableHead>
              <TableHead>Name</TableHead>
              <TableHead>Role</TableHead>
              <TableHead>Model</TableHead>
              <TableHead className="w-24">Enabled</TableHead>
              <TableHead className="w-20">Actions</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {agents.map((agent) => {
              const isSelected = selectedAgentId === agent.id;
              const StatusIcon = getStatusIcon("idle"); // ## TODO: Get actual status from run state

              return (
                <TableRow
                  key={agent.id}
                  className={cn(
                    "cursor-pointer",
                    isSelected && "bg-muted/50"
                  )}
                  onClick={() => onSelectAgent(agent)}
                >
                  <TableCell>
                    <StatusIcon
                      className={cn(
                        "h-4 w-4",
                        agent.isEnabled ? "text-primary" : "text-muted-foreground"
                      )}
                      aria-hidden="true"
                    />
                  </TableCell>
                  <TableCell className="font-medium">{agent.name}</TableCell>
                  <TableCell>
                    <Badge variant="outline" className="text-xs">
                      {agent.role}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-sm text-muted-foreground">
                    {agent.modelId}
                  </TableCell>
                  <TableCell>
                    <Badge
                      variant={agent.isEnabled ? "success" : "secondary"}
                      className="text-xs"
                    >
                      {agent.isEnabled ? "On" : "Off"}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          aria-label="Agent actions"
                        >
                          <MoreHorizontal className="h-4 w-4" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem
                          onClick={(e) => {
                            e.stopPropagation();
                            if (agent.isEnabled) {
                              onRunAgent(agent);
                            }
                          }}
                          className={!agent.isEnabled ? "opacity-50 pointer-events-none" : ""}
                        >
                          <Play className="mr-2 h-4 w-4" />
                          Run
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={(e) => {
                            e.stopPropagation();
                            onSelectAgent(agent);
                          }}
                        >
                          <Pencil className="mr-2 h-4 w-4" />
                          Edit
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={(e) => {
                            e.stopPropagation();
                            onDuplicateAgent(agent);
                          }}
                        >
                          <Copy className="mr-2 h-4 w-4" />
                          Duplicate
                        </DropdownMenuItem>
                        <DropdownMenuItem
                          onClick={(e) => {
                            e.stopPropagation();
                            onToggleEnabled(agent);
                          }}
                        >
                          {agent.isEnabled ? (
                            <>
                              <PowerOff className="mr-2 h-4 w-4" />
                              Disable
                            </>
                          ) : (
                            <>
                              <Power className="mr-2 h-4 w-4" />
                              Enable
                            </>
                          )}
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem
                          onClick={(e) => {
                            e.stopPropagation();
                            onDeleteAgent(agent);
                          }}
                          className="text-destructive focus:text-destructive"
                        >
                          <Trash2 className="mr-2 h-4 w-4" />
                          Delete
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              );
            })}
          </TableBody>
        </Table>
      </div>

      {/* Mobile: Card view */}
      <div className="md:hidden space-y-3">
        {agents.map((agent) => {
          const isSelected = selectedAgentId === agent.id;
          const StatusIcon = getStatusIcon("idle");

          return (
            <button
              key={agent.id}
              type="button"
              className={cn(
                "w-full rounded-lg border border-border p-4 text-left",
                "hover:bg-muted/50 transition-colors",
                isSelected && "bg-muted/50 border-primary"
              )}
              onClick={() => onSelectAgent(agent)}
            >
              <div className="flex items-start justify-between gap-4">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <StatusIcon
                      className={cn(
                        "h-4 w-4 shrink-0",
                        agent.isEnabled ? "text-primary" : "text-muted-foreground"
                      )}
                      aria-hidden="true"
                    />
                    <span className="font-medium truncate">{agent.name}</span>
                  </div>
                  <div className="mt-2 flex flex-wrap gap-2">
                    <Badge variant="outline" className="text-xs">
                      {agent.role}
                    </Badge>
                    <Badge
                      variant={agent.isEnabled ? "success" : "secondary"}
                      className="text-xs"
                    >
                      {agent.isEnabled ? "On" : "Off"}
                    </Badge>
                  </div>
                  <p className="mt-1 text-xs text-muted-foreground truncate">
                    {agent.modelId}
                  </p>
                </div>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 shrink-0"
                      aria-label="Agent actions"
                    >
                      <MoreHorizontal className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
                    <DropdownMenuItem
                      onClick={(e) => {
                        e.stopPropagation();
                        if (agent.isEnabled) {
                          onRunAgent(agent);
                        }
                      }}
                      className={!agent.isEnabled ? "opacity-50 pointer-events-none" : ""}
                    >
                      <Play className="mr-2 h-4 w-4" />
                      Run
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={(e) => {
                        e.stopPropagation();
                        onSelectAgent(agent);
                      }}
                    >
                      <Pencil className="mr-2 h-4 w-4" />
                      Edit
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={(e) => {
                        e.stopPropagation();
                        onDuplicateAgent(agent);
                      }}
                    >
                      <Copy className="mr-2 h-4 w-4" />
                      Duplicate
                    </DropdownMenuItem>
                    <DropdownMenuItem
                      onClick={(e) => {
                        e.stopPropagation();
                        onToggleEnabled(agent);
                      }}
                    >
                      {agent.isEnabled ? (
                        <>
                          <PowerOff className="mr-2 h-4 w-4" />
                          Disable
                        </>
                      ) : (
                        <>
                          <Power className="mr-2 h-4 w-4" />
                          Enable
                        </>
                      )}
                    </DropdownMenuItem>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem
                      onClick={(e) => {
                        e.stopPropagation();
                        onDeleteAgent(agent);
                      }}
                      className="text-destructive focus:text-destructive"
                    >
                      <Trash2 className="mr-2 h-4 w-4" />
                      Delete
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              </div>
            </button>
          );
        })}
      </div>
    </div>
  );
}
