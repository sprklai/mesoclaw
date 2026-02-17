import { useEffect, useState } from "react";

import type { Breakpoint } from "@/lib/breakpoints";
import {
  BREAKPOINT_MIN_WIDTHS,
  BREAKPOINT_ORDER,
  isMobileBreakpoint,
  isDesktopBreakpoint,
} from "@/lib/breakpoints";

/**
 * Derives the current breakpoint name from a given window width.
 * Returns the largest breakpoint whose min-width is <= the given width.
 */
function getBreakpointFromWidth(width: number): Breakpoint {
  // Iterate from largest to smallest breakpoint
  for (let i = BREAKPOINT_ORDER.length - 1; i >= 0; i--) {
    const bp = BREAKPOINT_ORDER[i];
    if (width >= BREAKPOINT_MIN_WIDTHS[bp]) {
      return bp;
    }
  }
  return "xs";
}

/**
 * useBreakpoint
 *
 * A custom hook that returns the current responsive breakpoint name.
 * Updates reactively when the viewport width changes.
 *
 * Breakpoints:
 *   "xs"  — < 640px
 *   "sm"  — 640px – 767px
 *   "md"  — 768px – 1023px
 *   "lg"  — 1024px – 1279px
 *   "xl"  — >= 1280px
 *
 * @example
 * ```tsx
 * const breakpoint = useBreakpoint();
 *
 * // Conditional rendering where CSS alone isn't sufficient
 * if (breakpoint === "xs") {
 *   return <MobileOnlyComponent />;
 * }
 * ```
 */
export function useBreakpoint(): Breakpoint {
  const [breakpoint, setBreakpoint] = useState<Breakpoint>(() => {
    if (typeof window === "undefined") return "lg";
    return getBreakpointFromWidth(window.innerWidth);
  });

  useEffect(() => {
    const handleResize = () => {
      const next = getBreakpointFromWidth(window.innerWidth);
      setBreakpoint((prev) => (prev !== next ? next : prev));
    };

    window.addEventListener("resize", handleResize, { passive: true });
    // Run once on mount to ensure SSR default is corrected
    handleResize();

    return () => {
      window.removeEventListener("resize", handleResize);
    };
  }, []);

  return breakpoint;
}

/**
 * useIsMobile
 *
 * Convenience hook that returns true when the viewport is < md (768px).
 * Equivalent to the Tailwind `md:` breakpoint boundary.
 *
 * @example
 * ```tsx
 * const isMobile = useIsMobile();
 * return isMobile ? <MobileNav /> : <DesktopNav />;
 * ```
 */
export function useIsMobile(): boolean {
  const breakpoint = useBreakpoint();
  return isMobileBreakpoint(breakpoint);
}

/**
 * useIsDesktop
 *
 * Convenience hook that returns true when the viewport is >= md (768px).
 *
 * @example
 * ```tsx
 * const isDesktop = useIsDesktop();
 * ```
 */
export function useIsDesktop(): boolean {
  const breakpoint = useBreakpoint();
  return isDesktopBreakpoint(breakpoint);
}
