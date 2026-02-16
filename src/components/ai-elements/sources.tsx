import { cn } from "@/lib/utils";
import { ExternalLink } from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";

export function Sources({ children }: { children: ReactNode }) {
  return <div className="mb-2">{children}</div>;
}

export function SourcesTrigger({ count }: { count: number }) {
  const [isOpen, setIsOpen] = useState(false);

  return (
    <button
      type="button"
      onClick={() => setIsOpen(!isOpen)}
      className="text-xs text-muted-foreground hover:text-foreground"
    >
      {count} source{count !== 1 ? "s" : ""}
    </button>
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
