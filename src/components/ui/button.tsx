import type { VariantProps } from "class-variance-authority";
import type { ComponentProps } from "react";

import { Button as BaseButton } from "@base-ui/react/button";
import { cva } from "class-variance-authority";

import { cn } from "@/lib/utils";

const buttonVariants = cva(
  [
    "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors",
    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
    "disabled:pointer-events-none disabled:opacity-50",
    "[&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0",
  ],
  {
    variants: {
      variant: {
        default:
          "bg-primary text-primary-foreground hover:bg-primary/90 active:bg-primary/80",
        ghost:
          "hover:bg-accent hover:text-accent-foreground active:bg-accent/80",
        outline:
          "border border-input bg-background hover:bg-accent hover:text-accent-foreground active:bg-accent/80",
        secondary:
          "bg-secondary text-secondary-foreground hover:bg-secondary/80 active:bg-secondary/70",
        destructive:
          "bg-destructive text-destructive-foreground hover:bg-destructive/90 active:bg-destructive/80",
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-10 px-4 py-2",
        sm: "h-9 px-3 text-xs",
        lg: "h-11 px-8",
        icon: "h-10 w-10",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  }
);

type ButtonProps = ComponentProps<typeof BaseButton> &
  VariantProps<typeof buttonVariants> & {
    className?: string;
  };

/**
 * Button component with accessibility support.
 *
 * ## Accessibility
 *
 * ### Icon-Only Buttons
 * When using the `size="icon"` variant with no text content, you MUST provide
 * an `aria-label` prop to make the button accessible to screen reader users:
 *
 * @example
 * ```tsx
 * // ✅ Correct - Icon button with aria-label
 * <Button size="icon" aria-label="Close dialog">
 *   <X className="h-4 w-4" />
 * </Button>
 *
 * // ❌ Incorrect - Icon button without aria-label
 * <Button size="icon">
 *   <X className="h-4 w-4" />
 * </Button>
 * ```
 *
 * ### Decorative Icons
 * When icons are purely decorative and already explained by surrounding text,
 * mark them with `aria-hidden="true"` to hide from screen readers:
 *
 * @example
 * ```tsx
 * <Button>
 *   <Search aria-hidden="true" className="h-4 w-4" />
 *   <span>Search</span>
 * </Button>
 * ```
 *
 * ## Keyboard Navigation
 * - Buttons are focusable and can be activated with Enter/Space
 * - Focus styles are automatically applied via `focus-visible:ring-*` classes
 * - Tab order follows DOM order
 *
 * @param {ButtonProps} props - Component props
 * @returns {JSX.Element} Accessible button element
 */
export function Button({ className, variant, size, ...props }: ButtonProps) {
  return (
    <BaseButton
      className={cn(buttonVariants({ variant, size, className }))}
      {...props}
    />
  );
}

export { buttonVariants };
