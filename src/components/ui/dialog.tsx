import type { ComponentProps, ReactNode } from "react";

import React, { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

// Create context first
const DialogContext = React.createContext<{
  open: boolean;
  onOpenChange: (open: boolean) => void;
}>({
  open: false,
  onOpenChange: () => {},
});

interface DialogProps {
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  children: ReactNode;
}

export function Dialog({ open, onOpenChange, children }: DialogProps) {
  const [isOpen, setIsOpen] = useState(open ?? false);

  useEffect(() => {
    if (open !== undefined) {
      setIsOpen(open);
    }
  }, [open]);

  const handleOpenChange = (newOpen: boolean) => {
    setIsOpen(newOpen);
    onOpenChange?.(newOpen);
  };

  return (
    <DialogContext.Provider
      value={{ open: isOpen, onOpenChange: handleOpenChange }}
    >
      {children}
    </DialogContext.Provider>
  );
}

export function DialogTrigger({
  children,
  asChild,
}: {
  children: ReactNode;
  asChild?: boolean;
}) {
  const { onOpenChange } = React.useContext(DialogContext);

  const handleClick = () => onOpenChange(true);

  if (asChild && React.isValidElement(children)) {
    return React.cloneElement(children, {
      onClick: handleClick,
    } as React.HTMLAttributes<HTMLElement>);
  }

  return <button onClick={handleClick}>{children}</button>;
}

export function DialogContent({
  children,
  className,
  ...props
}: ComponentProps<"div">) {
  const { open, onOpenChange } = React.useContext(DialogContext);

  // Generate IDs for accessibility (must be before conditional return)
  const titleId = React.useId();
  const descriptionId = React.useId();

  if (!open) return null;

  // Parse z-index from className to support nested dialogs
  const zIndexMatch = className?.match(/z-\[(\d+)\]/);
  const zIndex = zIndexMatch ? parseInt(zIndexMatch[1], 10) : 50;

  return (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-50 bg-black/80"
        style={{ zIndex }}
        onClick={() => onOpenChange(false)}
        aria-hidden="true"
      />

      {/*
       * Responsive dialog content:
       *
       * Mobile (< sm): Bottom sheet pattern — slides up from the bottom edge.
       *   - Full width, rounded top corners only
       *   - Max height 90vh with overflow scroll
       *   - Safe area bottom padding for notched devices
       *
       * Desktop (>= sm): Centered modal — classic overlay pattern.
       *   - Centered in viewport with padding
       *   - All corners rounded
       */}

      {/* Desktop: centered modal container */}
      <div
        className="fixed inset-0 z-50 hidden items-center justify-center p-4 overflow-y-auto sm:flex"
        style={{ zIndex }}
      >
        <div
          className={cn(
            "grid w-full max-w-lg gap-4 border bg-popover shadow-xl rounded-lg relative overflow-hidden p-6",
            "will-change-transform",
            className,
          )}
          style={{ color: "var(--popover-foreground)" }}
          role="dialog"
          aria-modal="true"
          aria-labelledby={titleId}
          aria-describedby={descriptionId}
          onClick={(e) => e.stopPropagation()}
          {...props}
        >
          {renderChildren(children, titleId, descriptionId)}
          <CloseButton onClick={() => onOpenChange(false)} />
        </div>
      </div>

      {/* Mobile: bottom sheet container */}
      <div
        className="fixed inset-x-0 bottom-0 z-50 flex sm:hidden"
        style={{ zIndex }}
      >
        <div
          className={cn(
            "grid w-full gap-4 border-t bg-popover shadow-xl",
            "rounded-t-2xl relative overflow-hidden p-6",
            // Safe area bottom padding for iPhone home indicator
            "pb-[calc(1.5rem+env(safe-area-inset-bottom))]",
            // Max height prevents the sheet from covering the full screen
            "max-h-[90dvh] overflow-y-auto",
            "will-change-transform",
            className,
          )}
          style={{ color: "var(--popover-foreground)" }}
          role="dialog"
          aria-modal="true"
          aria-labelledby={titleId}
          aria-describedby={descriptionId}
          onClick={(e) => e.stopPropagation()}
          {...props}
        >
          {/* Bottom sheet drag handle indicator */}
          <div className="absolute left-1/2 top-3 h-1 w-8 -translate-x-1/2 rounded-full bg-muted-foreground/30" />

          {renderChildren(children, titleId, descriptionId)}
          <CloseButton onClick={() => onOpenChange(false)} />
        </div>
      </div>
    </>
  );
}

/**
 * Clones children to inject accessibility IDs onto DialogTitle and
 * DialogDescription elements.
 */
function renderChildren(
  children: ReactNode,
  titleId: string,
  descriptionId: string,
): ReactNode {
  return React.Children.map(children, (child) => {
    if (React.isValidElement(child)) {
      if (child.type === DialogTitle) {
        return React.cloneElement(child, {
          id: (child.props as { id?: string }).id || titleId,
        } as React.HTMLAttributes<HTMLElement>);
      }
      if (child.type === DialogDescription) {
        return React.cloneElement(child, {
          id: (child.props as { id?: string }).id || descriptionId,
        } as React.HTMLAttributes<HTMLElement>);
      }
    }
    return child;
  });
}

function CloseButton({ onClick }: { onClick: () => void }) {
  return (
    <button
      className="absolute right-6 top-6 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 min-h-[44px] min-w-[44px] flex items-center justify-center"
      onClick={onClick}
      aria-label="Close dialog"
    >
      <span className="sr-only">Close</span>
      <svg
        className="h-4 w-4"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        aria-hidden="true"
      >
        <path
          d="M6 18L18 6M6 6l12 12"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
        />
      </svg>
    </button>
  );
}

export function DialogHeader({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      className={cn(
        "flex flex-col space-y-1.5 text-center sm:text-left",
        className,
      )}
      {...props}
    />
  );
}

export function DialogFooter({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      className={cn(
        "flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2",
        className,
      )}
      {...props}
    />
  );
}

export function DialogTitle({ className, ...props }: ComponentProps<"h2">) {
  return (
    <h2
      className={cn(
        "text-lg font-semibold leading-none tracking-tight",
        className,
      )}
      {...props}
    />
  );
}

export function DialogDescription({
  className,
  ...props
}: ComponentProps<"p">) {
  return (
    <p className={cn("text-sm text-muted-foreground", className)} {...props} />
  );
}
