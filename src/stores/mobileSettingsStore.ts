/**
 * mobileSettingsStore — device-local mobile preferences.
 *
 * Persisted to `localStorage` so they survive app restarts without needing
 * a backend round-trip.  These are purely client-side UX toggles.
 *
 * Phase 7.4.7 implementation.
 */

import { create } from "zustand";
import { persist } from "zustand/middleware";

export interface MobileSettings {
  /** Whether haptic impulses are enabled (navigator.vibrate). */
  hapticEnabled: boolean;
  /** Whether the app may show push notifications. */
  pushNotificationsEnabled: boolean;
  /** Whether to show the battery-optimisation banner (Android). */
  batteryOptimisationAcknowledged: boolean;
  /** iOS background-refresh allowed by the user (informational). */
  backgroundRefreshEnabled: boolean;
}

interface MobileSettingsStore extends MobileSettings {
  setHapticEnabled: (enabled: boolean) => void;
  setPushNotificationsEnabled: (enabled: boolean) => void;
  acknowledgeBatteryOptimisation: () => void;
  setBackgroundRefreshEnabled: (enabled: boolean) => void;
}

export const useMobileSettingsStore = create<MobileSettingsStore>()(
  persist(
    (set) => ({
      hapticEnabled: true,
      pushNotificationsEnabled: false,
      batteryOptimisationAcknowledged: false,
      backgroundRefreshEnabled: true,

      setHapticEnabled: (enabled) => set({ hapticEnabled: enabled }),
      setPushNotificationsEnabled: (enabled) =>
        set({ pushNotificationsEnabled: enabled }),
      acknowledgeBatteryOptimisation: () =>
        set({ batteryOptimisationAcknowledged: true }),
      setBackgroundRefreshEnabled: (enabled) =>
        set({ backgroundRefreshEnabled: enabled }),
    }),
    {
      name: "mesoclaw-mobile-settings",
    },
  ),
);
