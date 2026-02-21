/**
 * Gateway connection status indicator shown in the app topbar.
 *
 * Displays a small coloured dot + label reflecting the current state of the
 * daemon gateway connection tracked by `useGatewayStore`.
 */

import { useEffect } from "react";
import { cn } from "@/lib/utils";
import { getAppDisplayName } from "@/stores/appSettingsStore";
import { useGatewayStore } from "@/stores/gatewayStore";

export function GatewayStatus() {
  const connected = useGatewayStore((s) => s.connected);
  const checking = useGatewayStore((s) => s.checking);
  const checkConnection = useGatewayStore((s) => s.checkConnection);

  useEffect(() => {
    checkConnection();
    const interval = setInterval(checkConnection, 10_000);
    return () => clearInterval(interval);
  }, [checkConnection]);

  const label = checking ? "Connecting…" : connected ? "Daemon" : "Offline";

  const dotClass = checking
    ? "bg-yellow-400 animate-pulse"
    : connected
      ? "bg-green-500"
      : "bg-muted-foreground/40";

  const appName = getAppDisplayName();

  return (
    <div
      className="flex items-center gap-1.5 text-xs text-muted-foreground"
      title={
        checking
          ? "Checking gateway connection…"
          : connected
            ? `${appName} daemon is connected`
            : `${appName} daemon is not running`
      }
      aria-live="polite"
      aria-label={`Gateway: ${label}`}
    >
      <span className={cn("h-2 w-2 rounded-full", dotClass)} aria-hidden />
      <span className="hidden sm:inline">{label}</span>
    </div>
  );
}
