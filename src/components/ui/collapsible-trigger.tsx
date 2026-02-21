import type { ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

interface CollapsibleHeaderTriggerProps {
  isOpen: boolean;
  onToggle: () => void;
  title: string;
  icon?: ReactNode;
  className?: string;
}

export function CollapsibleHeaderTrigger({
  isOpen,
  onToggle,
  title,
  icon,
  className,
}: CollapsibleHeaderTriggerProps) {
  return (
    <button
      type="button"
      onClick={onToggle}
      className={cn(
        "flex w-full items-center gap-2 rounded-md px-3 py-2",
        "text-sm font-medium text-foreground",
        "hover:bg-accent transition-colors",
        className
      )}
    >
      {icon}
      <span className="flex-1 text-left">{title}</span>
      <ChevronDown
        className={cn(
          "size-4 text-muted-foreground transition-transform",
          isOpen && "rotate-180"
        )}
      />
    </button>
  );
}
