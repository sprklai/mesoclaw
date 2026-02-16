import type { ComponentProps } from "react";

import { cn } from "@/lib/utils";

type LabelProps = ComponentProps<"label">;

export function Label({ className, ...props }: LabelProps) {
  return (
    <label
      className={cn(
        "text-sm font-medium leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70",
        className
      )}
      {...props}
    />
  );
}
