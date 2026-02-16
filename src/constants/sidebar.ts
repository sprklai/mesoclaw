import type { SidebarItem } from "@/components/ui/sidebar/sidebar-nav-item";

import { Settings } from "@/lib/icons";

export const SIDEBAR_BOTTOM_ITEMS: SidebarItem[] = [
  { icon: Settings, label: "Settings", href: "/settings" },
];
