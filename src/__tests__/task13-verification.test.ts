/**
 * Task 13: Final Testing and Verification
 *
 * Basic verification tests for settings consolidation
 */

import { describe, it, expect } from "vitest";

describe("Task 13: Settings Consolidation", () => {
  describe("File Structure", () => {
    it("should have settings route", async () => {
      const settings = await import("@/routes/settings");
      expect(settings).toBeDefined();
    });

    it("should have AI Settings Tab component", async () => {
      const aiSettingsTab = await import("@/components/settings/AISettingsTab");
      expect(aiSettingsTab).toBeDefined();
    });

    it("should have App Settings Tab component", async () => {
      const appSettingsTab =
        await import("@/components/settings/AppSettingsTab");
      expect(appSettingsTab).toBeDefined();
    });

    it("should have App Settings Store", async () => {
      const appSettingsStore = await import("@/stores/appSettingsStore");
      expect(appSettingsStore).toBeDefined();
      expect(appSettingsStore.useAppSettingsStore).toBeDefined();
    });

    it("should have AI Model Quick Access component", async () => {
      const aiQuickAccess = await import("@/components/ai/AIModelQuickAccess");
      expect(aiQuickAccess).toBeDefined();
    });
  });

  describe("Store Exports", () => {
    it("should export useAppSettingsStore hook", async () => {
      const { useAppSettingsStore } = await import("@/stores/appSettingsStore");
      expect(typeof useAppSettingsStore).toBe("function");
    });

    it("should have app settings store with expected methods", async () => {
      const { useAppSettingsStore } = await import("@/stores/appSettingsStore");
      const store = useAppSettingsStore.getState();

      expect(store).toHaveProperty("theme");
      expect(store).toHaveProperty("setTheme");
      expect(store).toHaveProperty("sidebarExpanded");
      expect(store).toHaveProperty("setSidebarExpanded");
      expect(store).toHaveProperty("systemTray");
      expect(store).toHaveProperty("setSystemTray");
      expect(store).toHaveProperty("launchAtLogin");
      expect(store).toHaveProperty("setLaunchAtLogin");
      expect(store).toHaveProperty("notificationsEnabled");
      expect(store).toHaveProperty("setNotificationsEnabled");
      expect(store).toHaveProperty("cloudLlmEnabled");
      expect(store).toHaveProperty("setCloudLlmEnabled");
      expect(store).toHaveProperty("explanationVerbosity");
      expect(store).toHaveProperty("setExplanationVerbosity");
    });
  });

  describe("Component Exports", () => {
    it("should export AISettingsTab component", async () => {
      const { AISettingsTab } =
        await import("@/components/settings/AISettingsTab");
      expect(typeof AISettingsTab).toBe("function");
    });

    it("should export AppSettingsTab component", async () => {
      const { AppSettingsTab } =
        await import("@/components/settings/AppSettingsTab");
      expect(typeof AppSettingsTab).toBe("function");
    });

    it("should export AIModelQuickAccess components", async () => {
      const { AIModelQuickAccess, AIModelQuickAccessTrigger } =
        await import("@/components/ai/AIModelQuickAccess");
      expect(typeof AIModelQuickAccess).toBe("function");
      expect(typeof AIModelQuickAccessTrigger).toBe("function");
    });
  });
});
