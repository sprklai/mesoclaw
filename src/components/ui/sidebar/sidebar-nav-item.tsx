import { Link, useLocation } from "@tanstack/react-router";

import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

export interface SidebarItem {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  href: string;
}

interface SidebarNavItemProps {
  item: SidebarItem;
  expanded: boolean;
  onClick?: () => void;
}

export function SidebarNavItem({
  item,
  expanded,
  onClick,
}: SidebarNavItemProps) {
  const Icon = item.icon;
  const location = useLocation();
  const isActive = location.pathname === item.href;

  const linkContent = (
    <Link
      to={item.href}
      onClick={onClick}
      className={cn(
        "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
        "w-full text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
        expanded
          ? "h-10 justify-start gap-3 px-3"
          : "h-10 w-10 justify-center px-0",
        isActive && "bg-sidebar-accent text-sidebar-accent-foreground"
      )}
    >
      <Icon className="size-5 shrink-0" />
      {expanded && <span className="truncate">{item.label}</span>}
    </Link>
  );

  if (!expanded) {
    return <Tooltip content={item.label}>{linkContent}</Tooltip>;
  }

  return linkContent;
}
