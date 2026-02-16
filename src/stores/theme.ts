import { create } from "zustand";

import type { Theme } from "@/lib/tauri/settings/types";

import { useSettings } from "@/stores/settings";

interface ThemeStore {
  theme: Theme;
  resolvedTheme: "light" | "dark";
  setTheme: (theme: Theme) => Promise<void>;
  applyTheme: (theme: Theme) => void;
  initialize: () => void;
}

function getSystemTheme(): "light" | "dark" {
  if (typeof window === "undefined") return "light";
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}

/**
 * Applies theme to DOM by adding/removing 'dark' class
 * Returns the resolved theme (light or dark)
 */
function applyThemeToDOM(theme: Theme): "light" | "dark" {
  const resolved = theme === "system" ? getSystemTheme() : theme;
  const root = document.documentElement;

  if (resolved === "dark") {
    root.classList.add("dark");
  } else {
    root.classList.remove("dark");
  }

  return resolved;
}

export const useTheme = create<ThemeStore>((set) => ({
  theme: "system",
  resolvedTheme: "light",

  applyTheme: (theme: Theme) => {
    const resolved = applyThemeToDOM(theme);
    set({ theme, resolvedTheme: resolved });
  },

  setTheme: async (newTheme: Theme) => {
    await useSettings.getState().updateSettings({ theme: newTheme });
    const resolved = applyThemeToDOM(newTheme);
    set({ theme: newTheme, resolvedTheme: resolved });
  },

  initialize: () => {
    const settings = useSettings.getState().settings;
    if (settings) {
      const resolved = applyThemeToDOM(settings.theme);
      set({ theme: settings.theme, resolvedTheme: resolved });
    }

    // Note: System theme change listener is now handled by ThemeProvider component
    // This initialize() is called during StoreInitializer setup
  },
}));
