import { cn } from "@/lib/utils";

export interface SettingsSection {
  id: string;
  label: string;
  description?: string;
}

interface SettingsNavProps {
  sections: SettingsSection[];
  activeSection: string;
  onSectionChange: (id: string) => void;
}

export function SettingsNav({
  sections,
  activeSection,
  onSectionChange,
}: SettingsNavProps) {
  return (
    <>
      {/* Desktop: vertical nav list */}
      <nav aria-label="Settings sections" className="hidden flex-col gap-0.5 md:flex">
        {sections.map((section) => {
          const isActive = section.id === activeSection;
          return (
            <button
              key={section.id}
              type="button"
              onClick={() => onSectionChange(section.id)}
              aria-current={isActive ? "page" : undefined}
              className={cn(
                "w-full rounded-lg px-3 py-2 text-left text-sm font-medium transition-colors",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                isActive
                  ? "bg-primary/10 text-primary"
                  : "text-foreground hover:bg-accent hover:text-accent-foreground"
              )}
            >
              {section.label}
            </button>
          );
        })}
      </nav>

      {/* Mobile: select dropdown */}
      <div className="md:hidden">
        <label htmlFor="settings-section-select" className="sr-only">
          Settings section
        </label>
        <select
          id="settings-section-select"
          value={activeSection}
          onChange={(e) => onSectionChange(e.target.value)}
          className={cn(
            "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm",
            "focus:outline-none focus:ring-2 focus:ring-ring"
          )}
        >
          {sections.map((section) => (
            <option key={section.id} value={section.id}>
              {section.label}
            </option>
          ))}
        </select>
      </div>
    </>
  );
}
