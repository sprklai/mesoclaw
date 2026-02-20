/**
 * ActivityItem - Single row in the activity feed.
 *
 * Displays icon + title + status badge + relative time.
 */

import { Link } from "@tanstack/react-router";
import {
  AlertTriangle,
  Check,
  CircleSlash,
  Clock,
  Forward,
  Loader2,
  OctagonX,
  Pause,
} from "lucide-react";

import { cn } from "@/lib/utils";
import type { Activity, ActivityStatus } from "@/types/activity";

// ─── Status Configuration ───────────────────────────────────────────────────

interface StatusConfig {
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  label: string;
}

const STATUS_CONFIG: Record<ActivityStatus, StatusConfig> = {
  running: {
    icon: Loader2,
    color: "text-blue-500",
    label: "Running",
  },
  awaiting: {
    icon: Clock,
    color: "text-yellow-500",
    label: "Awaiting",
  },
  pending: {
    icon: Clock,
    color: "text-muted-foreground",
    label: "Pending",
  },
  paused: {
    icon: Pause,
    color: "text-muted-foreground",
    label: "Paused",
  },
  success: {
    icon: Check,
    color: "text-green-500",
    label: "Success",
  },
  error: {
    icon: OctagonX,
    color: "text-red-500",
    label: "Error",
  },
  cancelled: {
    icon: CircleSlash,
    color: "text-muted-foreground",
    label: "Cancelled",
  },
  terminated: {
    icon: OctagonX,
    color: "text-red-500",
    label: "Terminated",
  },
  stuck: {
    icon: AlertTriangle,
    color: "text-orange-500",
    label: "Stuck",
  },
  skipped: {
    icon: Forward,
    color: "text-muted-foreground",
    label: "Skipped",
  },
};

// ─── Helper Functions ───────────────────────────────────────────────────────

function formatRelativeTime(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);

  if (diffSec < 60) {
    return "just now";
  } else if (diffMin < 60) {
    return `${diffMin}m ago`;
  } else if (diffHour < 24) {
    return `${diffHour}h ago`;
  } else {
    return date.toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
    });
  }
}

function formatFutureTime(isoString: string): string {
  const date = new Date(isoString);
  const now = new Date();
  const diffMs = date.getTime() - now.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffMs < 0) {
    return "overdue";
  } else if (diffMin < 60) {
    return `in ${diffMin}m`;
  } else if (diffHour < 24) {
    return `in ${diffHour}h`;
  } else {
    return `in ${diffDay}d`;
  }
}

// ─── Component ──────────────────────────────────────────────────────────────

interface ActivityItemProps {
  activity: Activity;
  isFuture?: boolean;
}

export function ActivityItem({ activity, isFuture = false }: ActivityItemProps) {
  const config = STATUS_CONFIG[activity.status];
  const Icon = config.icon;
  const isAnimated = activity.status === "running";
  const timeText = isFuture
    ? formatFutureTime(activity.startedAt)
    : formatRelativeTime(activity.startedAt);

  const content = (
    <div
      className={cn(
        "flex items-center gap-3 py-2 px-3 rounded-lg",
        "hover:bg-muted/50 transition-colors",
        activity.linkTo && "cursor-pointer"
      )}
    >
      {/* Status Icon */}
      <div className={cn("flex-shrink-0", config.color)}>
        <Icon
          className={cn("size-4", isAnimated && "animate-spin")}
          aria-hidden
        />
      </div>

      {/* Title */}
      <span className="flex-1 text-sm truncate">{activity.title}</span>

      {/* Status Badge (for active states) */}
      {["running", "awaiting", "paused"].includes(activity.status) && (
        <span
          className={cn(
            "text-xs px-2 py-0.5 rounded-full",
            activity.status === "running" && "bg-blue-500/20 text-blue-600",
            activity.status === "awaiting" && "bg-yellow-500/20 text-yellow-600",
            activity.status === "paused" && "bg-muted text-muted-foreground"
          )}
        >
          {config.label}
        </span>
      )}

      {/* Time */}
      <span className="text-xs text-muted-foreground flex-shrink-0">
        {timeText}
      </span>
    </div>
  );

  // Wrap in Link if there's a navigation target
  if (activity.linkTo) {
    return (
      <Link to={activity.linkTo} className="block">
        {content}
      </Link>
    );
  }

  return content;
}

// ─── Planned Job Item ───────────────────────────────────────────────────────

interface PlannedJobItemProps {
  job: {
    id: string;
    name: string;
    nextRun: string;
    linkTo?: string;
  };
}

export function PlannedJobItem({ job }: PlannedJobItemProps) {
  const timeText = formatFutureTime(job.nextRun);

  const content = (
    <div
      className={cn(
        "flex items-center gap-3 py-2 px-3 rounded-lg",
        "hover:bg-muted/50 transition-colors",
        job.linkTo && "cursor-pointer"
      )}
    >
      {/* Calendar Icon */}
      <div className="flex-shrink-0 text-muted-foreground">
        <Clock className="size-4" aria-hidden />
      </div>

      {/* Name */}
      <span className="flex-1 text-sm truncate">{job.name}</span>

      {/* Time */}
      <span className="text-xs text-muted-foreground flex-shrink-0">
        {timeText}
      </span>
    </div>
  );

  if (job.linkTo) {
    return (
      <Link to={job.linkTo} className="block">
        {content}
      </Link>
    );
  }

  return content;
}
