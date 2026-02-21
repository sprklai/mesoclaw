import { cn } from "@/lib/utils";
import { Brain } from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";
import { CollapsibleHeaderTrigger } from "@/components/ui/collapsible-trigger";

export function Reasoning({ children, duration: _duration }: { children: ReactNode; duration?: number }) {
  return <div className="mb-2">{children}</div>;
}

export function ReasoningTrigger() {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <CollapsibleHeaderTrigger
      isOpen={isOpen}
      onToggle={() => setIsOpen(!isOpen)}
      title="View reasoning"
      icon={<Brain className="size-3" />}
      className="text-xs text-muted-foreground hover:text-foreground"
    />
  );
}

export function ReasoningContent({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("mt-2 rounded-md border border-border bg-muted/50 p-3 text-sm", className)}>
      {children}
    </div>
  );
}
