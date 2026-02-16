import { cn } from "@/lib/utils";
import { X } from "lucide-react";
import type { ReactNode } from "react";

export interface AttachmentData {
  id: string;
  name: string;
  type: string;
  url: string;
}

export function Attachments({
  children,
  variant = "default",
  className,
}: {
  children: ReactNode;
  variant?: "default" | "inline";
  className?: string;
}) {
  return (
    <div className={cn("flex flex-wrap gap-2", variant === "inline" && "py-2", className)}>
      {children}
    </div>
  );
}

export function Attachment({
  data: _data,
  onRemove: _onRemove,
  children,
}: {
  data: AttachmentData;
  onRemove: () => void;
  children: ReactNode;
}) {
  return (
    <div className="relative inline-flex items-center gap-2 rounded-md border border-border bg-muted px-3 py-2">
      {children}
    </div>
  );
}

export function AttachmentPreview() {
  return (
    <div className="flex items-center gap-2">
      <span className="text-sm">📎 Attachment</span>
    </div>
  );
}

export function AttachmentRemove() {
  return (
    <button
      type="button"
      className="rounded-sm p-0.5 hover:bg-muted-foreground/10"
    >
      <X className="size-3" />
    </button>
  );
}
