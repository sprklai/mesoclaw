import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

import { AdvancedSettingsTab } from "@/components/settings/AdvancedSettingsTab";
import { AISettingsTab } from "@/components/settings/AISettingsTab";
import { AppSettingsTab } from "@/components/settings/AppSettingsTab";
import { ChannelList } from "@/components/settings/ChannelList";
import { IdentityEditor } from "@/components/settings/IdentityEditor";
import { JobList } from "@/components/settings/JobList";
import { MobileSettings } from "@/components/settings/MobileSettings";
import { ModuleList } from "@/components/settings/ModuleList";
import { SettingsNav, type SettingsSection } from "@/components/settings/SettingsNav";
import { SkillsSettingsTab } from "@/components/settings/SkillsSettingsTab";
import { PageHeader } from "@/components/layout/PageHeader";
import { useHandleSettings } from "@/hooks/use-handle-settings";

export const Route = createFileRoute("/settings")({
  validateSearch: (search) => ({
    tab: (search.tab as string) ?? "ai",
  }),
  component: SettingsPage,
});

const SETTINGS_SECTIONS: SettingsSection[] = [
  { id: "ai", label: "AI Provider", description: "Providers, models, API keys" },
  { id: "skills", label: "Skills", description: "Prompt templates" },
  { id: "app", label: "App Settings", description: "Theme, autostart, notifications" },
  { id: "identity", label: "Identity", description: "Agent name and personality" },
  { id: "scheduler", label: "Scheduler", description: "Scheduled jobs" },
  { id: "modules", label: "Modules", description: "Sidecar modules" },
  { id: "channels", label: "Channels", description: "Telegram and other channels" },
  { id: "mobile", label: "Mobile", description: "Mobile-specific settings" },
  { id: "advanced", label: "Advanced", description: "Developer and advanced options" },
];

function SettingsPage() {
  const { tab } = Route.useSearch();
  const [activeSection, setActiveSection] = useState(tab ?? "ai");

  const {
    theme,
    settings,
    autostartEnabled,
    isLoading,
    isSaving,
    handleUpdateSetting,
    handleAutostartChange,
    handleTrayVisibilityChange,
    handleNotificationChange,
  } = useHandleSettings();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-12">
        <p className="text-muted-foreground">Loading settings…</p>
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="flex items-center justify-center p-12">
        <p className="text-destructive">Failed to load settings</p>
      </div>
    );
  }

  const activeLabel =
    SETTINGS_SECTIONS.find((s) => s.id === activeSection)?.label ?? "Settings";

  return (
    <div className="mx-auto w-full max-w-5xl">
      <PageHeader title="Settings" description="Configure your AI providers and application" />

      <div className="flex gap-8">
        {/* Left nav */}
        <div className="w-full shrink-0 md:w-48">
          <SettingsNav
            sections={SETTINGS_SECTIONS}
            activeSection={activeSection}
            onSectionChange={setActiveSection}
          />
        </div>

        {/* Content area */}
        <div className="min-w-0 flex-1">
          <h2 className="mb-6 text-lg font-semibold">{activeLabel}</h2>

          {activeSection === "ai" && <AISettingsTab />}
          {activeSection === "skills" && <SkillsSettingsTab />}
          {activeSection === "app" && (
            <AppSettingsTab
              theme={theme}
              settings={settings}
              isSaving={isSaving}
              onUpdateSetting={handleUpdateSetting}
              onAutostartChange={handleAutostartChange}
              onTrayVisibilityChange={handleTrayVisibilityChange}
              onNotificationChange={handleNotificationChange}
              autostartEnabled={autostartEnabled}
            />
          )}
          {activeSection === "identity" && <IdentityEditor />}
          {activeSection === "scheduler" && <JobList />}
          {activeSection === "modules" && <ModuleList />}
          {activeSection === "channels" && <ChannelList />}
          {activeSection === "mobile" && <MobileSettings />}
          {activeSection === "advanced" && <AdvancedSettingsTab />}
        </div>
      </div>
    </div>
  );
}
