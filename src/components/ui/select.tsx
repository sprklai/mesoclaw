import type { ComponentProps } from "react";

import { Select as BaseSelect } from "@base-ui/react/select";

import { Check, ChevronDown } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface SelectOption<T extends string = string> {
  value: T;
  label: string;
}

interface SelectProps<T extends string = string> {
  value: T;
  onValueChange: (value: T) => void;
  options: SelectOption<T>[];
  placeholder?: string;
  className?: string;
  disabled?: boolean;
}

export function Select<T extends string = string>({
  value,
  onValueChange,
  options,
  placeholder = "Select an option",
  className,
  disabled = false,
}: SelectProps<T>) {
  const selectedOption = options.find((opt) => opt.value === value);

  const handleValueChange = (newValue: T | null) => {
    if (newValue !== null) {
      onValueChange(newValue);
    }
  };

  return (
    <BaseSelect.Root
      value={value}
      onValueChange={handleValueChange}
      disabled={disabled}
    >
      <BaseSelect.Trigger
        className={cn(
          "inline-flex h-10 w-full items-center justify-between gap-2 rounded-md border border-input bg-popover px-3 py-2 text-sm text-popover-foreground",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
          "disabled:cursor-not-allowed disabled:opacity-50",
          "shadow-sm",
          className
        )}
      >
        <BaseSelect.Value className="truncate text-foreground">
          {selectedOption?.label ?? placeholder}
        </BaseSelect.Value>
        <BaseSelect.Icon className="text-muted-foreground">
          <ChevronDown className="size-4" />
        </BaseSelect.Icon>
      </BaseSelect.Trigger>
      <BaseSelect.Portal>
        <BaseSelect.Positioner className="z-50">
          <BaseSelect.Popup
            className={cn(
              "min-w-[200px] overflow-hidden rounded-md border border-border bg-popover p-1.5 text-popover-foreground shadow-lg",
              "animate-in fade-in-0 zoom-in-95",
              "will-change-transform"
            )}
          >
            {options.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </BaseSelect.Popup>
        </BaseSelect.Positioner>
      </BaseSelect.Portal>
    </BaseSelect.Root>
  );
}

interface SelectItemProps extends ComponentProps<typeof BaseSelect.Item> {
  children: React.ReactNode;
}

function SelectItem({ children, className, ...props }: SelectItemProps) {
  return (
    <BaseSelect.Item
      className={cn(
        "relative flex w-full cursor-pointer select-none items-center rounded-sm px-2 py-2 text-sm outline-none",
        "focus:bg-accent focus:text-accent-foreground",
        "data-disabled:pointer-events-none data-disabled:opacity-50",
        "transition-colors",
        className
      )}
      {...props}
    >
      <span className="absolute left-2 flex size-4 items-center justify-center">
        <BaseSelect.ItemIndicator>
          <Check className="size-4" />
        </BaseSelect.ItemIndicator>
      </span>
      <BaseSelect.ItemText>{children}</BaseSelect.ItemText>
    </BaseSelect.Item>
  );
}

export type { SelectOption, SelectProps };
