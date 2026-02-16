import type { ComponentProps, ReactNode } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Check, HelpCircle, X } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface OfflineCapabilitiesDialogProps {
  /** Whether the dialog is open */
  open?: boolean;
  /** Callback when dialog open state changes */
  onOpenChange?: (open: boolean) => void;
  /** Optional trigger element. If not provided, a default info button is rendered */
  trigger?: ReactNode;
  /** Additional CSS classes for the dialog content */
  className?: string;
}

/**
 * OfflineCapabilitiesDialog - Educational dialog about offline capabilities.
 *
 * Explains to users what features work offline vs require an active connection.
 *
 * **Behavior:**
 * - Modal dialog with two-column layout showing available vs unavailable features
 * - Can be used standalone or with a custom trigger button
 * - Default trigger is an info icon button
 *
 * **Accessibility:**
 * - Proper heading structure (h4 for section headings)
 * - Icons with aria-hidden="true" (decorative)
 * - Semantic HTML with ARIA attributes
 * - Keyboard-accessible trigger button
 * - Focus management handled by Dialog component
 *
 * **Example Usage:**
 * ```tsx
 * // With default trigger
 * <OfflineCapabilitiesDialog open={open} onOpenChange={setOpen} />
 *
 * // With custom trigger
 * <OfflineCapabilitiesDialog
 *   trigger={<Button>Learn More</Button>}
 *   open={open}
 *   onOpenChange={setOpen}
 * />
 *
 * // Controlled dialog without trigger
 * <OfflineCapabilitiesDialog open={open} onOpenChange={setOpen} />
 * ```
 */
export function OfflineCapabilitiesDialog({
  open,
  onOpenChange,
  trigger,
  className,
}: OfflineCapabilitiesDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      {trigger ? (
        <DialogTrigger asChild>{trigger}</DialogTrigger>
      ) : (
        <DialogTrigger asChild>
          <Button
            variant="ghost"
            size="icon"
            className="h-7 w-7"
            aria-label="Learn more about offline capabilities"
          >
            <HelpCircle className="h-4 w-4" aria-hidden="true" />
          </Button>
        </DialogTrigger>
      )}
      <DialogContent
        className={cn("max-w-2xl max-h-[90vh] overflow-y-auto", className)}
      >
        <DialogHeader>
          <DialogTitle>Offline Mode Capabilities</DialogTitle>
          <DialogDescription>
            Learn what features work offline and which require an active
            connection
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* Available Offline Section */}
          <section
            className="space-y-3"
            aria-labelledby="offline-available-heading"
          >
            <h4
              id="offline-available-heading"
              className="font-medium text-green-600 dark:text-green-400 flex items-center gap-2"
            >
              <Check className="w-4 h-4" aria-hidden="true" />
              Available Offline
            </h4>
            <ul className="mt-2 space-y-2 text-sm text-foreground">
              <li className="flex items-start gap-2">
                <Check
                  className="w-4 h-4 text-green-600 dark:text-green-400 mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                <span>
                  <strong>Browse schema structure</strong> - Explore tables,
                  columns, and their relationships
                </span>
              </li>
              <li className="flex items-start gap-2">
                <Check
                  className="w-4 h-4 text-green-600 dark:text-green-400 mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                <span>
                  <strong>View cached AI explanations</strong> - Read previously
                  generated explanations
                </span>
              </li>
              <li className="flex items-start gap-2">
                <Check
                  className="w-4 h-4 text-green-600 dark:text-green-400 mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                <span>
                  <strong>Read saved queries</strong> - Access your query
                  history
                </span>
              </li>
              <li className="flex items-start gap-2">
                <Check
                  className="w-4 h-4 text-green-600 dark:text-green-400 mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                <span>
                  <strong>View table and column metadata</strong> - Inspect data
                  types, constraints, and documentation
                </span>
              </li>
            </ul>
          </section>

          {/* Requires Connection Section */}
          <section
            className="space-y-3"
            aria-labelledby="offline-unavailable-heading"
          >
            <h4
              id="offline-unavailable-heading"
              className="font-medium text-red-600 dark:text-red-400 flex items-center gap-2"
            >
              <X className="w-4 h-4" aria-hidden="true" />
              Requires Connection
            </h4>
            <ul className="mt-2 space-y-2 text-sm text-foreground">
              <li className="flex items-start gap-2">
                <X
                  className="w-4 h-4 text-red-600 dark:text-red-400 mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                <span>
                  <strong>Execute queries against database</strong> - Run SQL
                  queries and view results
                </span>
              </li>
              <li className="flex items-start gap-2">
                <X
                  className="w-4 h-4 text-red-600 dark:text-red-400 mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                <span>
                  <strong>Refresh schema metadata</strong> - Update cached
                  schema information
                </span>
              </li>
              <li className="flex items-start gap-2">
                <X
                  className="w-4 h-4 text-red-600 dark:text-red-400 mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                <span>
                  <strong>Generate new AI explanations</strong> - Create new
                  AI-powered documentation
                </span>
              </li>
              <li className="flex items-start gap-2">
                <X
                  className="w-4 h-4 text-red-600 dark:text-red-400 mt-0.5 shrink-0"
                  aria-hidden="true"
                />
                <span>
                  <strong>Save annotations to backend</strong> - Persist new
                  notes and tags
                </span>
              </li>
            </ul>
          </section>

          {/* Info Section */}
          <section
            className="rounded-lg bg-muted p-4 text-sm"
            aria-labelledby="offline-info-heading"
          >
            <h5
              id="offline-info-heading"
              className="font-medium text-foreground mb-2"
            >
              How it works
            </h5>
            <p className="text-muted-foreground">
              When you go offline, dblens continues working with cached data.
              Schema information and previously generated explanations remain
              available, but new database queries and AI features require an
              active connection. Your work is automatically synchronized when
              you reconnect.
            </p>
          </section>
        </div>
      </DialogContent>
    </Dialog>
  );
}

/**
 * OfflineCapabilitiesTrigger - A standalone trigger button for the dialog.
 *
 * Use this when you need to place the trigger in a specific location,
 * such as within the OfflineBanner component.
 *
 * **Example:**
 * ```tsx
 * <OfflineCapabilitiesTrigger onClick={() => setOpen(true)} />
 * ```
 */
export function OfflineCapabilitiesTrigger({
  onClick,
  className,
  ...props
}: ComponentProps<"button">) {
  return (
    <Button
      variant="ghost"
      size="icon"
      className={cn("h-7 w-7", className)}
      onClick={onClick}
      aria-label="Learn more about offline capabilities"
      {...props}
    >
      <HelpCircle className="h-4 w-4" aria-hidden="true" />
    </Button>
  );
}
