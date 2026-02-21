import { Button } from "@/components/ui/button";
import { Tooltip } from "@/components/ui/tooltip";
import { APP_IDENTITY } from "@/config/app-identity";
import { PanelLeft, PanelLeftClose } from "@/lib/icons";
import { useAppIdentity } from "@/stores/appSettingsStore";
import { cn } from "@/lib/utils";

interface SidebarHeaderProps {
  expanded: boolean;
  onToggle: () => void;
}

export function SidebarHeader({ expanded, onToggle }: SidebarHeaderProps) {
  const identity = useAppIdentity();

  return (
    <div
      data-tauri-drag-region
      className={cn(
        "flex h-14 items-center gap-2 border-b border-sidebar-border px-3",
        expanded ? "justify-between" : "justify-center"
      )}
    >
      <div className="flex items-center gap-2">
        <img
          src={APP_IDENTITY.iconAssetPath}
          alt=""
          className="size-7 shrink-0"
          draggable={false}
        />
        {expanded && (
          <span className="truncate font-semibold text-sidebar-foreground">
            {identity.productName}
          </span>
        )}
      </div>
      <Tooltip content={expanded ? "Collapse sidebar" : "Expand sidebar"}>
        <Button
          variant="ghost"
          size="icon"
          onClick={onToggle}
          aria-label={expanded ? "Collapse sidebar" : "Expand sidebar"}
          className="hidden min-h-[44px] min-w-[44px] shrink-0 text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground md:inline-flex"
        >
          {expanded ? (
            <PanelLeftClose className="size-5" />
          ) : (
            <PanelLeft className="size-5" />
          )}
        </Button>
      </Tooltip>
    </div>
  );
}
