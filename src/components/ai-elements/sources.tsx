import { cn } from "@/lib/utils";
import { ExternalLink } from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";
import { CollapsibleHeaderTrigger } from "@/components/ui/collapsible-trigger";

export function Sources({ children }: { children: ReactNode }) {
  return <div className="mb-2">{children}</div>;
}

export function SourcesTrigger({ count }: { count: number }) {
  const [isOpen, setIsOpen] = useState(false);
  const title = `${count} source${count !== 1 ? "s" : ""}`;

  return (
    <CollapsibleHeaderTrigger
      isOpen={isOpen}
      onToggle={() => setIsOpen(!isOpen)}
      title={title}
      className="text-xs text-muted-foreground hover:text-foreground"
    />
  );
}

export function SourcesContent({ children, className }: { children: ReactNode; className?: string }) {
  return (
    <div className={cn("mt-2 space-y-1 rounded-md border border-border bg-muted/50 p-2", className)}>
      {children}
    </div>
  );
}

export function Source({ href, title }: { href: string; title: string }) {
  return (
    <a
      href={href}
      target="_blank"
      rel="noopener noreferrer"
      className="flex items-center gap-2 text-sm hover:underline"
    >
      <ExternalLink className="size-3" />
      {title}
    </a>
  );
}
