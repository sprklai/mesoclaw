import { useEffect, useState } from "react";

import type { Settings } from "@/lib/tauri/settings/types";

import { SettingRow } from "@/components/setting-row";
import { SettingsSection } from "@/components/settings-section";
import { Switch } from "@/components/ui/switch";
import { useSettings } from "@/stores/settings";

interface AdvancedSettingsTabProps {
  // Add props as needed
}

export function AdvancedSettingsTab(_props: AdvancedSettingsTabProps) {
  const { settings, updateSettings, isLoading } = useSettings();

  // Local state for inputs (updated on change, debounced save)
  const [temperature, setTemperature] = useState(0.7);
  const [maxTokens, setMaxTokens] = useState(4096);
  const [timeout, setTimeout] = useState(30);
  const [cacheDuration, setCacheDuration] = useState(24);
  const [customBaseUrl, setCustomBaseUrl] = useState("");
  const [enableCaching, setEnableCaching] = useState(true);
  const [streamResponses, setStreamResponses] = useState(true);
  const [debugMode, setDebugMode] = useState(false);

  // Load settings from store
  useEffect(() => {
    if (settings) {
      setTemperature(settings.temperature);
      setMaxTokens(settings.maxTokens);
      setTimeout(settings.timeout);
      setEnableCaching(settings.enableCaching);
      setStreamResponses(settings.streamResponses);
      setDebugMode(settings.debugMode);
      setCustomBaseUrl(settings.customBaseUrl ?? "");
    }
  }, [settings]);

  // Handle save with debounce for text inputs
  useEffect(() => {
    const timeoutId = window.setTimeout(() => {
      if (settings && !isLoading) {
        // eslint-disable-next-line @typescript-eslint/no-floating-promises
        updateSettings({
          temperature,
          maxTokens,
          timeout,
          enableCaching,
          streamResponses,
          debugMode,
          customBaseUrl: customBaseUrl || null,
        });
      }
    }, 500);
    return () => window.clearTimeout(timeoutId);
  }, [
    temperature,
    maxTokens,
    timeout,
    customBaseUrl,
    enableCaching,
    streamResponses,
    debugMode,
    settings,
    isLoading,
    updateSettings,
  ]);

  // Immediate save for toggles
  const handleToggleChange = (key: keyof Settings, value: boolean) => {
    // eslint-disable-next-line @typescript-eslint/no-floating-promises
    updateSettings({ [key]: value });
  };

  if (isLoading) {
    return <div className="p-4 text-muted-foreground">Loading settings...</div>;
  }

  return (
    <div className="space-y-6">
      {/* Caching Settings */}
      <SettingsSection
        title="Caching"
        description="Configure AI response caching to reduce API calls and improve performance"
      >
        <div className="space-y-4">
          <SettingRow
            label="Enable Caching"
            description="Cache AI responses to avoid redundant API calls"
          >
            <Switch
              checked={enableCaching}
              onCheckedChange={(checked) => {
                setEnableCaching(checked);
                handleToggleChange("enableCaching", checked);
              }}
            />
          </SettingRow>

          <SettingRow
            label="Cache Duration"
            description="How long to cache responses (in hours)"
          >
            <div className="flex items-center gap-2">
              <input
                type="number"
                value={cacheDuration}
                onChange={(e) => setCacheDuration(Number(e.target.value))}
                min={1}
                max={168}
                className="w-20 rounded-md border border-input bg-background px-3 py-2 text-sm"
              />
              <span className="text-sm text-muted-foreground">hours</span>
            </div>
          </SettingRow>
        </div>
      </SettingsSection>

      {/* Request Settings */}
      <SettingsSection
        title="Request Settings"
        description="Configure AI request parameters and behavior"
      >
        <div className="space-y-4">
          <SettingRow
            label="Temperature"
            description="Controls randomness in responses (0.0 - 1.0)"
          >
            <div className="flex items-center gap-2">
              <input
                type="number"
                value={temperature}
                onChange={(e) => setTemperature(Number(e.target.value))}
                min={0}
                max={1}
                step={0.1}
                className="w-20 rounded-md border border-input bg-background px-3 py-2 text-sm"
              />
            </div>
          </SettingRow>

          <SettingRow
            label="Max Tokens"
            description="Maximum number of tokens in the response"
          >
            <div className="flex items-center gap-2">
              <input
                type="number"
                value={maxTokens}
                onChange={(e) => setMaxTokens(Number(e.target.value))}
                min={256}
                max={32768}
                step={256}
                className="w-28 rounded-md border border-input bg-background px-3 py-2 text-sm"
              />
            </div>
          </SettingRow>

          <SettingRow label="Timeout" description="Request timeout in seconds">
            <div className="flex items-center gap-2">
              <input
                type="number"
                value={timeout}
                onChange={(e) => setTimeout(Number(e.target.value))}
                min={5}
                max={300}
                className="w-20 rounded-md border border-input bg-background px-3 py-2 text-sm"
              />
              <span className="text-sm text-muted-foreground">seconds</span>
            </div>
          </SettingRow>
        </div>
      </SettingsSection>

      {/* Advanced Options */}
      <SettingsSection
        title="Advanced Options"
        description="Additional configuration options for power users"
      >
        <div className="space-y-4">
          <SettingRow
            label="Stream Responses"
            description="Enable streaming for real-time response generation"
          >
            <Switch
              checked={streamResponses}
              onCheckedChange={(checked) => {
                setStreamResponses(checked);
                handleToggleChange("streamResponses", checked);
              }}
            />
          </SettingRow>

          <SettingRow
            label="Debug Mode"
            description="Enable detailed logging for AI requests and responses"
          >
            <Switch
              checked={debugMode}
              onCheckedChange={(checked) => {
                setDebugMode(checked);
                handleToggleChange("debugMode", checked);
              }}
            />
          </SettingRow>

          <SettingRow
            label="Custom Base URL"
            description="Use a custom base URL for API requests (overrides provider default)"
          >
            <input
              type="text"
              value={customBaseUrl}
              onChange={(e) => setCustomBaseUrl(e.target.value)}
              placeholder="https://api.example.com"
              className="w-64 rounded-md border border-input bg-background px-3 py-2 text-sm"
            />
          </SettingRow>
        </div>
      </SettingsSection>
    </div>
  );
}
