import * as React from "react";
import { createPortal } from "react-dom";

import { cn } from "@/lib/utils";

interface HoverCardProps {
  children: React.ReactNode;
  content: React.ReactNode;
  side?: "top" | "bottom" | "left" | "right";
  align?: "start" | "center" | "end";
}

export function HoverCard({
  children,
  content,
  side = "bottom",
  align = "center",
}: HoverCardProps) {
  const [isOpen, setIsOpen] = React.useState(false);
  const [position, setPosition] = React.useState<{
    top: number;
    left: number;
  } | null>(null);
  const triggerRef = React.useRef<HTMLSpanElement>(null);
  const contentRef = React.useRef<HTMLDivElement>(null);

  const handleMouseEnter = () => {
    if (!triggerRef.current) return;
    setIsOpen(true);
    // Calculate position immediately
    calculatePosition();
  };

  const handleMouseLeave = () => {
    setIsOpen(false);
    setPosition(null);
  };

  const calculatePosition = () => {
    if (!triggerRef.current) return;

    const triggerRect = triggerRef.current.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    // Create a temporary element to measure content size
    const tempDiv = document.createElement("div");
    tempDiv.style.position = "absolute";
    tempDiv.style.visibility = "hidden";
    tempDiv.style.width = "16rem"; // w-64
    tempDiv.style.padding = "0.75rem"; // px-3 py-2
    tempDiv.style.fontSize = "0.875rem"; // text-sm
    tempDiv.style.wordWrap = "break-word";
    tempDiv.textContent =
      typeof content === "string" ? content : "Content measurement";
    document.body.appendChild(tempDiv);
    const contentWidth = tempDiv.offsetWidth;
    const contentHeight = tempDiv.offsetHeight;
    document.body.removeChild(tempDiv);

    let top = 0;
    let left = 0;
    const gap = 8; // 0.5rem gap

    // Calculate horizontal position
    if (side === "bottom" || side === "top") {
      if (align === "start") {
        left = triggerRect.left;
        // Adjust if would overflow right edge
        if (left + contentWidth > viewportWidth - 8) {
          left = Math.max(8, viewportWidth - contentWidth - 8);
        }
      } else if (align === "end") {
        left = triggerRect.right - contentWidth;
        // Adjust if would overflow left edge
        if (left < 8) {
          left = 8;
        }
      } else {
        // center
        left = triggerRect.left + triggerRect.width / 2 - contentWidth / 2;
        // Adjust if would overflow either edge
        if (left < 8) {
          left = 8;
        } else if (left + contentWidth > viewportWidth - 8) {
          left = Math.max(8, viewportWidth - contentWidth - 8);
        }
      }
    } else if (side === "left") {
      left = triggerRect.left - contentWidth - gap;
      if (left < 8) {
        // Switch to right side if not enough space on left
        left = triggerRect.right + gap;
      }
    } else if (side === "right") {
      left = triggerRect.right + gap;
      if (left + contentWidth > viewportWidth - 8) {
        // Switch to left side if not enough space on right
        left = triggerRect.left - contentWidth - gap;
      }
    }

    // Calculate vertical position
    if (side === "top") {
      top = triggerRect.top - contentHeight - gap;
      if (top < 8) {
        // Switch to bottom if not enough space on top
        top = triggerRect.bottom + gap;
      }
    } else if (side === "bottom") {
      top = triggerRect.bottom + gap;
      if (top + contentHeight > viewportHeight - 8) {
        // Switch to top if not enough space on bottom
        top = triggerRect.top - contentHeight - gap;
      }
    } else if (side === "left" || side === "right") {
      if (align === "start") {
        top = triggerRect.top;
      } else if (align === "end") {
        top = triggerRect.bottom - contentHeight;
      } else {
        // center
        top = triggerRect.top + triggerRect.height / 2 - contentHeight / 2;
      }
      // Adjust if would overflow vertical edges
      if (top < 8) {
        top = 8;
      } else if (top + contentHeight > viewportHeight - 8) {
        top = Math.max(8, viewportHeight - contentHeight - 8);
      }
    }

    setPosition({ top, left });
  };

  React.useEffect(() => {
    if (isOpen) {
      calculatePosition();
    }
  }, [isOpen]);

  return (
    <span
      ref={triggerRef}
      className="relative inline-block"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
    >
      {children}
      {isOpen &&
        position &&
        createPortal(
          <div
            ref={contentRef}
            className={cn(
              "fixed z-50 w-64 max-w-[calc(100vw-2rem)] overflow-hidden rounded-md border border-border bg-popover px-3 py-2 text-popover-foreground shadow-md",
              "animate-in fade-in-0 zoom-in-95 duration-200"
            )}
            style={{
              top: `${position.top}px`,
              left: `${position.left}px`,
            }}
          >
            {content}
          </div>,
          document.body
        )}
    </span>
  );
}

export const HoverCardTrigger = HoverCard;

export const HoverCardContent = ({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) => (
  <div
    className={cn("p-2 text-sm wrap-break-word whitespace-normal", className)}
  >
    {children}
  </div>
);
