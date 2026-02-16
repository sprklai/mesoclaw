export type Theme = "light" | "dark" | "system";

export type LogLevel = "error" | "warn" | "info" | "debug" | "trace";

/**
 * Full Settings interface matching backend Settings struct.
 * Backend uses #[serde(rename_all = "camelCase")]
 */
export interface Settings {
  theme: Theme;
  sidebarExpanded: boolean;
  showInTray: boolean;
  launchAtLogin: boolean;
  enableLogging: boolean;
  logLevel: LogLevel;
  enableNotifications: boolean;
  notifyGeneral: boolean;
  notifyReminders: boolean;
  notifyUpdates: boolean;
  notifyAlerts: boolean;
  notifyActivity: boolean;
  llmModel: string;
  useCloudLLM: boolean;
  explanationVerbosity: string;
  // Advanced AI settings
  temperature: number;
  maxTokens: number;
  timeout: number;
  streamResponses: boolean;
  enableCaching: boolean;
  debugMode: boolean;
  customBaseUrl: string | null;
}

/**
 * Partial update for settings, matching backend SettingsUpdate struct.
 */
export type SettingsUpdate = Partial<Settings>;
