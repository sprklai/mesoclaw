/**
 * DailyTimeline — calendar-style list of daily memory files.
 *
 * Lists available dates (most recent first). Clicking a date fetches and
 * displays that day's raw markdown content in a scrollable panel.
 */

import { useEffect } from "react";

import { cn } from "@/lib/utils";
import { useMemoryStore } from "@/stores/memoryStore";

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Format "YYYY-MM-DD" → "Mon, Jan 5 2026" */
function formatDate(dateStr: string): string {
  try {
    const d = new Date(`${dateStr}T00:00:00`);
    return d.toLocaleDateString(undefined, {
      weekday: "short",
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return dateStr;
  }
}

/** Return "today" | "yesterday" | "" */
function relativeLabel(dateStr: string): string {
  const today = new Date().toISOString().slice(0, 10);
  const yesterday = new Date(Date.now() - 86_400_000).toISOString().slice(0, 10);
  if (dateStr === today) return "today";
  if (dateStr === yesterday) return "yesterday";
  return "";
}

// ─── DailyTimeline ────────────────────────────────────────────────────────────

interface DailyTimelineProps {
  className?: string;
}

export function DailyTimeline({ className }: DailyTimelineProps) {
  const availableDates = useMemoryStore((s) => s.availableDates);
  const datesLoading = useMemoryStore((s) => s.datesLoading);
  const selectedDate = useMemoryStore((s) => s.selectedDate);
  const dailyContent = useMemoryStore((s) => s.dailyContent);
  const dailyLoading = useMemoryStore((s) => s.dailyLoading);
  const loadDates = useMemoryStore((s) => s.loadDates);
  const selectDate = useMemoryStore((s) => s.selectDate);

  useEffect(() => {
    loadDates();
  }, [loadDates]);

  return (
    <div className={cn("flex flex-col gap-3", className)}>
      {/* Date list */}
      {datesLoading && (
        <p className="text-xs text-muted-foreground animate-pulse">Loading dates…</p>
      )}

      {!datesLoading && availableDates.length === 0 && (
        <p className="text-center text-sm text-muted-foreground py-6">
          No daily memory files yet.
        </p>
      )}

      {availableDates.length > 0 && (
        <ul className="flex flex-col gap-0.5" role="list">
          {availableDates.map((date) => {
            const rel = relativeLabel(date);
            const isSelected = selectedDate === date;
            return (
              <li key={date}>
                <button
                  type="button"
                  onClick={() => selectDate(date)}
                  className={cn(
                    "w-full flex items-center gap-3 rounded-md px-3 py-2 text-sm text-left hover:bg-accent transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                    isSelected && "bg-accent border border-primary"
                  )}
                  aria-pressed={isSelected}
                >
                  {/* Calendar dot */}
                  <span
                    className={cn(
                      "h-2 w-2 shrink-0 rounded-full",
                      isSelected ? "bg-primary" : "bg-muted-foreground/40"
                    )}
                  />

                  <span className="flex-1 font-medium">{formatDate(date)}</span>

                  {rel && (
                    <span className="shrink-0 text-xs text-muted-foreground italic">
                      {rel}
                    </span>
                  )}
                </button>

                {/* Inline content panel for selected date */}
                {isSelected && (
                  <div className="mx-3 mb-2 rounded-md border bg-muted/30 p-3">
                    {dailyLoading ? (
                      <p className="text-xs text-muted-foreground animate-pulse">
                        Loading…
                      </p>
                    ) : dailyContent ? (
                      <pre className="max-h-60 overflow-y-auto whitespace-pre-wrap break-words text-[11px] text-foreground/90">
                        {dailyContent}
                      </pre>
                    ) : (
                      <p className="text-xs text-muted-foreground">
                        No content for this date.
                      </p>
                    )}
                  </div>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
