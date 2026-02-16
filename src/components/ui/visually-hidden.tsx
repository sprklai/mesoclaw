import { cn } from "@/lib/utils";

interface VisuallyHiddenProps {
  children: React.ReactNode;
  className?: string;
}

/**
 * VisuallyHidden - Content that is screen reader only, not visible visually
 *
 * Use for:
 * - Icon button labels (when visual icon is self-explanatory to sighted users)
 * - Additional context for screen reader users
 * - Form labels that are visually implied
 *
 * @example
 * <button>
 *   <Search aria-hidden="true" />
 *   <VisuallyHidden>Search</VisuallyHidden>
 * </button>
 */
export function VisuallyHidden({ children, className }: VisuallyHiddenProps) {
  return (
    <span
      className={cn(
        "sr-only absolute w-px h-px p-0 -m-px overflow-hidden whitespace-nowrap border-0",
        className
      )}
    >
      {children}
    </span>
  );
}
