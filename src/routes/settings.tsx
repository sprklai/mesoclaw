import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

import { AdvancedSettingsTab } from "@/components/settings/AdvancedSettingsTab";
import { AISettingsTab } from "@/components/settings/AISettingsTab";
import { AppSettingsTab } from "@/components/settings/AppSettingsTab";
import { SkillsSettingsTab } from "@/components/settings/SkillsSettingsTab";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useHandleSettings } from "@/hooks/use-handle-settings";
import { Settings } from "@/lib/icons";
// Workspace store removed
// import { useWorkspaceStore } from "@/stores/workspace-store";

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
});

function SettingsPage() {
  const [activeTab, setActiveTab] = useState("ai");

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
        <p className="text-muted-foreground">Loading settings...</p>
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

  return (
    <div className="mx-auto w-full max-w-4xl space-y-6 sm:space-y-8">
      {/* Header with icon */}
      <div className="flex items-center gap-3">
        <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
          <Settings className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h1 className="text-xl font-bold sm:text-2xl">Settings</h1>
          <p className="text-sm text-muted-foreground">
            Configure your AI providers and model settings
          </p>
        </div>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList className="w-full">
          <TabsTrigger value="ai" className="flex-1 font-bold">
            AI Provider
          </TabsTrigger>
          <TabsTrigger value="skills" className="flex-1 font-bold">
            Skills
          </TabsTrigger>
          <TabsTrigger value="app" className="flex-1 font-bold">
            App Settings
          </TabsTrigger>
          <TabsTrigger value="advanced" className="flex-1 font-bold">
            Advanced
          </TabsTrigger>
        </TabsList>

        <TabsContent value="ai" className="mt-6">
          <AISettingsTab />
        </TabsContent>

        <TabsContent value="skills" className="mt-6">
          <SkillsSettingsTab />
        </TabsContent>

        <TabsContent value="app" className="mt-6">
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
        </TabsContent>

        <TabsContent value="advanced" className="mt-6">
          <AdvancedSettingsTab />
        </TabsContent>
      </Tabs>
    </div>
  );
}
