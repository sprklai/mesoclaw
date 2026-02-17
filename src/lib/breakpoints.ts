/**
 * Breakpoint System
 *
 * Defines the responsive breakpoints used throughout the application.
 * These align with Tailwind CSS 4's default breakpoint system.
 *
 * Breakpoints (mobile-first, min-width):
 *   xs  — < 640px   (extra small / phone portrait)
 *   sm  — >= 640px  (phone landscape / small tablet)
 *   md  — >= 768px  (tablet / small laptop)
 *   lg  — >= 1024px (desktop / large tablet landscape)
 *   xl  — >= 1280px (large desktop)
 */

/**
 * Breakpoint names as a union type for use in hooks and conditional logic.
 */
export type Breakpoint = "xs" | "sm" | "md" | "lg" | "xl";

/**
 * Minimum pixel widths for each breakpoint.
 * "xs" is the base (mobile-first) and has no minimum — it applies below `sm`.
 */
export const BREAKPOINT_MIN_WIDTHS = {
  xs: 0,
  sm: 640,
  md: 768,
  lg: 1024,
  xl: 1280,
} as const satisfies Record<Breakpoint, number>;

/**
 * Tailwind CSS responsive prefix strings for programmatic use.
 * Useful when building dynamic class names with `cn()`.
 */
export const BREAKPOINT_PREFIXES = {
  xs: "",
  sm: "sm:",
  md: "md:",
  lg: "lg:",
  xl: "xl:",
} as const satisfies Record<Breakpoint, string>;

/**
 * Media query strings for use with `window.matchMedia`.
 * These match Tailwind's default breakpoint behavior.
 */
export const BREAKPOINT_MEDIA_QUERIES = {
  xs: "(max-width: 639px)",
  sm: "(min-width: 640px) and (max-width: 767px)",
  md: "(min-width: 768px) and (max-width: 1023px)",
  lg: "(min-width: 1024px) and (max-width: 1279px)",
  xl: "(min-width: 1280px)",
} as const satisfies Record<Breakpoint, string>;

/**
 * Ordered breakpoints from smallest to largest.
 * Used for iterating from mobile up.
 */
export const BREAKPOINT_ORDER: readonly Breakpoint[] = [
  "xs",
  "sm",
  "md",
  "lg",
  "xl",
] as const;

/**
 * Returns true if the given breakpoint is considered "mobile" (< md).
 */
export const isMobileBreakpoint = (bp: Breakpoint): boolean =>
  bp === "xs" || bp === "sm";

/**
 * Returns true if the given breakpoint is considered "desktop" (>= md).
 */
export const isDesktopBreakpoint = (bp: Breakpoint): boolean =>
  bp === "md" || bp === "lg" || bp === "xl";
