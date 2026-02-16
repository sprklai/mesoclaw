import type { ReactNode } from "react";

import { Button } from "@/components/ui/button";
import { AlertCircle, WifiOff, X } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface OfflineBannerProps {
  /** Whether the app is in offline mode */
  isOffline: boolean;
  /** Source of the data (cache or null for live) */
  source: "cache" | null;
  /** Staleness level of the cached data */
  staleness?: "fresh" | "stale" | "very-stale";
  /** Optional callback to retry connection */
  onRetry?: () => void;
  /** Optional callback to dismiss the banner */
  onDismiss?: () => void;
  /** Additional CSS classes */
  className?: string;
}

/**
 * Offline status banner component.
 * Displays when the app is in offline mode, showing what data is being viewed (cached vs live).
 *
 * **Behavior:**
 * - Only renders when `isOffline` is true
 * - Shows appropriate message based on staleness level
 * - Includes retry button if `onRetry` is provided
 * - Includes dismiss button if `onDismiss` is provided
 * - Uses amber/yellow color scheme (warning, not error)
 *
 * **Accessibility:**
 * - Uses semantic HTML with proper ARIA attributes
 * - `role="alert"` for important status announcements
 * - `aria-live="polite"` for non-critical updates
 * - Proper contrast ratios and keyboard navigation
 *
 * **Example Usage:**
 * ```tsx
 * <OfflineBanner
 *   isOffline={isOffline}
 *   source="cache"
 *   staleness="stale"
 *   onRetry={() => retryConnection()}
 *   onDismiss={() => dismissBanner()}
 * />
 * ```
 */
export function OfflineBanner({
  isOffline,
  source,
  staleness = "fresh",
  onRetry,
  onDismiss,
  className,
}: OfflineBannerProps) {
  // Don't render if online
  if (!isOffline) {
    return null;
  }

  const getMessageConfig = (): {
    icon: ReactNode;
    message: string;
    ariaLabel: string;
  } => {
    switch (staleness) {
      case "very-stale":
        return {
          icon: (
            <AlertCircle
              className="h-4 w-4 text-amber-600 dark:text-amber-400"
              aria-hidden="true"
            />
          ),
          message: "Offline Mode - Showing cached data (may be outdated)",
          ariaLabel:
            "Offline mode - displaying cached data that may be significantly outdated",
        };
      case "stale":
        return {
          icon: (
            <WifiOff
              className="h-4 w-4 text-amber-600 dark:text-amber-400"
              aria-hidden="true"
            />
          ),
          message: "Offline Mode - Showing cached data",
          ariaLabel:
            "Offline mode - displaying cached data that may be slightly outdated",
        };
      case "fresh":
      default:
        return {
          icon: (
            <WifiOff
              className="h-4 w-4 text-amber-600 dark:text-amber-400"
              aria-hidden="true"
            />
          ),
          message: "Offline Mode - Showing recent cached data",
          ariaLabel: "Offline mode - displaying recently cached data",
        };
    }
  };

  const config = getMessageConfig();

  return (
    <div
      className={cn(
        "bg-amber-50 dark:bg-amber-900/20 border-b border-amber-200 dark:border-amber-800",
        className
      )}
      role="alert"
      aria-label={config.ariaLabel}
      aria-live="polite"
    >
      <div className="flex items-center justify-between gap-2 px-4 py-2">
        <div className="flex items-center gap-2 text-sm">
          {config.icon}
          <span className="text-amber-900 dark:text-amber-100 font-medium">
            {config.message}
          </span>
          {source === "cache" && (
            <span
              className="text-xs text-amber-700 dark:text-amber-300"
              aria-hidden="true"
            >
              (from cache)
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {onRetry && (
            <Button
              size="sm"
              variant="outline"
              onClick={onRetry}
              className="h-7 px-2 text-xs"
              aria-label="Retry connection to database"
            >
              Retry Connection
            </Button>
          )}
          {onDismiss && (
            <Button
              size="icon"
              variant="ghost"
              onClick={onDismiss}
              className="h-7 w-7"
              aria-label="Dismiss offline notification"
            >
              <X className="h-4 w-4" aria-hidden="true" />
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
