import type { ComponentType } from "react";

import { Link, useLocation } from "@tanstack/react-router";
import { Home, Settings, Sparkles } from "lucide-react";

import { cn } from "@/lib/utils";

interface MobileNavItem {
  icon: ComponentType<{ className?: string; "aria-hidden"?: boolean }>;
  label: string;
  href: string;
}

const MOBILE_NAV_ITEMS: MobileNavItem[] = [
  { icon: Home, label: "Home", href: "/" },
  { icon: Sparkles, label: "AI Chat", href: "/chat" },
  { icon: Settings, label: "Settings", href: "/settings" },
];

/**
 * MobileNav
 *
 * Fixed bottom navigation bar visible only on mobile (< md breakpoint).
 * Provides primary navigation with minimum 44x44px touch targets per
 * WCAG 2.5.8 / Apple HIG guidelines.
 *
 * Hidden on md and larger screens where the persistent sidebar is shown instead.
 */
export function MobileNav() {
  const location = useLocation();

  return (
    <nav
      className={cn(
        // Visible only on mobile (< 768px)
        "fixed bottom-0 left-0 right-0 z-50 md:hidden",
        // Background and border
        "border-t border-border bg-background",
      )}
      aria-label="Mobile navigation"
    >
      {/* Safe area bottom padding using env() */}
      <div
        className="flex items-stretch justify-around"
        style={{ paddingBottom: "env(safe-area-inset-bottom)" }}
      >
        {MOBILE_NAV_ITEMS.map((item) => {
          const Icon = item.icon;
          const isActive = location.pathname === item.href;

          return (
            <Link
              key={item.href}
              to={item.href}
              className={cn(
                // Minimum 44x44px touch target
                "flex min-h-[44px] min-w-[44px] flex-1 flex-col items-center justify-center gap-1 px-1 py-2",
                // Transition for active state
                "transition-colors",
                // Active vs inactive colors
                isActive
                  ? "text-primary"
                  : "text-muted-foreground hover:text-foreground",
              )}
              aria-label={item.label}
              aria-current={isActive ? "page" : undefined}
            >
              <Icon
                aria-hidden
                className={cn(
                  "size-5 shrink-0",
                  isActive && "scale-110 transition-transform",
                )}
              />
              <span className="text-[10px] font-medium leading-none">
                {item.label}
              </span>
            </Link>
          );
        })}
      </div>
    </nav>
  );
}
