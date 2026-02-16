import { useEffect } from "react";

import { useSettings } from "@/stores/settings";
import { useTheme } from "@/stores/theme";

interface ThemeProviderProps {
  children: React.ReactNode;
}

/**
 * ThemeProvider - Integrates app settings with theme application
 *
 * This component:
 * - Reads theme from Tauri settings store
 * - Applies theme classes to document.documentElement
 * - Listens for system theme changes when theme="system"
 * - Updates DOM when theme changes in settings
 *
 * Place this component high in the component tree (wrapping RouterProvider)
 */
export function ThemeProvider({ children }: ThemeProviderProps) {
  const settings = useSettings((state) => state.settings);
  const theme = useTheme((state) => state.theme);
  const resolvedTheme = useTheme((state) => state.resolvedTheme);
  const applyTheme = useTheme((state) => state.applyTheme);

  // Apply theme to DOM when theme setting changes
  useEffect(() => {
    if (settings?.theme) {
      applyTheme(settings.theme);
    }
  }, [settings?.theme, applyTheme]);

  // Listen for system theme preference changes
  useEffect(() => {
    if (theme !== "system") return;

    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");

    const handleChange = () => {
      applyTheme("system");
    };

    // Add event listener
    mediaQuery.addEventListener("change", handleChange);

    // Cleanup
    return () => {
      mediaQuery.removeEventListener("change", handleChange);
    };
  }, [theme, applyTheme]);

  // Add data-theme attribute to document for potential CSS selectors
  useEffect(() => {
    if (resolvedTheme) {
      document.documentElement.setAttribute("data-theme", resolvedTheme);
    }
  }, [resolvedTheme]);

  return <>{children}</>;
}
