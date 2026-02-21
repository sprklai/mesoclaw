import { AIProviderConfiguration } from "@/components/settings/AIProviderConfiguration";

/**
 * AI Settings Tab - wraps the reusable AIProviderConfiguration component.
 * This is the settings page version with all features enabled.
 */
export function AISettingsTab() {
  return (
    <AIProviderConfiguration
      showGlobalDefault={true}
      showHeader={true}
      compact={false}
    />
  );
}
