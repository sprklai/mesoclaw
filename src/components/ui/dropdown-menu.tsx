import type { ComponentProps, ReactNode } from "react";

import React, { useEffect, useRef, useState } from "react";

import { cn } from "@/lib/utils";

const DropdownMenuContext = React.createContext<{
  open: boolean;
  onOpenChange: (open: boolean) => void;
}>({
  open: false,
  onOpenChange: () => {},
});

export function DropdownMenu({
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
    <DropdownMenuContext.Provider
      value={{ open: isOpen, onOpenChange: handleOpenChange }}
    >
      <div className="relative inline-block">{children}</div>
    </DropdownMenuContext.Provider>
  );
}

export function DropdownMenuTrigger({
  children,
  asChild,
  className,
}: {
  children: ReactNode;
  asChild?: boolean;
  className?: string;
}) {
  const { open, onOpenChange } = React.useContext(DropdownMenuContext);

  const handleClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    onOpenChange(!open);
  };

  if (asChild && React.isValidElement(children)) {
    const childProps = children.props as {
      onClick?: (e: React.MouseEvent) => void;
      className?: string;
    };
    const originalOnClick = childProps.onClick;

    return React.cloneElement(children, {
      onClick: (e: React.MouseEvent) => {
        if (originalOnClick) {
          originalOnClick(e);
        }
        handleClick(e);
      },
      className: cn(className, childProps.className),
    } as React.HTMLAttributes<HTMLElement>);
  }

  return (
    <button className={className} onClick={handleClick}>
      {children}
    </button>
  );
}

export function DropdownMenuContent({
  children,
  className,
  align = "start",
  ...props
}: ComponentProps<"div"> & { align?: "start" | "end" | "center" }) {
  const { open, onOpenChange } = React.useContext(DropdownMenuContext);
  const contentRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        contentRef.current &&
        !contentRef.current.contains(event.target as Node)
      ) {
        onOpenChange(false);
      }
    };

    if (open) {
      document.addEventListener("mousedown", handleClickOutside);
      return () =>
        document.removeEventListener("mousedown", handleClickOutside);
    }
  }, [open, onOpenChange]);

  if (!open) return null;

  const alignmentClass = {
    start: "left-0",
    end: "right-0",
    center: "left-1/2 -translate-x-1/2",
  }[align];

  return (
    <div
      ref={contentRef}
      className={cn(
        "absolute z-50 mt-2 min-w-32 overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md",
        alignmentClass,
        className
      )}
      {...props}
    >
      {children}
    </div>
  );
}

export function DropdownMenuItem({
  className,
  onClick,
  ...props
}: ComponentProps<"div">) {
  const { onOpenChange } = React.useContext(DropdownMenuContext);

  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    onClick?.(e);
    onOpenChange(false);
  };

  return (
    <div
      className={cn(
        "relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground data-disabled:pointer-events-none data-disabled:opacity-50",
        className
      )}
      onClick={handleClick}
      {...props}
    />
  );
}

export function DropdownMenuSeparator({
  className,
  ...props
}: ComponentProps<"div">) {
  return (
    <div className={cn("-mx-1 my-1 h-px bg-muted", className)} {...props} />
  );
}

export function DropdownMenuLabel({
  className,
  ...props
}: ComponentProps<"div">) {
  return (
    <div
      className={cn("px-2 py-1.5 text-sm font-semibold", className)}
      {...props}
    />
  );
}

export function DropdownMenuCheckboxItem({
  className,
  checked,
  onCheckedChange,
  children,
  ...props
}: ComponentProps<"div"> & {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
}) {
  const handleClick = () => {
    onCheckedChange(!checked);
  };

  return (
    <div
      className={cn(
        "relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground data-disabled:pointer-events-none data-disabled:opacity-50",
        className
      )}
      onClick={handleClick}
      role="menuitemcheckbox"
      aria-checked={checked}
      {...props}
    >
      <span className="mr-2 h-4 w-4 shrink-0">{checked ? "☑" : "☐"}</span>
      {children}
    </div>
  );
}
