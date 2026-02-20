/**
 * ActivityDashboard - Container for activity sections.
 *
 * Displays three sections: NOW (realtime), SCHEDULED (planned), RECENT (completed).
 */

import { useEffect } from "react";
import { Link } from "@tanstack/react-router";
import { ArrowRight } from "lucide-react";

import { RealtimeFeed, PlannedFeed, RecentFeed } from "./ActivityFeed";
import { useActivityStore } from "@/stores/activityStore";
import { cn } from "@/lib/utils";

// ─── Dashboard Component ─────────────────────────────────────────────────────

export function ActivityDashboard() {
  const {
    realtime,
    planned,
    recent,
    loading,
    error,
    startAutoRefresh,
    stopAutoRefresh,
  } = useActivityStore();

  // Start auto-refresh on mount, stop on unmount
  useEffect(() => {
    startAutoRefresh();
    return () => stopAutoRefresh();
  }, [startAutoRefresh, stopAutoRefresh]);

  // Calculate total activity count
  const totalCount = realtime.length + planned.length + recent.length;

  return (
    <section aria-labelledby="activity-heading">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <h2
          id="activity-heading"
          className="text-xs font-semibold uppercase tracking-wider text-muted-foreground"
        >
          Activity
        </h2>
        <Link
          to="/logs"
          className={cn(
            "flex items-center gap-1 text-xs text-muted-foreground",
            "hover:text-foreground transition-colors",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          )}
        >
          View All
          <ArrowRight className="size-3" aria-hidden />
        </Link>
      </div>

      {/* Content */}
      <div className="rounded-xl border border-border bg-card overflow-hidden">
        {/* Error State */}
        {error && (
          <div className="p-4 text-sm text-red-500 bg-red-500/10">
            Failed to load activities: {error}
          </div>
        )}

        {/* Loading State (only on initial load) */}
        {loading && totalCount === 0 && (
          <div className="p-6 text-center text-sm text-muted-foreground">
            Loading activities...
          </div>
        )}

        {/* Activity Sections */}
        {!error && totalCount > 0 && (
          <div className="divide-y divide-border">
            {/* Realtime Section */}
            {realtime.length > 0 && <RealtimeFeed activities={realtime} />}

            {/* Planned Section */}
            {planned.length > 0 && <PlannedFeed jobs={planned} />}

            {/* Recent Section */}
            <RecentFeed activities={recent} />
          </div>
        )}

        {/* Empty State */}
        {!error && !loading && totalCount === 0 && (
          <div className="py-8 text-center">
            <p className="text-sm text-muted-foreground">
              No activity yet. Actions will appear here as they happen.
            </p>
          </div>
        )}
      </div>
    </section>
  );
}

// ─── Compact Dashboard Variant ───────────────────────────────────────────────

interface CompactDashboardProps {
  maxItems?: number;
  showScheduled?: boolean;
}

/**
 * A more compact version of the dashboard for sidebar or smaller spaces.
 */
export function CompactDashboard({
  maxItems = 3,
  showScheduled = false,
}: CompactDashboardProps) {
  const { realtime, planned, recent, startAutoRefresh, stopAutoRefresh } =
    useActivityStore();

  useEffect(() => {
    startAutoRefresh();
    return () => stopAutoRefresh();
  }, [startAutoRefresh, stopAutoRefresh]);

  const hasContent =
    realtime.length > 0 ||
    (showScheduled && planned.length > 0) ||
    recent.length > 0;

  if (!hasContent) {
    return null;
  }

  return (
    <div className="rounded-lg border border-border bg-card overflow-hidden">
      {realtime.length > 0 && (
        <RealtimeFeed activities={realtime} maxItems={maxItems} />
      )}
      {showScheduled && planned.length > 0 && (
        <PlannedFeed jobs={planned} maxItems={maxItems} />
      )}
      {recent.length > 0 && <RecentFeed activities={recent} maxItems={maxItems} />}
    </div>
  );
}
