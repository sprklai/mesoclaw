import type { ComponentProps, ReactNode } from "react";

import {
  createContext,
  useContext,
  useEffect,
  useRef,
  useState,
  useCallback,
} from "react";

import { cn } from "@/lib/utils";

interface ContextMenuContextValue {
  open: boolean;
  position: { x: number; y: number };
  onOpenChange: (open: boolean) => void;
  setPosition: (position: { x: number; y: number }) => void;
}

const ContextMenuContext = createContext<ContextMenuContextValue>({
  open: false,
  position: { x: 0, y: 0 },
  onOpenChange: () => {},
  setPosition: () => {},
});

export function ContextMenu({ children }: { children: ReactNode }) {
  const [isOpen, setIsOpen] = useState(false);
  const [position, setPosition] = useState({ x: 0, y: 0 });

  return (
    <ContextMenuContext.Provider
      value={{
        open: isOpen,
        position,
        onOpenChange: setIsOpen,
        setPosition,
      }}
    >
      {children}
    </ContextMenuContext.Provider>
  );
}

export function ContextMenuTrigger({
  children,
  className,
  disabled,
}: {
  children: ReactNode;
  className?: string;
  disabled?: boolean;
}) {
  const { onOpenChange, setPosition } = useContext(ContextMenuContext);

  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      if (disabled) return;
      e.preventDefault();
      e.stopPropagation();
      setPosition({ x: e.clientX, y: e.clientY });
      onOpenChange(true);
    },
    [disabled, onOpenChange, setPosition]
  );

  return (
    <div className={className} onContextMenu={handleContextMenu}>
      {children}
    </div>
  );
}

export function ContextMenuContent({
  children,
  className,
  ...props
}: ComponentProps<"div">) {
  const { open, position, onOpenChange } = useContext(ContextMenuContext);
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

    const handleEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onOpenChange(false);
      }
    };

    const handleScroll = () => {
      onOpenChange(false);
    };

    if (open) {
      document.addEventListener("mousedown", handleClickOutside);
      document.addEventListener("keydown", handleEscape);
      window.addEventListener("scroll", handleScroll, true);
      return () => {
        document.removeEventListener("mousedown", handleClickOutside);
        document.removeEventListener("keydown", handleEscape);
        window.removeEventListener("scroll", handleScroll, true);
      };
    }
  }, [open, onOpenChange]);

  // Adjust position to keep menu within viewport
  useEffect(() => {
    if (open && contentRef.current) {
      const rect = contentRef.current.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;

      let x = position.x;
      let y = position.y;

      if (x + rect.width > viewportWidth) {
        x = viewportWidth - rect.width - 8;
      }
      if (y + rect.height > viewportHeight) {
        y = viewportHeight - rect.height - 8;
      }

      if (x !== position.x || y !== position.y) {
        contentRef.current.style.left = `${x}px`;
        contentRef.current.style.top = `${y}px`;
      }
    }
  }, [open, position]);

  if (!open) return null;

  return (
    <div
      ref={contentRef}
      className={cn(
        "fixed z-[100] min-w-[8rem] overflow-hidden rounded-md border bg-popover p-1 text-popover-foreground shadow-md animate-in fade-in-0 zoom-in-95",
        className
      )}
      style={{ left: position.x, top: position.y }}
      {...props}
    >
      {children}
    </div>
  );
}

export function ContextMenuItem({
  className,
  onClick,
  disabled,
  ...props
}: ComponentProps<"div"> & { disabled?: boolean }) {
  const { onOpenChange } = useContext(ContextMenuContext);

  const handleClick = (e: React.MouseEvent<HTMLDivElement>) => {
    if (disabled) return;
    onClick?.(e);
    onOpenChange(false);
  };

  return (
    <div
      className={cn(
        "relative flex cursor-pointer select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none transition-colors hover:bg-accent hover:text-accent-foreground focus:bg-accent focus:text-accent-foreground",
        disabled && "pointer-events-none opacity-50",
        className
      )}
      onClick={handleClick}
      {...props}
    />
  );
}

export function ContextMenuSeparator({
  className,
  ...props
}: ComponentProps<"div">) {
  return (
    <div className={cn("-mx-1 my-1 h-px bg-muted", className)} {...props} />
  );
}

export function ContextMenuLabel({
  className,
  ...props
}: ComponentProps<"div">) {
  return (
    <div
      className={cn(
        "px-2 py-1.5 text-xs font-medium text-muted-foreground",
        className
      )}
      {...props}
    />
  );
}
