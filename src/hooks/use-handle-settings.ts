import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { requestPermission } from "@tauri-apps/plugin-notification";
import { useCallback, useEffect, useState } from "react";

import type { Settings, Theme } from "@/lib/tauri/settings/types";

import { useAsyncAction } from "@/hooks/use-async-action";
import {
  getSettings,
  setTrayVisible,
  updateSettings,
} from "@/lib/tauri/settings";
import { useTheme } from "@/stores/theme";

export function useHandleSettings() {
  const setTheme = useTheme((state) => state.setTheme);
  const theme = useTheme((state) => state.theme);

  const [settings, setSettings] = useState<Settings | null>(null);
  const [autostartEnabled, setAutostartEnabled] = useState(false);

  const [isLoading, setIsLoading] = useState(true);
  const [withSaving, isSaving] = useAsyncAction();

  useEffect(() => {
    const loadSettings = async () => {
      try {
        const [loadedSettings, autostart] = await Promise.all([
          getSettings(),
          isAutostartEnabled(),
        ]);
        setSettings(loadedSettings);
        setAutostartEnabled(autostart);
      } catch (error) {
        console.error("Failed to load settings:", error);
      } finally {
        setIsLoading(false);
      }
    };

    loadSettings();
  }, []);

  const handleUpdateSetting = useCallback(
    async <K extends keyof Settings>(key: K, value: Settings[K]) => {
      if (!settings) return;

      const previousSettings = settings;
      setSettings((prev) => (prev ? { ...prev, [key]: value } : null));

      await withSaving(
        async () => {
          const updatedSettings = await updateSettings({ [key]: value });
          setSettings(updatedSettings);

          if (key === "theme") {
            await setTheme(value as Theme);
          }
        },
        {
          onError: () => setSettings(previousSettings),
          errorMessage: `Failed to update ${String(key)}`,
          successMessage: "Settings updated",
        }
      );
    },
    [settings, setTheme, withSaving]
  );

  const handleAutostartChange = useCallback(
    async (enabled: boolean) => {
      const previousValue = autostartEnabled;
      setAutostartEnabled(enabled);

      await withSaving(
        async () => {
          await (enabled ? enableAutostart() : disableAutostart());
          await updateSettings({ launchAtLogin: enabled });
          setSettings((prev) =>
            prev ? { ...prev, launchAtLogin: enabled } : null
          );
        },
        {
          onError: () => setAutostartEnabled(previousValue),
          errorMessage: "Failed to update autostart setting",
          successMessage: enabled ? "Autostart enabled" : "Autostart disabled",
        }
      );
    },
    [autostartEnabled, withSaving]
  );

  const handleTrayVisibilityChange = useCallback(
    async (visible: boolean) => {
      if (!settings) return;

      const previousSettings = settings;
      setSettings((prev) => (prev ? { ...prev, showInTray: visible } : null));

      await withSaving(
        async () => {
          await setTrayVisible(visible);
          await updateSettings({ showInTray: visible });
        },
        {
          onError: () => setSettings(previousSettings),
          errorMessage: "Failed to update tray visibility",
          successMessage: visible
            ? "System tray enabled"
            : "System tray disabled",
        }
      );
    },
    [settings, withSaving]
  );

  const handleNotificationChange = useCallback(
    async (enabled: boolean) => {
      if (!settings) return;

      const previousSettings = settings;

      // If enabling notifications, request permission first
      if (enabled) {
        const granted = await requestPermission();
        if (granted !== "granted") {
          // Permission denied, don't enable notifications
          return;
        }
      }

      setSettings((prev) =>
        prev ? { ...prev, enableNotifications: enabled } : null
      );

      await withSaving(
        async () => {
          await updateSettings({ enableNotifications: enabled });
        },
        {
          onError: () => setSettings(previousSettings),
          errorMessage: "Failed to update notification settings",
          successMessage: enabled
            ? "Notifications enabled"
            : "Notifications disabled",
        }
      );
    },
    [settings, withSaving]
  );

  return {
    theme,
    settings,
    autostartEnabled,
    isLoading,
    isSaving,
    handleUpdateSetting,
    handleAutostartChange,
    handleTrayVisibilityChange,
    handleNotificationChange,
  };
}
