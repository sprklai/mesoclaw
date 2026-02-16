import type { ComponentProps } from "react";

import { createContext, useContext, useState } from "react";

import { cn } from "@/lib/utils";

interface CollapsibleContextValue {
  open: boolean;
  setOpen: (open: boolean) => void;
}

const CollapsibleContext = createContext<CollapsibleContextValue | null>(null);

function useCollapsibleContext() {
  const context = useContext(CollapsibleContext);
  if (!context) {
    throw new Error("Collapsible components must be used within Collapsible");
  }
  return context;
}

interface CollapsibleProps extends ComponentProps<"div"> {
  open?: boolean;
  onOpenChange?: (open: boolean) => void;
  defaultOpen?: boolean;
}

export function Collapsible({
  open: controlledOpen,
  onOpenChange,
  defaultOpen = false,
  children,
  className,
  ...props
}: CollapsibleProps) {
  const [uncontrolledOpen, setUncontrolledOpen] = useState(defaultOpen);
  const open = controlledOpen !== undefined ? controlledOpen : uncontrolledOpen;

  const handleSetOpen = (newOpen: boolean) => {
    if (controlledOpen === undefined) {
      setUncontrolledOpen(newOpen);
    }
    onOpenChange?.(newOpen);
  };

  return (
    <CollapsibleContext.Provider value={{ open, setOpen: handleSetOpen }}>
      <div className={cn("", className)} {...props}>
        {children}
      </div>
    </CollapsibleContext.Provider>
  );
}

type CollapsibleTriggerProps = ComponentProps<"button">;

export function CollapsibleTrigger({
  children,
  className,
  onClick,
  ...props
}: CollapsibleTriggerProps) {
  const { open, setOpen } = useCollapsibleContext();

  const handleClick = (e: React.MouseEvent<HTMLButtonElement>) => {
    setOpen(!open);
    onClick?.(e);
  };

  return (
    <button
      className={cn("", className)}
      type="button"
      onClick={handleClick}
      {...props}
    >
      {children}
    </button>
  );
}

type CollapsibleContentProps = ComponentProps<"div">;

export function CollapsibleContent({
  children,
  className,
  ...props
}: CollapsibleContentProps) {
  const { open } = useCollapsibleContext();

  if (!open) {
    return null;
  }

  return (
    <div className={cn("animate-in slide-in-from-top-2", className)} {...props}>
      {children}
    </div>
  );
}
