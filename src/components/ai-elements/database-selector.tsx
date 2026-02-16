/**
 * Database Selector Components
 *
 * Command palette-style selector for database types with grouping and badges.
 * Follows the same pattern as ModelSelector from AI SDK Elements.
 */

import type { ComponentProps, ReactNode } from "react";

import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command";
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Database,
  getDatabaseBrandColor,
  getDatabaseIcon,
  HardDrive,
} from "@/lib/icons";
import { cn } from "@/lib/utils";
import { DatabaseType } from "@/types/database-registry";

export type DatabaseSelectorProps = ComponentProps<typeof Command>;

export const DatabaseSelector = (props: DatabaseSelectorProps) => (
  <Command {...props} />
);

export type DatabaseSelectorTriggerProps = ComponentProps<typeof DialogTrigger>;

export const DatabaseSelectorTrigger = (
  props: DatabaseSelectorTriggerProps
) => <DialogTrigger {...props} />;

export type DatabaseSelectorContentProps = ComponentProps<
  typeof DialogContent
> & {
  title?: ReactNode;
};

export const DatabaseSelectorContent = ({
  className,
  children,
  title = "Select Database",
  ...props
}: DatabaseSelectorContentProps) => (
  <DialogContent
    className={cn(
      "overflow-hidden p-0 shadow-xl",
      "will-change-transform",
      "w-full min-w-[320px] max-w-[500px]",
      "bg-popover text-popover-foreground",
      className
    )}
    {...props}
  >
    <DialogTitle className="sr-only">{title}</DialogTitle>
    {children}
  </DialogContent>
);

export type DatabaseSelectorDialogProps = ComponentProps<typeof Dialog>;

export const DatabaseSelectorDialog = (props: DatabaseSelectorDialogProps) => (
  <Dialog {...props} />
);

export type DatabaseSelectorInputProps = ComponentProps<typeof CommandInput>;

export const DatabaseSelectorInput = ({
  className,
  ...props
}: DatabaseSelectorInputProps) => (
  <CommandInput className={cn("border-0 focus:ring-0", className)} {...props} />
);

export type DatabaseSelectorListProps = ComponentProps<typeof CommandList>;

export const DatabaseSelectorList = (props: DatabaseSelectorListProps) => (
  <CommandList {...props} />
);

export type DatabaseSelectorEmptyProps = ComponentProps<typeof CommandEmpty>;

export const DatabaseSelectorEmpty = (props: DatabaseSelectorEmptyProps) => (
  <CommandEmpty {...props} />
);

export type DatabaseSelectorGroupProps = ComponentProps<typeof CommandGroup>;

export const DatabaseSelectorGroup = ({
  className,
  ...props
}: DatabaseSelectorGroupProps) => (
  <CommandGroup
    className={cn(
      "**:[[cmdk-group-heading]]:px-3 **:[[cmdk-group-heading]]:py-2 **:[[cmdk-group-heading]]:text-sm **:[[cmdk-group-heading]]:font-semibold **:[[cmdk-group-heading]]:text-foreground",
      className
    )}
    {...props}
  />
);

export interface DatabaseSelectorItemProps extends ComponentProps<
  typeof CommandItem
> {
  badge?: string;
}

export const DatabaseSelectorItem = ({
  className,
  disabled,
  badge,
  children,
  ...props
}: DatabaseSelectorItemProps) => (
  <CommandItem
    className={cn(
      "px-3 py-2.5",
      "focus:bg-accent",
      disabled && "opacity-50 cursor-not-allowed",
      className
    )}
    disabled={disabled}
    {...props}
  >
    <div className="flex items-center justify-between w-full">
      <div className="flex items-center gap-3 flex-1 min-w-0">{children}</div>
      {badge && (
        <span className="ml-2 text-xs px-2 py-0.5 rounded-full bg-muted text-muted-foreground shrink-0">
          {badge}
        </span>
      )}
    </div>
  </CommandItem>
);

export interface DatabaseSelectorLogoProps {
  connectionMode?: "file" | "network";
  databaseType?: DatabaseType;
  showBrandColor?: boolean;
  className?: string;
}

export const DatabaseSelectorLogo = ({
  connectionMode = "network",
  databaseType,
  showBrandColor = true,
  className,
}: DatabaseSelectorLogoProps) => {
  // If database type is provided, use the brand icon
  if (databaseType) {
    const Icon = getDatabaseIcon(databaseType);
    const brandColor = showBrandColor
      ? getDatabaseBrandColor(databaseType)
      : undefined;

    return (
      <div
        className={cn(
          "flex h-5 w-5 shrink-0 items-center justify-center",
          className
        )}
      >
        <Icon
          className="h-full w-full"
          style={brandColor ? { color: brandColor } : undefined}
        />
      </div>
    );
  }

  // Fallback to generic icons based on connection mode
  const Icon = connectionMode === "file" ? HardDrive : Database;
  return (
    <div
      className={cn(
        "flex h-5 w-5 shrink-0 items-center justify-center",
        className
      )}
    >
      <Icon className="h-full w-full text-muted-foreground" />
    </div>
  );
};

export type DatabaseSelectorNameProps = ComponentProps<"span">;

export const DatabaseSelectorName = ({
  className,
  ...props
}: DatabaseSelectorNameProps) => (
  <span
    className={cn("text-sm font-medium text-foreground", className)}
    {...props}
  />
);

export type DatabaseSelectorDescriptionProps = ComponentProps<"span">;

export const DatabaseSelectorDescription = ({
  className,
  ...props
}: DatabaseSelectorDescriptionProps) => (
  <span className={cn("text-xs text-muted-foreground", className)} {...props} />
);
