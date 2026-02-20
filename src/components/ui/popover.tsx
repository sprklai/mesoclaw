import type { ComponentProps } from "react";

import { Popover as BasePopover } from "@base-ui/react/popover";
import { forwardRef } from "react";

import { cn } from "@/lib/utils";

const Popover = BasePopover.Root;

const PopoverTrigger = forwardRef<
  HTMLButtonElement,
  ComponentProps<typeof BasePopover.Trigger>
>(({ className, ...props }, ref) => (
  <BasePopover.Trigger
    ref={ref}
    className={cn("outline-none", className)}
    {...props}
  />
));
PopoverTrigger.displayName = "PopoverTrigger";

const PopoverContent = forwardRef<
  HTMLDivElement,
  ComponentProps<typeof BasePopover.Popup> & {
    align?: "start" | "center" | "end";
  }
>(({ className, align = "center", ...props }, ref) => (
  <BasePopover.Portal>
    <BasePopover.Positioner align={align} sideOffset={4}>
      <BasePopover.Popup
        ref={ref}
        className={cn(
          "z-50 w-72 rounded-md border bg-popover p-4 text-popover-foreground shadow-md outline-none",
          "data-[state=open]:animate-in data-[state=closed]:animate-out",
          "data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0",
          "data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
          "data-[side=bottom]:slide-in-from-top-2",
          "data-[side=left]:slide-in-from-right-2",
          "data-[side=right]:slide-in-from-left-2",
          "data-[side=top]:slide-in-from-bottom-2",
          className
        )}
        {...props}
      />
    </BasePopover.Positioner>
  </BasePopover.Portal>
));
PopoverContent.displayName = "PopoverContent";

export { Popover, PopoverTrigger, PopoverContent };
