import type { ComponentProps, ReactNode } from "react";

import React, { useEffect, useState } from "react";

import { cn } from "@/lib/utils";

const AlertDialogContext = React.createContext<{
  open: boolean;
  onOpenChange: (open: boolean) => void;
}>({
  open: false,
  onOpenChange: () => {},
});

export function AlertDialog({
  children,
  open,
  onOpenChange,
}: {
  children: ReactNode;
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
}) {
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
    <AlertDialogContext.Provider
      value={{ open: isOpen, onOpenChange: handleOpenChange }}
    >
      {children}
    </AlertDialogContext.Provider>
  );
}

export function AlertDialogTrigger({
  children,
  asChild,
}: {
  children: ReactNode;
  asChild?: boolean;
}) {
  const { onOpenChange } = React.useContext(AlertDialogContext);

  const handleClick = () => onOpenChange(true);

  if (asChild && React.isValidElement(children)) {
    return React.cloneElement(children, {
      onClick: handleClick,
    } as React.HTMLAttributes<HTMLElement>);
  }

  return <button onClick={handleClick}>{children}</button>;
}

export function AlertDialogContent({
  children,
  className,
  ...props
}: ComponentProps<"div">) {
  const { open } = React.useContext(AlertDialogContext);

  if (!open) return null;

  return (
    <>
      {/* Backdrop */}
      <div className="fixed inset-0 z-50 bg-black/80" />
      {/* Content */}
      <div className="fixed left-[50%] top-[50%] z-50 translate-x-[-50%] translate-y-[-50%]">
        <div
          className={cn(
            "grid w-full max-w-lg gap-4 border bg-background p-6 shadow-lg sm:rounded-lg",
            className
          )}
          onClick={(e) => e.stopPropagation()}
          {...props}
        >
          {children}
        </div>
      </div>
    </>
  );
}

export function AlertDialogHeader({
  className,
  ...props
}: ComponentProps<"div">) {
  return (
    <div
      className={cn(
        "flex flex-col space-y-2 text-center sm:text-left",
        className
      )}
      {...props}
    />
  );
}

export function AlertDialogFooter({
  className,
  ...props
}: ComponentProps<"div">) {
  return (
    <div
      className={cn(
        "flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2",
        className
      )}
      {...props}
    />
  );
}

export function AlertDialogTitle({
  className,
  ...props
}: ComponentProps<"h2">) {
  return <h2 className={cn("text-lg font-semibold", className)} {...props} />;
}

export function AlertDialogDescription({
  className,
  ...props
}: ComponentProps<"p">) {
  return (
    <p className={cn("text-sm text-muted-foreground", className)} {...props} />
  );
}

export function AlertDialogAction({
  className,
  ...props
}: ComponentProps<"button">) {
  const { onOpenChange } = React.useContext(AlertDialogContext);

  return (
    <button
      className={cn(
        "inline-flex h-10 items-center justify-center rounded-md bg-primary px-4 py-2 text-sm font-semibold text-primary-foreground ring-offset-background transition-colors hover:bg-primary/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
        className
      )}
      onClick={(e) => {
        props.onClick?.(e);
        onOpenChange(false);
      }}
      {...props}
    />
  );
}

export function AlertDialogCancel({
  className,
  ...props
}: ComponentProps<"button">) {
  const { onOpenChange } = React.useContext(AlertDialogContext);

  return (
    <button
      className={cn(
        "inline-flex h-10 items-center justify-center rounded-md border border-input bg-background px-4 py-2 text-sm font-semibold ring-offset-background transition-colors hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50",
        className
      )}
      onClick={() => onOpenChange(false)}
      {...props}
    />
  );
}
