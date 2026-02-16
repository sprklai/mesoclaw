import { useEffect, useRef } from "react";

import { cn } from "@/lib/utils";

interface LiveRegionProps {
  message?: string;
  role?: "status" | "alert";
  className?: string;
}

/**
 * LiveRegion - Announces messages to screen readers without visual output
 *
 * Use for:
 * - Non-visual feedback (copy success, status updates)
 * - Toast notifications
 * - Form validation feedback
 * - Async operation completion
 *
 * @example
 * const [announce, setAnnounce] = useState("");
 *
 * <LiveRegion message={announce} role="status" />
 *
 * // Later:
 * setAnnounce("Changes saved successfully");
 */
export function LiveRegion({
  message,
  role = "status",
  className,
}: LiveRegionProps) {
  const announcementRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (message && announcementRef.current) {
      // Clear the content first to re-trigger announcement
      announcementRef.current.textContent = "";
      // Use setTimeout to ensure screen readers pick up the change
      const timeoutId = setTimeout(() => {
        if (announcementRef.current) {
          announcementRef.current.textContent = message;
        }
      }, 100);

      return () => clearTimeout(timeoutId);
    }
  }, [message]);

  return (
    <div
      ref={announcementRef}
      role={role}
      aria-live={role === "alert" ? "assertive" : "polite"}
      aria-atomic="true"
      className={cn("sr-only", className)}
    />
  );
}
