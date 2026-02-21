/**
 * ModuleList — card grid of discovered sidecar modules with start/stop and
 * inline detail view.
 */

import { useEffect } from "react";

import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { ModuleDetail } from "./ModuleDetail";
import { ModuleScaffold } from "./ModuleScaffold";
import { useModuleStore } from "@/stores/moduleStore";
import type { ModuleEntry, ModuleStatus } from "@/stores/moduleStore";

// ─── Helpers ──────────────────────────────────────────────────────────────────

function statusDot(status: ModuleStatus): string {
  switch (status) {
    case "running":
      return "bg-green-500";
    case "starting":
      return "bg-yellow-400 animate-pulse";
    case "error":
      return "bg-destructive";
    default:
      return "bg-muted-foreground/40";
  }
}

function typeBadgeVariant(
  type: string,
): "default" | "secondary" | "outline" | "destructive" {
  if (type === "mcp") return "default";
  if (type === "service") return "secondary";
  return "outline";
}

// ─── ModuleCard ───────────────────────────────────────────────────────────────

interface ModuleCardProps {
  entry: ModuleEntry;
  isSelected: boolean;
  onSelect: () => void;
}

function ModuleCard({ entry, isSelected, onSelect }: ModuleCardProps) {
  const startModule = useModuleStore((s) => s.startModule);
  const stopModule = useModuleStore((s) => s.stopModule);

  const { manifest, status } = entry;
  const { module: info, runtime } = manifest;

  const isRunning = status === "running" || status === "starting";

  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "w-full rounded-lg border bg-card p-4 text-left transition-colors hover:bg-accent/50",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        isSelected && "border-primary bg-accent/30",
      )}
    >
      <div className="flex items-start gap-3">
        {/* Status dot */}
        <span
          className={cn(
            "mt-1 h-2 w-2 rounded-full shrink-0",
            statusDot(status),
          )}
          title={status}
        />

        <div className="flex-1 min-w-0">
          {/* Name + badges */}
          <div className="flex flex-wrap items-center gap-1.5 mb-1">
            <span className="font-medium text-sm">{info.name}</span>
            <Badge
              variant={typeBadgeVariant(info.type)}
              className="text-xs"
            >
              {info.type}
            </Badge>
            <Badge variant="outline" className="text-xs">
              {runtime.type}
            </Badge>
          </div>

          {/* Command */}
          <p className="text-xs font-mono text-muted-foreground truncate">
            {runtime.command}
          </p>
        </div>

        {/* Start/stop toggle — stop propagation so it doesn't select the card */}
        {/* biome-ignore lint/a11y/noStaticElementInteractions: wrapper purely stops propagation */}
        <div
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => e.stopPropagation()}
        >
          <Switch
            checked={isRunning}
            onCheckedChange={(v) => {
              if (v) startModule(info.id);
              else stopModule(info.id);
            }}
            aria-label={isRunning ? "Stop module" : "Start module"}
          />
        </div>
      </div>
    </button>
  );
}

// ─── ModuleList ───────────────────────────────────────────────────────────────

interface ModuleListProps {
  className?: string;
}

export function ModuleList({ className }: ModuleListProps) {
  const modules = useModuleStore((s) => s.modules);
  const loading = useModuleStore((s) => s.loading);
  const error = useModuleStore((s) => s.error);
  const selectedId = useModuleStore((s) => s.selectedId);
  const loadModules = useModuleStore((s) => s.loadModules);
  const selectModule = useModuleStore((s) => s.selectModule);
  const openScaffold = useModuleStore((s) => s.openScaffold);
  const scaffoldOpen = useModuleStore((s) => s.scaffoldOpen);

  useEffect(() => {
    loadModules();
  }, [loadModules]);

  const selectedEntry = modules.find((m) => m.manifest.module.id === selectedId);

  return (
    <div className={cn("flex flex-col gap-4", className)}>
      {/* Header */}
      <div className="flex items-center justify-between">
        <p className="text-sm font-semibold">Modules</p>
        <Button
          variant="outline"
          size="sm"
          onClick={openScaffold}
          disabled={scaffoldOpen}
        >
          + New Module
        </Button>
      </div>

      {/* Error */}
      {error && !scaffoldOpen && (
        <p className="text-xs text-destructive">{error}</p>
      )}

      {/* Loading */}
      {loading && (
        <p className="text-xs text-muted-foreground animate-pulse">
          Loading modules…
        </p>
      )}

      {/* Content */}
      {!loading && (
        <div className="flex gap-4">
          {/* Module card grid */}
          <div className="flex flex-col gap-2 w-64 shrink-0">
            {modules.length === 0 ? (
              <p className="text-sm text-muted-foreground py-8 text-center">
                No modules installed yet.
              </p>
            ) : (
              modules.map((entry) => (
                <ModuleCard
                  key={entry.manifest.module.id}
                  entry={entry}
                  isSelected={selectedId === entry.manifest.module.id}
                  onSelect={() =>
                    selectModule(
                      selectedId === entry.manifest.module.id
                        ? null
                        : entry.manifest.module.id,
                    )
                  }
                />
              ))
            )}
          </div>

          {/* Detail panel */}
          {selectedEntry && (
            <div className="flex-1 rounded-lg border bg-card p-4 overflow-y-auto">
              <div className="flex items-center justify-between mb-3">
                <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                  Module Details
                </p>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-xs"
                  onClick={() => selectModule(null)}
                >
                  ✕
                </Button>
              </div>
              <ModuleDetail entry={selectedEntry} />
            </div>
          )}
        </div>
      )}

      {/* Scaffold dialog */}
      <ModuleScaffold />
    </div>
  );
}
