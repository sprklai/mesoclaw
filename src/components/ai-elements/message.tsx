import { cn } from "@/lib/utils";
import { Markdown } from "@/components/ai-elements/markdown";
import type { ReactNode } from "react";

export interface MessageProps {
  from: "user" | "assistant";
  children: ReactNode;
  className?: string;
}

export function Message({ from, children, className }: MessageProps) {
  const isUser = from === "user";

  return (
    <div
      className={cn(
        "group flex gap-4 px-4 py-4",
        isUser && "justify-end",
        className
      )}
    >
      <div
        className={cn(
          "flex min-w-0 max-w-[85%] flex-col gap-2",
          isUser && "items-end"
        )}
      >
        {children}
      </div>
    </div>
  );
}

export function MessageContent({ children }: { children: ReactNode }) {
  return <div className="prose prose-sm max-w-none dark:prose-invert">{children}</div>;
}

export function MessageResponse({ children }: { children: string }) {
  return <Markdown content={children} />;
}

// Message branching components (for multiple versions of a message)
export function MessageBranch({
  children,
  defaultBranch: _defaultBranch = 0,
}: {
  children: ReactNode;
  defaultBranch?: number;
}) {
  // For now, just render children without branching logic
  // This can be enhanced later with state management
  return <div>{children}</div>;
}

export function MessageBranchContent({ children }: { children: ReactNode }) {
  return <>{children}</>;
}

export function MessageBranchSelector({ children }: { children: ReactNode }) {
  return <div className="mt-2 flex items-center gap-2 text-sm text-muted-foreground">{children}</div>;
}

export function MessageBranchPrevious() {
  return (
    <button
      type="button"
      className="rounded px-2 py-1 hover:bg-muted"
      onClick={() => {
        // Handle previous branch
      }}
    >
      ←
    </button>
  );
}

export function MessageBranchNext() {
  return (
    <button
      type="button"
      className="rounded px-2 py-1 hover:bg-muted"
      onClick={() => {
        // Handle next branch
      }}
    >
      →
    </button>
  );
}

export function MessageBranchPage() {
  return <span>1 / 1</span>;
}
