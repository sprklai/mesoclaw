import { useCallback, useState } from "react";

import type { SidebarItem } from "@/components/ui/sidebar/sidebar-nav-item";

import { Button } from "@/components/ui/button";
import { SidebarHeader } from "@/components/ui/sidebar/sidebar-header";
import { SidebarNav } from "@/components/ui/sidebar/sidebar-nav";
import { Menu, X } from "@/lib/icons";

interface SidebarMobileProps {
  expanded: boolean;
  onToggle: () => void;
  topItems: readonly SidebarItem[];
  bottomItems: readonly SidebarItem[];
}

export function SidebarMobile({
  expanded,
  onToggle,
  topItems,
  bottomItems,
}: SidebarMobileProps) {
  const [mobileOpen, setMobileOpen] = useState(false);

  const openMobileSidebar = useCallback(() => {
    setMobileOpen(true);
  }, []);

  const closeMobileSidebar = useCallback(() => {
    setMobileOpen(false);
  }, []);

  return (
    <>
      <MobileMenuButton onClick={openMobileSidebar} />
      {mobileOpen && (
        <>
          <MobileOverlay onClick={closeMobileSidebar} />
          <MobileSidebarPanel
            expanded={expanded}
            onToggle={onToggle}
            topItems={topItems}
            bottomItems={bottomItems}
            onClose={closeMobileSidebar}
          />
        </>
      )}
    </>
  );
}

interface MobileMenuButtonProps {
  onClick: () => void;
}

function MobileMenuButton({ onClick }: MobileMenuButtonProps) {
  return (
    <Button
      variant="ghost"
      size="icon"
      onClick={onClick}
      className="fixed left-3 top-3 z-40 md:hidden"
      aria-label="Open menu"
    >
      <Menu className="size-5" />
    </Button>
  );
}

interface MobileOverlayProps {
  onClick: () => void;
}

function MobileOverlay({ onClick }: MobileOverlayProps) {
  return (
    <div
      className="fixed inset-0 z-40 bg-background/80 backdrop-blur-sm md:hidden"
      onClick={onClick}
      onKeyDown={(e) => e.key === "Escape" && onClick()}
    />
  );
}

interface MobileSidebarPanelProps {
  expanded: boolean;
  onToggle: () => void;
  topItems: readonly SidebarItem[];
  bottomItems: readonly SidebarItem[];
  onClose: () => void;
}

function MobileSidebarPanel({
  expanded,
  onToggle,
  topItems,
  bottomItems,
  onClose,
}: MobileSidebarPanelProps) {
  return (
    <aside className="fixed inset-y-0 left-0 z-50 flex w-64 flex-col border-r border-sidebar-border bg-sidebar md:hidden">
      <Button
        variant="ghost"
        size="icon"
        onClick={onClose}
        className="absolute right-2 top-2 z-50"
        aria-label="Close menu"
      >
        <X className="size-5" />
      </Button>

      <SidebarHeader expanded={expanded} onToggle={onToggle} />
      <SidebarNav items={topItems} expanded={expanded} onItemClick={onClose} />
      <SidebarNav
        items={bottomItems}
        expanded={expanded}
        variant="bottom"
        onItemClick={onClose}
      />
    </aside>
  );
}
