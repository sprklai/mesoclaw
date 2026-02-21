/**
 * MemorySearch — debounced semantic search over the agent's memory store.
 *
 * Shows: key, category badge, score bar, content preview (2 lines).
 * Click a result to expand the full content inline.
 */

import { useEffect, useRef } from "react";

import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { useMemoryStore } from "@/stores/memoryStore";

import type { MemoryEntry } from "@/stores/memoryStore";

// ─── Helpers ──────────────────────────────────────────────────────────────────

const DEBOUNCE_MS = 350;

const CATEGORY_VARIANT: Record<
  string,
  "default" | "secondary" | "outline" | "success"
> = {
  core: "default",
  daily: "success",
  conversation: "secondary",
};

function categoryVariant(cat: string) {
  return CATEGORY_VARIANT[cat] ?? "outline";
}

/** Render score as a thin coloured bar 0–100%. */
function ScoreBar({ score }: { score: number }) {
  const pct = Math.round(Math.min(1, Math.max(0, score)) * 100);
  return (
    <div
      className="h-1 w-full overflow-hidden rounded-full bg-muted"
      title={`Relevance: ${pct}%`}
    >
      <div
        className="h-full bg-primary transition-all"
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}

// ─── ResultRow ────────────────────────────────────────────────────────────────

function ResultRow({
  entry,
  selected,
  onSelect,
}: {
  entry: MemoryEntry;
  selected: boolean;
  onSelect: (e: MemoryEntry) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onSelect(selected ? null! : entry)}
      className={cn(
        "w-full rounded-md border px-3 py-2 text-left text-sm transition-colors hover:bg-accent focus:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        selected && "border-primary bg-accent"
      )}
    >
      {/* Header */}
      <div className="flex items-center gap-2">
        <span className="flex-1 truncate font-mono text-xs font-semibold">
          {entry.key}
        </span>
        <Badge variant={categoryVariant(entry.category)} className="shrink-0 text-xs">
          {entry.category}
        </Badge>
        <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
          {Math.round(entry.score * 100)}%
        </span>
      </div>

      <ScoreBar score={entry.score} />

      {/* Preview or full content */}
      {selected ? (
        <pre className="mt-2 max-h-60 overflow-y-auto whitespace-pre-wrap break-words text-[11px] text-foreground/90">
          {entry.content}
        </pre>
      ) : (
        <p className="mt-1 line-clamp-2 text-xs text-muted-foreground">
          {entry.content}
        </p>
      )}
    </button>
  );
}

// ─── MemorySearch ─────────────────────────────────────────────────────────────

interface MemorySearchProps {
  className?: string;
}

export function MemorySearch({ className }: MemorySearchProps) {
  const query = useMemoryStore((s) => s.query);
  const results = useMemoryStore((s) => s.results);
  const searching = useMemoryStore((s) => s.searching);
  const searchError = useMemoryStore((s) => s.searchError);
  const selectedEntry = useMemoryStore((s) => s.selectedEntry);
  const setQuery = useMemoryStore((s) => s.setQuery);
  const search = useMemoryStore((s) => s.search);
  const selectEntry = useMemoryStore((s) => s.selectEntry);

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setQuery(val);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => search(val), DEBOUNCE_MS);
  };

  useEffect(() => {
    return () => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
    };
  }, []);

  const handleSelect = (entry: MemoryEntry | null) => selectEntry(entry);

  return (
    <div className={cn("flex flex-col gap-3", className)}>
      {/* Search input */}
      <div className="relative">
        <Input
          value={query}
          onChange={handleChange}
          placeholder="Search memories…"
          aria-label="Search memories"
        />
        {searching && (
          <span
            aria-label="Searching"
            className="absolute right-3 top-1/2 -translate-y-1/2 inline-block h-3 w-3 animate-spin rounded-full border-2 border-muted border-t-primary"
          />
        )}
      </div>

      {/* Error */}
      {searchError && (
        <p className="text-xs text-destructive">{searchError}</p>
      )}

      {/* Results */}
      {!searching && query && results.length === 0 && !searchError && (
        <p className="text-center text-sm text-muted-foreground py-6">
          No memories found for "{query}"
        </p>
      )}

      {results.length > 0 && (
        <div className="flex flex-col gap-1.5">
          {results.map((entry) => (
            <ResultRow
              key={entry.id}
              entry={entry}
              selected={selectedEntry?.id === entry.id}
              onSelect={handleSelect}
            />
          ))}
        </div>
      )}
    </div>
  );
}
