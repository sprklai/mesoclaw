import { useCallback, useEffect, useRef, type ReactNode } from "react";

import { X } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface ResponsiveSidebarProps {
  /** Content to render inside the sidebar */
  children: ReactNode;
  /** Whether the mobile drawer is open (ignored on desktop) */
  mobileOpen: boolean;
  /** Called when the mobile drawer should close */
  onMobileClose: () => void;
  /** Optional additional className for the sidebar panel */
  className?: string;
}

/**
 * ResponsiveSidebar
 *
 * A sidebar component that adapts its behavior based on viewport width:
 *
 * - **Desktop (>= md / 768px)**: Renders as a persistent 256px sidebar on
 *   the left side of the layout. Always visible, no overlay.
 *
 * - **Mobile (< md)**: Renders as a slide-in drawer from the left edge,
 *   controlled by the `mobileOpen` / `onMobileClose` props. Includes a
 *   semi-transparent overlay backdrop and traps focus while open.
 *
 * The hamburger button to open the drawer on mobile is provided separately
 * (e.g. in the header bar) and passes `mobileOpen` state down here.
 *
 * @example
 * ```tsx
 * const [drawerOpen, setDrawerOpen] = useState(false);
 *
 * <ResponsiveSidebar
 *   mobileOpen={drawerOpen}
 *   onMobileClose={() => setDrawerOpen(false)}
 * >
 *   <SidebarNav />
 * </ResponsiveSidebar>
 * ```
 */
export function ResponsiveSidebar({
  children,
  mobileOpen,
  onMobileClose,
  className,
}: ResponsiveSidebarProps) {
  const sidebarRef = useRef<HTMLElement>(null);

  // Close on Escape key
  useEffect(() => {
    if (!mobileOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onMobileClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [mobileOpen, onMobileClose]);

  // Lock body scroll when drawer is open on mobile
  useEffect(() => {
    if (mobileOpen) {
      document.body.style.overflow = "hidden";
    } else {
      document.body.style.overflow = "";
    }
    return () => {
      document.body.style.overflow = "";
    };
  }, [mobileOpen]);

  const handleOverlayClick = useCallback(() => {
    onMobileClose();
  }, [onMobileClose]);

  return (
    <>
      {/* ============================================================
          MOBILE: Overlay backdrop (only when drawer is open)
          ============================================================ */}
      {mobileOpen && (
        <div
          className="fixed inset-0 z-40 bg-background/80 backdrop-blur-sm md:hidden"
          onClick={handleOverlayClick}
          onKeyDown={(e) => e.key === "Escape" && handleOverlayClick()}
          aria-hidden="true"
        />
      )}

      {/* ============================================================
          SIDEBAR PANEL
          - Mobile: fixed slide-in drawer from left
          - Desktop: static 256px sidebar (always visible)
          ============================================================ */}
      <aside
        ref={sidebarRef}
        className={cn(
          // ── Desktop layout (md and up) ──────────────────────────
          // Persistent sidebar, always visible, 256px wide
          "hidden md:flex md:h-full md:w-64 md:flex-col",
          "md:border-r md:border-border md:bg-background",

          // ── Mobile layout ───────────────────────────────────────
          // Fixed position drawer that slides in from the left
          mobileOpen && [
            "fixed inset-y-0 left-0 z-50",
            "flex flex-col w-64",
            "border-r border-border bg-background",
            "shadow-xl",
          ],

          className,
        )}
        aria-label="Primary navigation"
        // On mobile, use dialog role when drawer is open
        role={mobileOpen ? "dialog" : undefined}
        aria-modal={mobileOpen ? "true" : undefined}
      >
        {/* Close button — only shown on mobile when drawer is open */}
        {mobileOpen && (
          <Button
            variant="ghost"
            size="icon"
            onClick={onMobileClose}
            className="absolute right-2 top-2 z-50 md:hidden"
            aria-label="Close navigation drawer"
          >
            <X className="size-5" />
          </Button>
        )}

        {children}
      </aside>
    </>
  );
}
