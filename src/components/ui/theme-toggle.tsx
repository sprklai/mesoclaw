import { Monitor, Moon, Sun } from "lucide-react";
import { Tooltip } from "@/components/ui/tooltip";
import { useTheme } from "@/stores/theme";
import type { Theme } from "@/lib/tauri/settings/types";
import { cn } from "@/lib/utils";

const MODES: { value: Theme; label: string; icon: typeof Sun }[] = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System (OS)", icon: Monitor },
];

interface ThemeToggleProps {
  compact?: boolean;
}

export function ThemeToggle({ compact = false }: ThemeToggleProps) {
  const { theme, setTheme } = useTheme();

  if (compact) {
    const currentIndex = MODES.findIndex((m) => m.value === theme);
    const next = MODES[(currentIndex + 1) % MODES.length];
    const Current = MODES[currentIndex]?.icon ?? Sun;

    return (
      <Tooltip content={`Theme: ${MODES[currentIndex]?.label ?? "System"} — click to change`}>
        <button
          type="button"
          onClick={() => void setTheme(next.value)}
          className={cn(
            "flex min-h-[44px] min-w-[44px] items-center justify-center rounded-lg",
            "text-sidebar-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          )}
          aria-label={`Current theme: ${MODES[currentIndex]?.label}. Click to switch to ${next.label}`}
        >
          <Current className="size-4" aria-hidden />
        </button>
      </Tooltip>
    );
  }

  return (
    <div
      className="flex rounded-lg border border-sidebar-border bg-sidebar p-0.5"
      role="group"
      aria-label="Theme mode"
    >
      {MODES.map(({ value, label, icon: Icon }) => (
        <Tooltip key={value} content={label}>
          <button
            type="button"
            onClick={() => void setTheme(value)}
            aria-label={label}
            aria-pressed={theme === value}
            className={cn(
              "flex flex-1 items-center justify-center rounded-md py-1.5 text-xs transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              theme === value
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            <Icon className="size-3.5" aria-hidden />
          </button>
        </Tooltip>
      ))}
    </div>
  );
}
