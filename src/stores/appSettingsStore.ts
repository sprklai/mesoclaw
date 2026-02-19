import { create } from "zustand";
import { persist } from "zustand/middleware";

export interface AppSettings {
  // Appearance
  theme: "light" | "dark" | "system";
  sidebarExpanded: boolean;

  // Behavior
  systemTray: boolean;
  launchAtLogin: boolean;

  // Notifications
  notificationsEnabled: boolean;

  // Privacy & Data
  cloudLlmEnabled: boolean;
  explanationVerbosity: "concise" | "standard" | "detailed";

  // Onboarding
  onboardingCompleted: boolean;
}

interface AppSettingsActions {
  setTheme: (theme: AppSettings["theme"]) => void;
  setSidebarExpanded: (expanded: boolean) => void;
  setSystemTray: (enabled: boolean) => void;
  setLaunchAtLogin: (enabled: boolean) => void;
  setNotificationsEnabled: (enabled: boolean) => void;
  setCloudLlmEnabled: (enabled: boolean) => void;
  setExplanationVerbosity: (
    verbosity: AppSettings["explanationVerbosity"]
  ) => void;
  completeOnboarding: () => void;
  resetSettings: () => void;
}

type AppSettingsStore = AppSettings & AppSettingsActions;

const DEFAULT_SETTINGS: AppSettings = {
  theme: "system",
  sidebarExpanded: true,
  systemTray: true,
  launchAtLogin: false,
  notificationsEnabled: true,
  cloudLlmEnabled: true,
  explanationVerbosity: "standard",
  onboardingCompleted: false,
};

export const useAppSettingsStore = create<AppSettingsStore>()(
  persist(
    (set) => ({
      ...DEFAULT_SETTINGS,

      setTheme: (theme) => set({ theme }),
      setSidebarExpanded: (sidebarExpanded) => set({ sidebarExpanded }),
      setSystemTray: (systemTray) => set({ systemTray }),
      setLaunchAtLogin: (launchAtLogin) => set({ launchAtLogin }),
      setNotificationsEnabled: (notificationsEnabled) =>
        set({ notificationsEnabled }),
      setCloudLlmEnabled: (cloudLlmEnabled) => set({ cloudLlmEnabled }),
      setExplanationVerbosity: (explanationVerbosity) =>
        set({ explanationVerbosity }),

      completeOnboarding: () => set({ onboardingCompleted: true }),

      resetSettings: () => set(DEFAULT_SETTINGS),
    }),
    {
      name: "app-settings-storage",
    }
  )
);
