import type { SidebarItem } from "@/components/ui/sidebar/sidebar-nav-item";

import { SidebarNavItem } from "@/components/ui/sidebar/sidebar-nav-item";

interface SidebarNavProps {
  items: readonly SidebarItem[];
  expanded: boolean;
  variant?: "top" | "bottom";
  onItemClick?: () => void;
}

export function SidebarNav({
  items,
  expanded,
  variant = "top",
  onItemClick,
}: SidebarNavProps) {
  const isBottom = variant === "bottom";

  return (
    <nav
      className={`flex flex-col gap-1 p-3 ${isBottom ? "border-t border-sidebar-border" : "flex-1"}`}
    >
      {items.map((item) => (
        <SidebarNavItem
          key={item.label}
          item={item}
          expanded={expanded}
          onClick={onItemClick}
        />
      ))}
    </nav>
  );
}
