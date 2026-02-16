import type { LogLevel, Theme } from "@/lib/tauri/settings/types";
import type { Settings } from "@/lib/tauri/settings/types";

import { SettingRow } from "@/components/setting-row";
import { SettingsSection } from "@/components/settings-section";
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { LOG_LEVEL_OPTIONS, THEME_OPTIONS } from "@/constants/settings";

interface AppSettingsTabProps {
  theme: Theme;
  settings: Settings;
  isSaving: boolean;
  onUpdateSetting: <K extends keyof Settings>(
    key: K,
    value: Settings[K]
  ) => void;
  onAutostartChange: (enabled: boolean) => void;
  onTrayVisibilityChange: (visible: boolean) => void;
  onNotificationChange: (enabled: boolean) => void;
  autostartEnabled: boolean;
}

export function AppSettingsTab({
  theme,
  settings,
  isSaving,
  onUpdateSetting,
  onAutostartChange,
  onTrayVisibilityChange,
  onNotificationChange,
  autostartEnabled,
}: AppSettingsTabProps) {
  return (
    <div className="space-y-6">
      {/* Appearance Section */}
      <SettingsSection
        title="Appearance"
        description="Customize how the application looks"
      >
        <SettingRow
          label="Theme"
          description="Select your preferred color scheme"
        >
          <Select
            value={theme}
            onValueChange={(value) => onUpdateSetting("theme", value)}
            options={THEME_OPTIONS}
            disabled={isSaving}
            className="w-full sm:w-40"
          />
        </SettingRow>

        <SettingRow
          label="Sidebar Expanded"
          description="Keep the sidebar expanded by default"
          htmlFor="sidebar-expanded"
        >
          <Switch
            id="sidebar-expanded"
            checked={settings.sidebarExpanded}
            onCheckedChange={(checked) =>
              onUpdateSetting("sidebarExpanded", checked)
            }
            disabled={isSaving}
          />
        </SettingRow>
      </SettingsSection>

      {/* Behavior Section */}
      <SettingsSection
        title="Behavior"
        description="Control how the application behaves"
      >
        <SettingRow
          label="Show in System Tray"
          description="Show the application icon in the system tray"
          htmlFor="show-in-tray"
        >
          <Switch
            id="show-in-tray"
            checked={settings.showInTray}
            onCheckedChange={onTrayVisibilityChange}
            disabled={isSaving}
          />
        </SettingRow>

        <SettingRow
          label="Launch at Login"
          description="Automatically start when you log in"
          htmlFor="launch-at-login"
        >
          <Switch
            id="launch-at-login"
            checked={autostartEnabled}
            onCheckedChange={onAutostartChange}
            disabled={isSaving}
          />
        </SettingRow>
      </SettingsSection>

      {/* Notifications Section */}
      <SettingsSection
        title="Notifications"
        description="Configure notification preferences"
      >
        <SettingRow
          label="Enable Notifications"
          description="Allow the app to send you notifications"
          htmlFor="enable-notifications"
        >
          <Switch
            id="enable-notifications"
            checked={settings.enableNotifications}
            onCheckedChange={onNotificationChange}
            disabled={isSaving}
          />
        </SettingRow>
      </SettingsSection>

      {/* Developer Section */}
      <SettingsSection
        title="Developer"
        description="Advanced settings for debugging and development"
      >
        <SettingRow
          label="Enable Logging"
          description="Enable detailed application logging"
          htmlFor="enable-logging"
        >
          <Switch
            id="enable-logging"
            checked={settings.enableLogging}
            onCheckedChange={(checked) =>
              onUpdateSetting("enableLogging", checked)
            }
            disabled={isSaving}
          />
        </SettingRow>

        <SettingRow
          label="Log Level"
          description="Set the minimum log level to record"
        >
          <Select<LogLevel>
            value={settings.logLevel}
            onValueChange={(value) => onUpdateSetting("logLevel", value)}
            options={LOG_LEVEL_OPTIONS}
            disabled={isSaving || !settings.enableLogging}
            className="w-full sm:w-40"
          />
        </SettingRow>
      </SettingsSection>
    </div>
  );
}
