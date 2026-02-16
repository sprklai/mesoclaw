import { SettingRow } from "@/components/setting-row";
import { SettingsSection } from "@/components/settings-section";
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";

interface PrivacySettingsProps {
  useCloudLLM: boolean;
  explanationVerbosity: "concise" | "balanced" | "detailed";
  onUseCloudLLMChange: (enabled: boolean) => void;
  onExplanationVerbosityChange: (
    verbosity: "concise" | "balanced" | "detailed"
  ) => void;
}

const VERBOSITY_OPTIONS = [
  { value: "concise" as const, label: "Concise - Brief explanations" },
  { value: "balanced" as const, label: "Balanced - Standard detail" },
  {
    value: "detailed" as const,
    label: "Detailed - Comprehensive explanations",
  },
];

export function PrivacySettings({
  useCloudLLM,
  explanationVerbosity,
  onUseCloudLLMChange,
  onExplanationVerbosityChange,
}: PrivacySettingsProps) {
  return (
    <SettingsSection
      title="Privacy & Preferences"
      description="Control how your data is processed and displayed"
    >
      <SettingRow
        label="Use Cloud AI Provider"
        description="Enable cloud-based AI for explanations. When disabled, only local analysis will be used."
        htmlFor="use-cloud-llm"
      >
        <Switch
          id="use-cloud-llm"
          checked={useCloudLLM}
          onCheckedChange={onUseCloudLLMChange}
        />
      </SettingRow>

      <SettingRow
        label="Explanation Verbosity"
        description="Control the level of detail in AI-generated explanations"
        htmlFor="verbosity-select"
      >
        <Select
          value={explanationVerbosity}
          onValueChange={onExplanationVerbosityChange}
          options={VERBOSITY_OPTIONS}
          placeholder="Select verbosity level"
          className="w-full max-w-sm"
        />
      </SettingRow>

      <div className="rounded-lg border border-border bg-muted/50 p-4">
        <h3 className="mb-2 text-sm font-medium">Privacy Notice</h3>
        <ul className="space-y-1 text-sm text-muted-foreground">
          <li>• All schema metadata is stored locally on your device</li>
          <li>
            • Database credentials are stored securely in your OS keychain
          </li>
          <li>
            • When cloud AI is enabled, schema metadata is sent to the AI
            provider
          </li>
          <li>
            • No telemetry or usage data is collected without your consent
          </li>
          <li>• Your actual database data is never sent to external servers</li>
        </ul>
      </div>
    </SettingsSection>
  );
}
