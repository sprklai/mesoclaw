import { useCallback, useEffect, useState } from "react";

import { SidebarHeader } from "@/components/ui/sidebar/sidebar-header";
import { SidebarMobile } from "@/components/ui/sidebar/sidebar-mobile";
import { SidebarNav } from "@/components/ui/sidebar/sidebar-nav";
import type { SidebarItem } from "@/components/ui/sidebar/sidebar-nav-item";
import { SidebarSkeleton } from "@/components/ui/sidebar/sidebar-skeleton";
import { SIDEBAR_BOTTOM_ITEMS } from "@/constants/sidebar";
import { useAsyncAction } from "@/hooks/use-async-action";
import { Brain, Home, MessageSquare, Sparkles } from "@/lib/icons";
import { updateSettings } from "@/lib/tauri/settings";
import { cn } from "@/lib/utils";
import { useSettings } from "@/stores/settings";

export function Sidebar() {
  const settings = useSettings((state) => state.settings);
  const isLoading = useSettings((state) => state.isLoading);

  const [expanded, setExpanded] = useState(true);
  const [withSaving] = useAsyncAction();

  useEffect(() => {
    if (settings) {
      setExpanded(settings.sidebarExpanded);
    }
  }, [settings]);

  const toggleExpanded = useCallback(async () => {
    const previousExpanded = expanded;
    const newExpanded = !expanded;
    setExpanded(newExpanded);

    await withSaving(
      async () => {
        await updateSettings({ sidebarExpanded: newExpanded });
      },
      {
        onError: () => setExpanded(previousExpanded),
        errorMessage: "Failed to persist sidebar state",
      }
    );
  }, [expanded, withSaving]);

  const navItems: readonly SidebarItem[] = [
    {
      icon: Home,
      label: "Home",
      href: "/",
    },
    {
      icon: Sparkles,
      label: "AI Chat",
      href: "/chat",
    },
    {
      icon: Brain,
      label: "Memory",
      href: "/memory",
    },
    {
      icon: MessageSquare,
      label: "Channels",
      href: "/channels",
    },
  ] as const;

  if (isLoading) {
    return <SidebarSkeleton />;
  }

  return (
    <>
      <SidebarMobile
        onToggle={toggleExpanded}
        expanded={expanded}
        topItems={navItems}
        bottomItems={SIDEBAR_BOTTOM_ITEMS}
      />
      <aside
        className={cn(
          "hidden md:flex h-full flex-col border-r border-border bg-background transition-all duration-300",
          expanded ? "w-64" : "w-16"
        )}
      >
        <SidebarHeader expanded={expanded} onToggle={toggleExpanded} />

        <SidebarNav items={navItems} expanded={expanded} />

        <SidebarNav
          items={SIDEBAR_BOTTOM_ITEMS}
          expanded={expanded}
          variant="bottom"
        />
      </aside>
    </>
  );
}
