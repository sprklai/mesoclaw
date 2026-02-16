import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

export interface SuggestionProps {
  suggestion: string;
  onClick: () => void;
  className?: string;
}

export function Suggestion({ suggestion, onClick, className }: SuggestionProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "rounded-lg border border-border bg-background px-4 py-2 text-sm transition-colors hover:bg-muted",
        className
      )}
    >
      {suggestion}
    </button>
  );
}

export function Suggestions({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex flex-wrap gap-2", className)}>
      {children}
    </div>
  );
}
