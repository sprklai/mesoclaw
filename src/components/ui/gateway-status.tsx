/**
 * Gateway connection status indicator shown in the app topbar.
 *
 * Displays a small coloured dot + label reflecting the current state of the
 * MesoClaw daemon gateway connection tracked by `useGatewayStore`.
 */

import { cn } from "@/lib/utils";
import { useGatewayStore } from "@/stores/gatewayStore";

export function GatewayStatus() {
  const connected = useGatewayStore((s) => s.connected);
  const checking = useGatewayStore((s) => s.checking);

  const label = checking ? "Connecting…" : connected ? "Daemon" : "Offline";

  const dotClass = checking
    ? "bg-yellow-400 animate-pulse"
    : connected
      ? "bg-green-500"
      : "bg-muted-foreground/40";

  return (
    <div
      className="flex items-center gap-1.5 text-xs text-muted-foreground"
      title={
        checking
          ? "Checking gateway connection…"
          : connected
            ? "MesoClaw daemon is connected"
            : "MesoClaw daemon is not running"
      }
      aria-live="polite"
      aria-label={`Gateway: ${label}`}
    >
      <span className={cn("h-2 w-2 rounded-full", dotClass)} aria-hidden />
      <span className="hidden sm:inline">{label}</span>
    </div>
  );
}
