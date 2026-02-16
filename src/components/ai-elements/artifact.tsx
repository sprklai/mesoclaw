/**
 * Artifact Component
 *
 * A structured container for displaying generated content like code, documents,
 * or other outputs with built-in header actions.
 *
 * Based on: https://ai-sdk.dev/elements/components/artifact
 */

import { type ComponentProps, forwardRef } from "react";

import { Button } from "@/components/ui/button";
import { type LucideIcon, X } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface ArtifactProps {
  /** Child elements to render inside the artifact. */
  children: React.ReactNode;
  /** Additional CSS classes to apply. */
  className?: string;
}

interface ArtifactHeaderProps {
  /** Child elements (usually title, description, and actions). */
  children: React.ReactNode;
  /** Additional CSS classes to apply. */
  className?: string;
}

interface ArtifactTitleProps extends ComponentProps<"p"> {
  /** Title text content. */
  children: React.ReactNode;
}

interface ArtifactDescriptionProps extends ComponentProps<"p"> {
  /** Description text content. */
  children: React.ReactNode;
}

interface ArtifactActionsProps {
  /** Action buttons to display. */
  children: React.ReactNode;
  /** Additional CSS classes to apply. */
  className?: string;
}

interface ArtifactActionProps {
  /** Lucide icon component to display. */
  icon?: LucideIcon;
  /** Screen reader label for accessibility. */
  label?: string;
  /** Tooltip text to display on hover. */
  tooltip?: string;
  /** Click handler. */
  onClick?: () => void;
  /** Additional CSS classes to apply. */
  className?: string;
  /** Button type. */
  type?: "button" | "submit" | "reset";
}

interface ArtifactContentProps {
  /** Main content to display in the artifact body. */
  children: React.ReactNode;
  /** Additional CSS classes to apply. */
  className?: string;
}

/**
 * Main Artifact container component.
 */
export function Artifact({ children, className }: ArtifactProps) {
  return (
    <div
      className={cn(
        "overflow-hidden rounded-xl border border-border bg-card shadow-sm",
        className
      )}
    >
      {children}
    </div>
  );
}

/**
 * Header section of the artifact containing title, description, and actions.
 */
export function ArtifactHeader({ children, className }: ArtifactHeaderProps) {
  return (
    <div
      className={cn(
        "flex items-start justify-between border-b border-border bg-muted/30 px-4 py-3",
        className
      )}
    >
      {children}
    </div>
  );
}

/**
 * Title text in the artifact header.
 */
export function ArtifactTitle({
  children,
  className,
  ...props
}: ArtifactTitleProps) {
  return (
    <p
      className={cn("text-sm font-semibold text-foreground", className)}
      {...props}
    >
      {children}
    </p>
  );
}

/**
 * Description text in the artifact header.
 */
export function ArtifactDescription({
  children,
  className,
  ...props
}: ArtifactDescriptionProps) {
  return (
    <p className={cn("text-xs text-muted-foreground", className)} {...props}>
      {children}
    </p>
  );
}

/**
 * Container for action buttons in the header.
 */
export function ArtifactActions({ children, className }: ArtifactActionsProps) {
  return (
    <div className={cn("flex items-center gap-1", className)}>{children}</div>
  );
}

/**
 * Individual action button with icon, optional label, and tooltip.
 */
export function ArtifactAction({
  icon: Icon,
  label,
  tooltip,
  onClick,
  className,
  type = "button",
}: ArtifactActionProps) {
  return (
    <Button
      variant="ghost"
      size="sm"
      className={cn("h-7 w-7 p-0", className)}
      onClick={onClick}
      title={tooltip || label}
      type={type}
      aria-label={label}
    >
      {Icon && <Icon className="h-3.5 w-3.5" />}
    </Button>
  );
}

/**
 * Close button for dismissing the artifact.
 */
export const ArtifactClose = forwardRef<
  HTMLButtonElement,
  ComponentProps<typeof Button>
>(function ArtifactClose({ className, ...props }, ref) {
  return (
    <Button
      ref={ref}
      variant="ghost"
      size="sm"
      className={cn("h-7 w-7 p-0", className)}
      {...props}
    >
      <X className="h-3.5 w-3.5" />
    </Button>
  );
});

/**
 * Main content area of the artifact.
 */
export function ArtifactContent({ children, className }: ArtifactContentProps) {
  return <div className={cn("p-4", className)}>{children}</div>;
}
