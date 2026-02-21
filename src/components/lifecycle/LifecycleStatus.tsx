/**
 * LifecycleStatus Component
 *
 * Displays the health status of all managed resources with controls
 * for monitoring, recovery, and resource management.
 */

import { useEffect } from "react";
import { useLifecycleStore } from "@/stores/lifecycleStore";
import { cn } from "@/lib/utils";
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  Clock,
  RefreshCw,
  Square,
  Trash2,
  XCircle,
  List,
} from "@/lib/icons";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tooltip } from "@/components/ui/tooltip";
import type { ResourceStatus } from "@/stores/lifecycleStore";

interface LifecycleStatusProps {
  className?: string;
  filterType?: string;
  showControls?: boolean;
  compact?: boolean;
}

export function LifecycleStatus({
  className,
  filterType,
  showControls = true,
  compact = false,
}: LifecycleStatusProps) {
  const {
    resources,
    stats,
    isLoading,
    error,
    isMonitoring,
    selectedResourceId,
    fetchAllResources,
    fetchResourcesByType,
    fetchStats,
    selectResource,
    retryResource,
    stopResource,
    killResource,
    setupEventListeners,
    cleanupEventListeners,
  } = useLifecycleStore();

  // Initialize event listeners and fetch initial data
  useEffect(() => {
    setupEventListeners();
    if (filterType) {
      fetchResourcesByType(filterType);
    } else {
      fetchAllResources();
    }
    fetchStats();

    return () => {
      cleanupEventListeners();
    };
  }, [
    filterType,
    fetchAllResources,
    fetchResourcesByType,
    fetchStats,
    setupEventListeners,
    cleanupEventListeners,
  ]);

  const getStateIcon = (state: ResourceStatus["state"]) => {
    switch (state) {
      case "idle":
        return <Clock className="h-4 w-4 text-muted-foreground" />;
      case "running":
        return <Activity className="h-4 w-4 text-green-500 animate-pulse" />;
      case "stuck":
        return <AlertTriangle className="h-4 w-4 text-red-500" />;
      case "recovering":
        return <RefreshCw className="h-4 w-4 text-yellow-500 animate-spin" />;
      case "completed":
        return <CheckCircle2 className="h-4 w-4 text-green-600" />;
      case "failed":
        return <XCircle className="h-4 w-4 text-red-600" />;
    }
  };

  const getStateColor = (state: ResourceStatus["state"]) => {
    switch (state) {
      case "idle":
        return "bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200";
      case "running":
        return "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200";
      case "stuck":
        return "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200";
      case "recovering":
        return "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200";
      case "completed":
        return "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200";
      case "failed":
        return "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200";
    }
  };

  const formatDate = (dateStr: string) => {
    try {
      return new Date(dateStr).toLocaleTimeString();
    } catch {
      return dateStr;
    }
  };

  const handleRetry = async (resourceId: string) => {
    await retryResource(resourceId);
  };

  const handleStop = async (resourceId: string) => {
    await stopResource(resourceId);
  };

  const handleKill = async (resourceId: string) => {
    if (window.confirm("Are you sure you want to force kill this resource?")) {
      await killResource(resourceId);
    }
  };

  if (error) {
    return (
      <div className={cn("p-4 text-red-500", className)}>
        <AlertTriangle className="inline h-4 w-4 mr-2" />
        {error}
      </div>
    );
  }

  return (
    <div className={cn("space-y-4", className)}>
      {/* Stats Overview */}
      {stats && !compact && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div className="rounded-lg border bg-card p-4">
            <div className="text-sm font-medium text-muted-foreground">
              Total Resources
            </div>
            <div className="text-2xl font-bold">{stats.totalResources}</div>
          </div>

          <div className="rounded-lg border bg-card p-4">
            <div className="text-sm font-medium text-muted-foreground">
              Running
            </div>
            <div className="text-2xl font-bold text-green-600">
              {stats.running}
            </div>
          </div>

          <div className="rounded-lg border bg-card p-4">
            <div className="text-sm font-medium text-muted-foreground">
              Stuck
            </div>
            <div className="text-2xl font-bold text-red-600">{stats.stuck}</div>
          </div>

          <div className="rounded-lg border bg-card p-4">
            <div className="text-sm font-medium text-muted-foreground">
              Monitoring
            </div>
            <div className="flex items-center gap-2">
              <div
                className={cn(
                  "h-2 w-2 rounded-full",
                  isMonitoring ? "bg-green-500" : "bg-gray-400"
                )}
              />
              <span className="text-sm">
                {isMonitoring ? "Active" : "Inactive"}
              </span>
            </div>
          </div>
        </div>
      )}

      {/* Resource List */}
      <div className="rounded-lg border bg-card">
        <div className="flex items-center justify-between border-b p-4">
          <h3 className="text-lg font-semibold flex items-center gap-2">
            <List className="h-5 w-5" />
            Resources
          </h3>
          <Button
            variant="outline"
            size="sm"
            onClick={() =>
              filterType
                ? fetchResourcesByType(filterType)
                : fetchAllResources()
            }
            disabled={isLoading}
          >
            <RefreshCw
              className={cn("h-4 w-4 mr-2", isLoading && "animate-spin")}
            />
            Refresh
          </Button>
        </div>
        <div className="p-4">
          {resources.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              No resources tracked
            </div>
          ) : compact ? (
            // Compact view - simple list
            <div className="space-y-2">
              {resources.map((resource) => (
                <div
                  key={resource.id}
                  className={cn(
                    "flex items-center justify-between p-2 rounded-lg",
                    "hover:bg-muted/50 cursor-pointer",
                    selectedResourceId === resource.id && "bg-muted"
                  )}
                  onClick={() => selectResource(resource.id)}
                >
                  <div className="flex items-center gap-2">
                    {getStateIcon(resource.state)}
                    <span className="font-mono text-sm truncate max-w-[200px]">
                      {resource.id}
                    </span>
                  </div>
                  <Badge className={getStateColor(resource.state)}>
                    {resource.state}
                  </Badge>
                </div>
              ))}
            </div>
          ) : (
            // Full view - table
            <div className="overflow-x-auto">
              <table className="w-full">
                <thead>
                  <tr className="border-b">
                    <th className="text-left p-2 text-sm font-medium">Resource ID</th>
                    <th className="text-left p-2 text-sm font-medium">Type</th>
                    <th className="text-left p-2 text-sm font-medium">State</th>
                    <th className="text-left p-2 text-sm font-medium">Progress</th>
                    <th className="text-left p-2 text-sm font-medium">Tier</th>
                    <th className="text-left p-2 text-sm font-medium">Created</th>
                    {showControls && <th className="text-left p-2 text-sm font-medium">Actions</th>}
                  </tr>
                </thead>
                <tbody>
                  {resources.map((resource) => (
                    <tr
                      key={resource.id}
                      className={cn(
                        "border-b cursor-pointer hover:bg-muted/50",
                        selectedResourceId === resource.id && "bg-muted"
                      )}
                      onClick={() => selectResource(resource.id)}
                    >
                      <td className="p-2 font-mono text-xs">
                        <Tooltip content={resource.id}>
                          <span className="truncate max-w-[150px] block">
                            {resource.id}
                          </span>
                        </Tooltip>
                      </td>
                      <td className="p-2">
                        <Badge variant="outline">{resource.resourceType}</Badge>
                      </td>
                      <td className="p-2">
                        <div className="flex items-center gap-2">
                          {getStateIcon(resource.state)}
                          <Badge className={getStateColor(resource.state)}>
                            {resource.state}
                          </Badge>
                        </div>
                      </td>
                      <td className="p-2">
                        {resource.progress !== undefined ? (
                          <div className="flex items-center gap-2">
                            <div className="w-16 h-2 bg-gray-200 rounded-full overflow-hidden">
                              <div
                                className="h-full bg-green-500"
                                style={{ width: `${(resource.progress ?? 0) * 100}%` }}
                              />
                            </div>
                            <span className="text-xs text-muted-foreground">
                              {Math.round((resource.progress ?? 0) * 100)}%
                            </span>
                          </div>
                        ) : (
                          <span className="text-muted-foreground">-</span>
                        )}
                      </td>
                      <td className="p-2">
                        <Badge variant="secondary">
                          Tier {resource.escalationTier}
                        </Badge>
                      </td>
                      <td className="p-2 text-xs text-muted-foreground">
                        {formatDate(resource.createdAt)}
                      </td>
                      {showControls && (
                        <td className="p-2">
                          <div className="flex items-center gap-1">
                            <Tooltip content="Retry">
                              <Button
                                variant="ghost"
                                size="icon"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleRetry(resource.id);
                                }}
                                disabled={
                                  resource.state !== "stuck" &&
                                  resource.state !== "failed"
                                }
                              >
                                <RefreshCw className="h-4 w-4" />
                              </Button>
                            </Tooltip>

                            <Tooltip content="Stop">
                              <Button
                                variant="ghost"
                                size="icon"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleStop(resource.id);
                                }}
                                disabled={resource.state !== "running"}
                              >
                                <Square className="h-4 w-4" />
                              </Button>
                            </Tooltip>

                            <Tooltip content="Kill">
                              <Button
                                variant="ghost"
                                size="icon"
                                className="text-red-500 hover:text-red-600"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleKill(resource.id);
                                }}
                                disabled={
                                  resource.state !== "stuck" &&
                                  resource.state !== "running"
                                }
                              >
                                <Trash2 className="h-4 w-4" />
                              </Button>
                            </Tooltip>
                          </div>
                        </td>
                      )}
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default LifecycleStatus;
