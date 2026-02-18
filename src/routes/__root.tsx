import { Outlet, createRootRoute } from "@tanstack/react-router";

import { MobileNav } from "@/components/layout/MobileNav";
import { Sidebar } from "@/components/ui/sidebar";
import { APP_IDENTITY } from "@/config/app-identity";
import { GatewayStatus } from "@/components/ui/gateway-status";
import { useMobileSwipe } from "@/hooks/useMobileSwipe";
import { useVirtualKeyboard } from "@/hooks/useVirtualKeyboard";
import { useSidebarStore } from "@/stores/sidebarStore";

export const Route = createRootRoute({
  component: RootLayout,
});

function RootLayout() {
  const openMobile = useSidebarStore((state) => state.openMobile);
  const closeMobile = useSidebarStore((state) => state.closeMobile);

  // Swipe right from the left edge → open sidebar; swipe left → close it.
  const { onTouchStart, onTouchEnd } = useMobileSwipe({
    onSwipeRight: openMobile,
    onSwipeLeft: closeMobile,
  });

  // Track the software keyboard height so the CSS variable `--keyboard-height`
  // is always up-to-date for layout compensation (e.g. chat input area).
  useVirtualKeyboard();

  return (
    /*
     * Responsive root layout — adapts across all breakpoints:
     *
     *   xs / sm (< 768px)  → Single column, MobileNav fixed at bottom
     *   md      (768-1279)  → 2-column: [256px sidebar | 1fr main]
     *   xl      (>= 1280px) → 3-column: [256px sidebar | 1fr main | 320px panel]
     *
     * The third column (xl) is reserved for future contextual panels (e.g. AI
     * assistant, schema detail view). It renders as an empty slot until content
     * is passed via a route-level outlet or context.
     */
    // biome-ignore lint/a11y/noNoninteractiveElementToInteractiveRole: touch handlers on root layout div are for gesture detection, not interactive semantics
    <div
      className="flex h-screen flex-col overflow-hidden md:flex-row"
      onTouchStart={onTouchStart}
      onTouchEnd={onTouchEnd}
    >
      {/*
       * Sidebar: hidden on xs/sm (< md), shown as persistent sidebar on md+.
       * On mobile the Sidebar component renders a floating drawer instead.
       */}
      <Sidebar />

      {/*
       * Main content area
       * - Takes all remaining horizontal space on md/lg
       * - On xl, leaves room for the right panel via the parent grid approach
       */}
      <main className="flex min-w-0 flex-1 flex-col overflow-hidden">
        {/* ── Topbar / title bar ────────────────────────────────── */}
        <div
          data-tauri-drag-region
          className="flex h-14 shrink-0 items-center justify-between border-b border-border px-4"
        >
          <div className="flex items-center gap-3">
            <img
              src={APP_IDENTITY.iconAssetPath}
              alt={`${APP_IDENTITY.productName} icon`}
              className="h-7 w-7"
              draggable={false}
            />
            <span className="text-xl font-bold">{APP_IDENTITY.productName}</span>
          </div>
          <GatewayStatus />
        </div>

        {/*
         * Scrollable page content.
         * pb-16 on mobile to avoid content being obscured by the MobileNav bar.
         * md:pb-0 removes that padding on desktop where MobileNav is hidden.
         */}
        <div className="flex-1 overflow-auto p-4 pb-20 pt-6 md:p-6 md:pb-6">
          <Outlet />
        </div>
      </main>

      {/*
       * Right contextual panel — xl breakpoint only (>= 1280px).
       * Currently renders empty; route-level components can populate this
       * via a future named outlet or portal mechanism.
       * Width: 320px fixed (xl:w-80)
       */}
      <aside className="hidden border-l border-border xl:flex xl:w-80 xl:flex-col">
        {/* Future: contextual panel content (schema details, AI assistant, etc.) */}
      </aside>

      {/*
       * Mobile bottom navigation bar.
       * Visible only on xs/sm (< md). Fixed to bottom of viewport.
       */}
      <MobileNav />
    </div>
  );
}
