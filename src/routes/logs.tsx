import { invoke } from "@tauri-apps/api/core";
import { createFileRoute } from "@tanstack/react-router";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { PageHeader } from "@/components/layout/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ArrowDown, Pause, Play, RefreshCw, Search, Trash2, X } from "@/lib/icons";
import { cn } from "@/lib/utils";

const AUTO_REFRESH_INTERVAL_MS = 2000;

export const Route = createFileRoute("/logs")({
  component: LogsPage,
});

// ── Types ──────────────────────────────────────────────────────────────────────

interface LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

type LogLevel = "TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR" | "ALL";

const LEVELS: LogLevel[] = ["ALL", "TRACE", "DEBUG", "INFO", "WARN", "ERROR"];

// ── Helpers ────────────────────────────────────────────────────────────────────

function levelColor(level: string): string {
  switch (level.toUpperCase()) {
    case "ERROR":
      return "text-destructive";
    case "WARN":
      return "text-yellow-500 dark:text-yellow-400";
    case "INFO":
      return "text-blue-500 dark:text-blue-400";
    case "DEBUG":
      return "text-green-600 dark:text-green-400";
    case "TRACE":
      return "text-muted-foreground";
    default:
      return "text-foreground";
  }
}

function levelBadgeVariant(
  level: string,
): "default" | "destructive" | "secondary" | "outline" {
  switch (level.toUpperCase()) {
    case "ERROR":
      return "destructive";
    case "WARN":
      return "outline";
    default:
      return "secondary";
  }
}

// ── Main component ─────────────────────────────────────────────────────────────

function LogsPage() {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [activeLevel, setActiveLevel] = useState<LogLevel>("ALL");
  const [search, setSearch] = useState("");
  const [autoRefresh, setAutoRefresh] = useState(true);

  const bottomRef = useRef<HTMLDivElement>(null);
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const isAtBottomRef = useRef(true);
  const [showScrollBtn, setShowScrollBtn] = useState(false);

  const scrollToBottom = useCallback((behavior: ScrollBehavior = "smooth") => {
    bottomRef.current?.scrollIntoView({ behavior });
  }, []);

  const handleScroll = useCallback(() => {
    const el = scrollContainerRef.current;
    if (!el) return;
    const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
    isAtBottomRef.current = atBottom;
    setShowScrollBtn(!atBottom);
  }, []);

  const fetchLogs = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await invoke<LogEntry[]>("get_logs_command", {
        maxLines: 5000,
      });
      setEntries(data.slice().reverse());
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  // Initial load — scroll to bottom immediately
  useEffect(() => {
    void fetchLogs().then(() => scrollToBottom("instant"));
  }, [fetchLogs, scrollToBottom]);

  // Auto-refresh polling
  useEffect(() => {
    if (!autoRefresh) return;
    const id = setInterval(() => void fetchLogs(), AUTO_REFRESH_INTERVAL_MS);
    return () => clearInterval(id);
  }, [autoRefresh, fetchLogs]);

  // ── Filtering ────────────────────────────────────────────────────────────────

  const filtered = useMemo(() => {
    return entries.filter((e) => {
      if (activeLevel !== "ALL" && e.level.toUpperCase() !== activeLevel)
        return false;
      if (search.trim()) {
        const q = search.toLowerCase();
        return (
          e.message.toLowerCase().includes(q) ||
          e.target.toLowerCase().includes(q) ||
          e.timestamp.toLowerCase().includes(q)
        );
      }
      return true;
    });
  }, [entries, activeLevel, search]);

  // Auto-scroll to bottom when filtered list changes — only if already at bottom
  useEffect(() => {
    if (isAtBottomRef.current) {
      scrollToBottom("smooth");
    }
  }, [filtered, scrollToBottom]);

  // ── Level counts ─────────────────────────────────────────────────────────────

  const counts = useMemo(() => {
    const map: Record<string, number> = {};
    for (const e of entries) {
      const l = e.level.toUpperCase();
      map[l] = (map[l] ?? 0) + 1;
    }
    return map;
  }, [entries]);

  return (
    <div className="flex h-full flex-col gap-4">
      <PageHeader
        title="Logs"
        description="Application log viewer — recent entries from the current session"
      />

      {/* ── Toolbar ─────────────────────────────────────────────────────────── */}
      <div className="flex flex-wrap items-center gap-2">
        {/* Level filter buttons */}
        <div className="flex flex-wrap gap-1" role="group" aria-label="Filter by log level">
          {LEVELS.map((lvl) => (
            <button
              key={lvl}
              type="button"
              onClick={() => setActiveLevel(lvl)}
              className={cn(
                "rounded-md px-3 py-1 text-xs font-medium transition-colors",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                activeLevel === lvl
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted text-muted-foreground hover:bg-accent hover:text-accent-foreground",
              )}
              aria-pressed={activeLevel === lvl}
            >
              {lvl}
              {lvl !== "ALL" && counts[lvl] != null && (
                <span className="ml-1.5 opacity-70">
                  {counts[lvl]}
                </span>
              )}
            </button>
          ))}
        </div>

        {/* Search */}
        <div className="relative ml-auto min-w-[180px] max-w-xs flex-1">
          <Search
            aria-hidden
            className="absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            className="h-8 pl-8 pr-8 text-xs"
            placeholder="Search messages…"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          {search && (
            <button
              type="button"
              onClick={() => setSearch("")}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              aria-label="Clear search"
            >
              <X className="size-3.5" />
            </button>
          )}
        </div>

        {/* Clear */}
        <Button
          variant="ghost"
          size="icon"
          className="size-8"
          onClick={() => setEntries([])}
          disabled={entries.length === 0}
          aria-label="Clear logs"
          title="Clear logs"
        >
          <Trash2 className="size-4" aria-hidden />
        </Button>

        {/* Auto-refresh toggle */}
        <Button
          variant={autoRefresh ? "default" : "ghost"}
          size="icon"
          className="size-8"
          onClick={() => setAutoRefresh((v) => !v)}
          aria-label={autoRefresh ? "Pause auto-refresh" : "Resume auto-refresh"}
          title={autoRefresh ? "Pause auto-refresh" : "Resume auto-refresh"}
        >
          {autoRefresh ? (
            <Pause className="size-4" aria-hidden />
          ) : (
            <Play className="size-4" aria-hidden />
          )}
        </Button>

        {/* Manual refresh */}
        <Button
          variant="ghost"
          size="icon"
          className="size-8"
          onClick={() => void fetchLogs()}
          disabled={loading}
          aria-label="Refresh logs"
        >
          <RefreshCw
            className={cn("size-4", loading && "animate-spin")}
            aria-hidden
          />
        </Button>
      </div>

      {/* ── Log table ────────────────────────────────────────────────────────── */}
      {error ? (
        <div className="rounded-lg border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">
          {error}
        </div>
      ) : (
        <div className="relative flex-1 overflow-hidden rounded-xl border border-border">
          {showScrollBtn && (
            <button
              type="button"
              onClick={() => scrollToBottom("smooth")}
              aria-label="Scroll to latest logs"
              className={cn(
                "absolute bottom-3 right-3 z-20",
                "flex size-8 items-center justify-center rounded-full",
                "bg-background/70 backdrop-blur-sm",
                "border border-border shadow-md",
                "text-muted-foreground transition-opacity hover:text-foreground",
                "hover:bg-background/90",
              )}
            >
              <ArrowDown className="size-4" aria-hidden />
            </button>
          )}
          <div
            ref={scrollContainerRef}
            onScroll={handleScroll}
            className="h-full overflow-y-auto font-mono text-xs"
          >
            {filtered.length === 0 ? (
              <p className="p-6 text-center text-sm text-muted-foreground">
                {loading ? "Loading…" : "No log entries match the current filter."}
              </p>
            ) : (
              <table className="w-full border-collapse">
                <thead className="sticky top-0 z-10 bg-background/95 backdrop-blur-sm">
                  <tr className="border-b border-border">
                    <th className="px-3 py-2 text-left text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                      Time
                    </th>
                    <th className="px-3 py-2 text-left text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                      Level
                    </th>
                    <th className="hidden px-3 py-2 text-left text-[10px] font-semibold uppercase tracking-wider text-muted-foreground sm:table-cell">
                      Target
                    </th>
                    <th className="px-3 py-2 text-left text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                      Message
                    </th>
                  </tr>
                </thead>
                <tbody>
                  {filtered.map((entry, i) => (
                    <tr
                      // biome-ignore lint/suspicious/noArrayIndexKey: log entries have no stable id
                      key={i}
                      className={cn(
                        "border-b border-border/50 transition-colors hover:bg-muted/30",
                        entry.level.toUpperCase() === "ERROR" &&
                          "bg-destructive/5",
                        entry.level.toUpperCase() === "WARN" &&
                          "bg-yellow-500/5",
                      )}
                    >
                      {/* Timestamp */}
                      <td className="whitespace-nowrap px-3 py-1.5 text-muted-foreground">
                        {formatTimestamp(entry.timestamp)}
                      </td>

                      {/* Level badge */}
                      <td className="whitespace-nowrap px-3 py-1.5">
                        <Badge
                          variant={levelBadgeVariant(entry.level)}
                          className={cn(
                            "text-[10px]",
                            levelColor(entry.level),
                          )}
                        >
                          {entry.level}
                        </Badge>
                      </td>

                      {/* Target */}
                      <td className="hidden max-w-[180px] truncate px-3 py-1.5 text-muted-foreground sm:table-cell">
                        {entry.target}
                      </td>

                      {/* Message */}
                      <td className="break-all px-3 py-1.5">
                        <span className={levelColor(entry.level)}>
                          {entry.message}
                        </span>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
            <div ref={bottomRef} />
          </div>
        </div>
      )}

      {/* ── Footer stats ────────────────────────────────────────────────────── */}
      <p className="text-right text-xs text-muted-foreground">
        {filtered.length} / {entries.length} entries
        {autoRefresh && (
          <span className="ml-2 opacity-60">· live</span>
        )}
      </p>
    </div>
  );
}

// ── Utilities ──────────────────────────────────────────────────────────────────

/** Shorten an ISO timestamp to just the time part for compact display. */
function formatTimestamp(ts: string): string {
  // e.g. "2025-01-01T12:34:56.789012Z" → "12:34:56.789"
  const t = ts.split("T")[1];
  if (!t) return ts;
  return t.replace("Z", "").split(".").map((p, i) => (i === 1 ? p.slice(0, 3) : p)).join(".");
}
