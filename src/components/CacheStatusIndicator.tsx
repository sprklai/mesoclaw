import type { ComponentProps, ReactNode } from "react";

import { Badge } from "@/components/ui/badge";
import { Tooltip } from "@/components/ui/tooltip";
import { CheckCircle2, Clock, AlertTriangle } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface CacheStatusIndicatorProps {
  /** Staleness level of the cached data */
  staleness: "fresh" | "stale" | "very-stale" | null;
  /** Source of the data (backend or cache) */
  source: "backend" | "cache" | null;
  /** Additional CSS classes */
  className?: string;
}

/**
 * Visual indicator showing cache freshness status.
 * Displays color-coded badges with icons to indicate whether
 * data is live (fresh), cached and updating (stale), or cached
 * and potentially outdated (very-stale).
 *
 * **Visual Indicators:**
 * - Fresh: Green badge with check icon, "Live" label
 * - Stale: Yellow badge with clock icon, "Cached (updating...)" label
 * - Very stale: Orange badge with warning icon, "Cached (may be outdated)" label
 * - Null: Component doesn't render (no data)
 *
 * **Accessibility:**
 * - Uses semantic HTML with proper ARIA attributes
 * - Includes screen reader announcements via aria-label
 * - Keyboard accessible via tooltip interaction
 */
export function CacheStatusIndicator({
  staleness,
  source,
  className,
}: CacheStatusIndicatorProps) {
  // Don't render if no staleness data
  if (staleness === null) {
    return null;
  }

  const getStatusConfig = (): {
    variant: ComponentProps<typeof Badge>["variant"];
    icon: ReactNode;
    label: string;
    description: string;
    ariaLabel: string;
  } => {
    switch (staleness) {
      case "fresh":
        return {
          variant: "success",
          icon: <CheckCircle2 className="h-3 w-3" aria-hidden="true" />,
          label: "Live",
          description:
            source === "cache"
              ? "Data is fresh from cache (recently updated)"
              : "Data is live from the database",
          ariaLabel: "Live data - Recently updated",
        };
      case "stale":
        return {
          variant: "warning",
          icon: <Clock className="h-3 w-3 animate-spin" aria-hidden="true" />,
          label: "Cached (updating...)",
          description:
            "Data is from cache and may be slightly outdated. Update in progress...",
          ariaLabel: "Stale data - Updating in background",
        };
      case "very-stale":
        return {
          variant: "warning",
          icon: <AlertTriangle className="h-3 w-3" aria-hidden="true" />,
          label: "Cached (may be outdated)",
          description:
            "Data is from cache and may be significantly outdated. Consider refreshing.",
          ariaLabel: "Very stale data - May be outdated",
        };
    }
  };

  const config = getStatusConfig();

  const tooltipContent = (
    <div className="space-y-1">
      <p className="font-medium">{config.ariaLabel}</p>
      <p className="text-xs opacity-80">{config.description}</p>
      {source === "cache" && (
        <p className="text-xs opacity-60">Source: Cache (not from database)</p>
      )}
    </div>
  );

  return (
    <Tooltip content={tooltipContent} delayDuration={200}>
      <div
        className={cn("flex items-center gap-2 text-xs", className)}
        role="status"
        aria-label={config.ariaLabel}
        aria-live="polite"
      >
        <Badge variant={config.variant} className="gap-1">
          {config.icon}
          <span>{config.label}</span>
        </Badge>
        {source === "cache" && (
          <span className="text-xs text-muted-foreground" aria-hidden="true">
            (from cache)
          </span>
        )}
      </div>
    </Tooltip>
  );
}
