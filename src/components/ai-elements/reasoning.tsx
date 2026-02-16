import { cn } from "@/lib/utils";
import { Brain } from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";

export function Reasoning({ children, duration: _duration }: { children: ReactNode; duration?: number }) {
  return <div className="mb-2">{children}</div>;
}

export function ReasoningTrigger() {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <button
      type="button"
      onClick={() => setIsOpen(!isOpen)}
      className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
    >
      <Brain className="size-3" />
      View reasoning
    </button>
  );
}

export function ReasoningContent({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("mt-2 rounded-md border border-border bg-muted/50 p-3 text-sm", className)}>
      {children}
    </div>
  );
}
