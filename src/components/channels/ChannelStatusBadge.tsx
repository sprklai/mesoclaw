/**
 * Channel Status Badge Component
 *
 * Displays a visual indicator for channel connection status
 * with appropriate colors and animations.
 */
import { cn } from "@/lib/utils";
import type { ChannelStatus } from "@/stores/channelStore";

const statusConfig: Record<
  ChannelStatus,
  {
    color: string;
    pulse: boolean;
    label: string;
  }
> = {
  connected: {
    color: "bg-green-500",
    pulse: false,
    label: "Connected",
  },
  disconnected: {
    color: "bg-gray-400",
    pulse: false,
    label: "Disconnected",
  },
  reconnecting: {
    color: "bg-yellow-500",
    pulse: true,
    label: "Reconnecting...",
  },
  error: {
    color: "bg-red-500",
    pulse: true,
    label: "Error",
  },
};

interface ChannelStatusBadgeProps {
  status: ChannelStatus;
  showLabel?: boolean;
  className?: string;
}

export function ChannelStatusBadge({
  status,
  showLabel = true,
  className,
}: ChannelStatusBadgeProps) {
  const config = statusConfig[status];

  return (
    <div className={cn("flex items-center gap-1.5", className)}>
      <span
        className={cn(
          "size-2 rounded-full",
          config.color,
          config.pulse && "animate-pulse"
        )}
        aria-hidden="true"
      />
      {showLabel && (
        <span className="text-xs text-muted-foreground">{config.label}</span>
      )}
    </div>
  );
}

/**
 * Compact status dot for use in lists.
 */
export function ChannelStatusDot({
  status,
  className,
}: {
  status: ChannelStatus;
  className?: string;
}) {
  const config = statusConfig[status];

  return (
    <span
      className={cn(
        "size-2 shrink-0 rounded-full",
        config.color,
        config.pulse && "animate-pulse",
        className
      )}
      title={config.label}
      aria-label={config.label}
    />
  );
}
