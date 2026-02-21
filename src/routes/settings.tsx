import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { AdvancedSettingsTab } from "@/components/settings/AdvancedSettingsTab";
import { AISettingsTab } from "@/components/settings/AISettingsTab";
import { AppSettingsTab } from "@/components/settings/AppSettingsTab";
import { ChannelList } from "@/components/settings/ChannelList";
import { IdentityEditor } from "@/components/settings/IdentityEditor";
import { JobList } from "@/components/settings/JobList";
import { MobileSettings } from "@/components/settings/MobileSettings";
import { ModuleList } from "@/components/settings/ModuleList";
import { ProfileSettings } from "@/components/settings/ProfileSettings";
import { RouterSettings } from "@/components/settings/RouterSettings";
import { SettingsNav, type SettingsSection } from "@/components/settings/SettingsNav";
import { SkillsSettingsTab } from "@/components/settings/SkillsSettingsTab";
import { Button } from "@/components/ui/button";
import { Bot, ExternalLink } from "@/lib/icons";
import { PageHeader } from "@/components/layout/PageHeader";
import { Link } from "@tanstack/react-router";
import { useHandleSettings } from "@/hooks/use-handle-settings";
import { useContextPanelStore } from "@/stores/contextPanelStore";

export const Route = createFileRoute("/settings")({
  validateSearch: (search) => ({
    tab: (search.tab as string) ?? "ai",
  }),
  component: SettingsPage,
});

const SETTINGS_SECTIONS: SettingsSection[] = [
  { id: "ai", label: "AI Provider", description: "Providers, models, API keys" },
  { id: "router", label: "Router", description: "Model routing and discovery" },
  { id: "profile", label: "Profile", description: "Your name and bot name" },
  { id: "skills", label: "Skills", description: "Prompt templates" },
  { id: "agents", label: "Agents", description: "Agent configurations" },
  { id: "app", label: "App Settings", description: "Theme, autostart, notifications" },
  { id: "identity", label: "Agent Personality", description: "SOUL.md and identity files" },
  { id: "scheduler", label: "Scheduler", description: "Scheduled jobs" },
  { id: "modules", label: "Modules", description: "Sidecar modules" },
  { id: "channels", label: "Channels", description: "Telegram and other channels" },
  { id: "mobile", label: "Mobile", description: "Mobile-specific settings" },
  { id: "advanced", label: "Advanced", description: "Developer and advanced options" },
];

function SettingsContextPanel({ activeSection }: { activeSection: string }) {
  const section = SETTINGS_SECTIONS.find((s) => s.id === activeSection);

  return (
    <div className="space-y-4 p-4">
      {section && (
        <>
          <div>
            <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Current Section
            </p>
            <p className="text-sm font-medium">{section.label}</p>
            <p className="mt-1 text-xs text-muted-foreground">{section.description}</p>
          </div>
          <div className="rounded-lg border border-border bg-muted/50 p-3">
            <p className="text-xs text-muted-foreground">
              <span className="font-medium text-foreground">Tip:</span> Changes auto-save as
              you update settings.
            </p>
          </div>
        </>
      )}
    </div>
  );
}

function SettingsPage() {
  const { tab } = Route.useSearch();
  const [activeSection, setActiveSection] = useState(tab ?? "ai");

  useEffect(() => {
    useContextPanelStore.getState().setContent(
      <SettingsContextPanel activeSection={activeSection} />,
    );
    return () => useContextPanelStore.getState().clearContent();
  }, [activeSection]);

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

      <div className="flex gap-6">
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
          {activeSection === "router" && <RouterSettings />}
          {activeSection === "profile" && <ProfileSettings />}
          {activeSection === "skills" && <SkillsSettingsTab />}
          {activeSection === "agents" && (
            <div className="space-y-6">
              <p className="text-sm text-muted-foreground">
                Configure agent profiles, tool access, and system prompts. Agents are
                configurable AI assistants that can use tools to perform tasks.
              </p>
              <div className="rounded-lg border bg-muted/30 p-6">
                <div className="flex items-center gap-4">
                  <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-primary/10">
                    <Bot className="h-6 w-6 text-primary" />
                  </div>
                  <div className="flex-1">
                    <h3 className="font-semibold">Agent Management</h3>
                    <p className="text-sm text-muted-foreground">
                      Create, edit, and manage agent configurations
                    </p>
                  </div>
                  <Link to="/agents">
                    <Button>
                      Open Agents Page
                      <ExternalLink className="ml-2 h-4 w-4" />
                    </Button>
                  </Link>
                </div>
              </div>
              <div className="rounded-lg border p-4">
                <h4 className="mb-2 font-medium">Tool Profiles</h4>
                <p className="text-sm text-muted-foreground">
                  When creating agents, you can select a tool profile to control which
                  tools they can access:
                </p>
                <ul className="mt-2 space-y-1 text-sm">
                  <li><strong>Minimal</strong> — Read-only file access and memory</li>
                  <li><strong>Coding</strong> — Shell, filesystem, and memory tools</li>
                  <li><strong>Messaging</strong> — Memory, web, and UI tools</li>
                  <li><strong>Full</strong> — All available tools</li>
                </ul>
              </div>
            </div>
          )}
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
