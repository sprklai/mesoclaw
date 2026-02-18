/**
 * ModuleDetail — expanded view for a selected module.
 *
 * Shows manifest details (info, runtime, security) and health status.
 */

import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { useModuleStore } from "@/stores/moduleStore";
import type { ModuleEntry, ModuleStatus } from "@/stores/moduleStore";

// ─── Helpers ──────────────────────────────────────────────────────────────────

function statusColor(status: ModuleStatus): string {
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

function Row({ label, value }: { label: string; value: React.ReactNode }) {
  return (
    <div className="flex items-start gap-2 py-1 border-b last:border-0">
      <span className="text-xs text-muted-foreground w-32 shrink-0">{label}</span>
      <span className="text-xs font-mono break-all">{value}</span>
    </div>
  );
}

// ─── ModuleDetail ─────────────────────────────────────────────────────────────

interface ModuleDetailProps {
  entry: ModuleEntry;
  className?: string;
}

export function ModuleDetail({ entry, className }: ModuleDetailProps) {
  const startModule = useModuleStore((s) => s.startModule);
  const stopModule = useModuleStore((s) => s.stopModule);

  const { manifest, status, healthy, errorMessage } = entry;
  const { module: info, runtime, security } = manifest;

  const canStart = status === "stopped" || status === "error";
  const canStop = status === "running" || status === "starting";

  return (
    <div className={cn("flex flex-col gap-4", className)}>
      {/* Header */}
      <div className="flex items-center gap-3">
        <span
          className={cn("h-2.5 w-2.5 rounded-full shrink-0", statusColor(status))}
          title={status}
        />
        <div className="flex-1 min-w-0">
          <p className="font-semibold text-sm leading-tight">{info.name}</p>
          <p className="text-xs text-muted-foreground font-mono">{info.id} v{info.version}</p>
        </div>
        {canStart && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => startModule(info.id)}
          >
            Start
          </Button>
        )}
        {canStop && (
          <Button
            variant="outline"
            size="sm"
            className="text-destructive hover:text-destructive"
            onClick={() => stopModule(info.id)}
          >
            Stop
          </Button>
        )}
      </div>

      {/* Description */}
      {info.description && (
        <p className="text-xs text-muted-foreground">{info.description}</p>
      )}

      {/* Error message */}
      {errorMessage && (
        <p className="text-xs text-destructive">{errorMessage}</p>
      )}

      {/* Health */}
      <div className="rounded-md border bg-muted/20 p-3">
        <p className="text-xs font-semibold mb-2 uppercase tracking-wide text-muted-foreground">
          Health
        </p>
        <div className="flex items-center gap-2">
          {healthy === null ? (
            <span className="text-xs text-muted-foreground">Unknown</span>
          ) : healthy ? (
            <Badge variant="secondary" className="text-[10px] bg-green-100 text-green-700">
              Healthy
            </Badge>
          ) : (
            <Badge variant="destructive" className="text-[10px]">
              Unhealthy
            </Badge>
          )}
        </div>
      </div>

      {/* Module info */}
      <div className="rounded-md border bg-muted/20 p-3">
        <p className="text-xs font-semibold mb-2 uppercase tracking-wide text-muted-foreground">
          Module
        </p>
        <Row label="Type" value={<Badge variant="outline" className="text-[10px]">{info.type}</Badge>} />
        <Row label="Version" value={info.version} />
      </div>

      {/* Runtime config */}
      <div className="rounded-md border bg-muted/20 p-3">
        <p className="text-xs font-semibold mb-2 uppercase tracking-wide text-muted-foreground">
          Runtime
        </p>
        <Row label="Runtime" value={<Badge variant="outline" className="text-[10px]">{runtime.type}</Badge>} />
        <Row label="Command" value={runtime.command} />
        {runtime.args.length > 0 && (
          <Row label="Args" value={runtime.args.join(" ")} />
        )}
        {runtime.timeout_secs !== null && (
          <Row label="Timeout" value={`${runtime.timeout_secs}s`} />
        )}
        {Object.keys(runtime.env ?? {}).length > 0 && (
          <Row
            label="Env vars"
            value={Object.keys(runtime.env).join(", ")}
          />
        )}
      </div>

      {/* Security config */}
      <div className="rounded-md border bg-muted/20 p-3">
        <p className="text-xs font-semibold mb-2 uppercase tracking-wide text-muted-foreground">
          Security
        </p>
        <Row label="Network" value={security.allow_network ? "Allowed" : "Blocked"} />
        <Row label="Filesystem" value={security.allow_filesystem ? "Allowed" : "Blocked"} />
        <Row label="Max memory" value={`${security.max_memory_mb} MB`} />
      </div>
    </div>
  );
}
