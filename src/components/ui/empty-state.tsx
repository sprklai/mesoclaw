import type { LucideIcon } from "@/lib/icons";

import { cn } from "@/lib/utils";

import { Button } from "./button";

interface EmptyStateProps {
  icon?: LucideIcon;
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
  className?: string;
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: EmptyStateProps) {
  return (
    <div
      className={cn("flex h-full items-center justify-center p-12", className)}
      role="status"
      aria-live="polite"
    >
      <div className="text-center max-w-md">
        {Icon && (
          <Icon
            className="mx-auto mb-4 h-12 w-12 text-muted-foreground/50"
            aria-hidden="true"
          />
        )}
        <p className="text-lg font-medium text-muted-foreground">{title}</p>
        {description && (
          <p className="mt-2 text-sm text-muted-foreground/70">{description}</p>
        )}
        {action && (
          <Button className="mt-6" onClick={action.onClick}>
            {action.label}
          </Button>
        )}
      </div>
    </div>
  );
}
