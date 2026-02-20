/**
 * ActivityFeed - Vertical list of activity items.
 *
 * Groups activities by section with a header.
 */

import { ActivityItem, PlannedJobItem } from "./ActivityItem";
import type { Activity } from "@/types/activity";
import { cn } from "@/lib/utils";

// ─── Section Header ─────────────────────────────────────────────────────────

interface SectionHeaderProps {
  title: string;
  count: number;
  dotColor?: string;
}

function SectionHeader({ title, count, dotColor = "bg-primary" }: SectionHeaderProps) {
  return (
    <div className="flex items-center gap-2 px-3 py-2 border-b border-border/50">
      <div className={cn("size-2 rounded-full", dotColor)} aria-hidden />
      <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
        {title}
      </span>
      <span className="text-xs font-medium text-muted-foreground">
        ({count})
      </span>
    </div>
  );
}

// ─── Activity Feed ───────────────────────────────────────────────────────────

interface ActivityFeedProps {
  activities: Activity[];
  maxItems?: number;
  emptyMessage?: string;
}

export function ActivityFeed({
  activities,
  maxItems = 10,
  emptyMessage = "No recent activity",
}: ActivityFeedProps) {
  const visible = activities.slice(0, maxItems);

  if (visible.length === 0) {
    return (
      <div className="py-6 text-center text-sm text-muted-foreground">
        {emptyMessage}
      </div>
    );
  }

  return (
    <div className="divide-y divide-border/30">
      {visible.map((activity) => (
        <ActivityItem key={activity.id} activity={activity} />
      ))}
    </div>
  );
}

// ─── Realtime Feed ───────────────────────────────────────────────────────────

interface RealtimeFeedProps {
  activities: Activity[];
  maxItems?: number;
}

export function RealtimeFeed({
  activities,
  maxItems = 5,
}: RealtimeFeedProps) {
  const visible = activities.slice(0, maxItems);
  // Only show green dot when there are active running tasks
  const dotColor = visible.length > 0 ? "bg-green-500" : "bg-muted-foreground";

  return (
    <div>
      <SectionHeader
        title="Now"
        count={visible.length}
        dotColor={dotColor}
      />
      {visible.length === 0 ? (
        <div className="py-4 text-center text-sm text-muted-foreground">
          No active tasks
        </div>
      ) : (
        <div className="divide-y divide-border/30">
          {visible.map((activity) => (
            <ActivityItem key={activity.id} activity={activity} />
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Planned Feed ────────────────────────────────────────────────────────────

interface PlannedFeedProps {
  jobs: Array<{
    id: string;
    name: string;
    nextRun: string;
    linkTo?: string;
  }>;
  maxItems?: number;
}

export function PlannedFeed({ jobs, maxItems = 5 }: PlannedFeedProps) {
  const visible = jobs.slice(0, maxItems);

  return (
    <div>
      <SectionHeader
        title="Scheduled"
        count={visible.length}
        dotColor="bg-blue-500"
      />
      {visible.length === 0 ? (
        <div className="py-4 text-center text-sm text-muted-foreground">
          No upcoming jobs
        </div>
      ) : (
        <div className="divide-y divide-border/30">
          {visible.map((job) => (
            <PlannedJobItem key={job.id} job={job} />
          ))}
        </div>
      )}
    </div>
  );
}

// ─── Recent Feed ─────────────────────────────────────────────────────────────

interface RecentFeedProps {
  activities: Activity[];
  maxItems?: number;
}

export function RecentFeed({ activities, maxItems = 10 }: RecentFeedProps) {
  const visible = activities.slice(0, maxItems);

  return (
    <div>
      <SectionHeader
        title="Recent"
        count={visible.length}
        dotColor="bg-muted-foreground"
      />
      {visible.length === 0 ? (
        <div className="py-4 text-center text-sm text-muted-foreground">
          No recent activity
        </div>
      ) : (
        <div className="divide-y divide-border/30">
          {visible.map((activity) => (
            <ActivityItem key={activity.id} activity={activity} />
          ))}
        </div>
      )}
    </div>
  );
}
