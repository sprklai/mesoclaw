import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import type { ProviderWithModels } from "@/lib/models";

import { ProviderModelSelector } from "@/components/ai-elements/provider-model-selector";
import { SettingRow } from "@/components/setting-row";
import { SettingsSection } from "@/components/settings-section";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { CheckCircle2, Eye, EyeOff, Loader2, XCircle } from "@/lib/icons";

interface AISettingsProps {
  providers: ProviderWithModels[];
  selectedProvider: string;
  selectedModel: string;
  apiKey: string;
  isLoadingApiKey?: boolean;
  onProviderChange: (providerId: string) => void;
  onModelChange: (modelId: string) => void;
  onApiKeyChange: (apiKey: string) => void;
  onAddCustomModel?: () => void;
}

interface TestResult {
  success: boolean;
  message: string;
}

export function AISettings({
  providers,
  selectedProvider,
  selectedModel,
  apiKey,
  isLoadingApiKey = false,
  onProviderChange,
  onModelChange,
  onApiKeyChange,
  onAddCustomModel,
}: AISettingsProps) {
  const [isTestingConnection, setIsTestingConnection] = useState(false);
  const [testResult, setTestResult] = useState<TestResult | null>(null);
  const [localApiKey, setLocalApiKey] = useState(apiKey);
  const [showApiKey, setShowApiKey] = useState(false);

  // Sync localApiKey with apiKey prop when dialog reopens
  useEffect(() => {
    setLocalApiKey(apiKey);
  }, [apiKey]);

  // Get the provider name for display
  const getProviderName = (providerId: string) => {
    const provider = providers.find((p) => p.id === providerId);
    return provider?.name || providerId;
  };

  const handleTestConnection = async () => {
    if (!localApiKey.trim()) {
      setTestResult({
        success: false,
        message: "Please enter an API key",
      });
      return;
    }

    if (!selectedProvider || !selectedModel) {
      setTestResult({
        success: false,
        message: "Please select a provider and model",
      });
      return;
    }

    setIsTestingConnection(true);
    setTestResult(null);

    try {
      const result = await invoke<TestResult>("test_llm_provider_command", {
        config: {
          providerId: selectedProvider,
          modelId: selectedModel,
          apiKey: localApiKey,
        },
      });
      setTestResult(result);
    } catch (error) {
      console.error("Connection test error:", error);
      setTestResult({
        success: false,
        message: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setIsTestingConnection(false);
    }
  };

  const handleSaveApiKey = () => {
    onApiKeyChange(localApiKey);
  };

  return (
    <SettingsSection
      title="AI Provider"
      description={`Configure ${getProviderName(selectedProvider)} for AI-powered explanations`}
    >
      <SettingRow
        label="API Key"
        description={`Your ${getProviderName(selectedProvider)} API key (stored securely in OS keychain)`}
        htmlFor="api-key"
      >
        <div className="flex w-full max-w-sm flex-col gap-2">
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Input
                id="api-key"
                type={showApiKey ? "text" : "password"}
                placeholder={
                  isLoadingApiKey ? "Loading API key..." : "Enter API key"
                }
                value={localApiKey}
                onChange={(e) => setLocalApiKey(e.target.value)}
                disabled={isLoadingApiKey}
                className="pr-10"
              />
              <Button
                type="button"
                variant="ghost"
                size="sm"
                className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
                onClick={() => setShowApiKey(!showApiKey)}
                tabIndex={-1}
                disabled={isLoadingApiKey}
              >
                {isLoadingApiKey ? (
                  <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                ) : showApiKey ? (
                  <EyeOff className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <Eye className="h-4 w-4 text-muted-foreground" />
                )}
              </Button>
            </div>
            <Button
              onClick={handleSaveApiKey}
              disabled={isLoadingApiKey || localApiKey === apiKey}
              variant="outline"
            >
              Save
            </Button>
          </div>
          {testResult && (
            <div
              className={`flex items-center gap-2 text-sm ${
                testResult.success ? "text-green-600" : "text-red-600"
              }`}
            >
              {testResult.success ? (
                <CheckCircle2 className="h-4 w-4" />
              ) : (
                <XCircle className="h-4 w-4" />
              )}
              <span>{testResult.message}</span>
            </div>
          )}
        </div>
      </SettingRow>

      <SettingRow
        label="Provider & Model"
        description="Select the AI provider and model to use for explanations"
      >
        <div className="w-full max-w-sm">
          <ProviderModelSelector
            providers={providers}
            selectedProvider={selectedProvider}
            selectedModel={selectedModel}
            onProviderChange={onProviderChange}
            onModelChange={onModelChange}
            onAddCustomModel={onAddCustomModel}
            disabled={isLoadingApiKey}
          />
        </div>
      </SettingRow>

      <SettingRow
        label="Test Connection"
        description="Verify that your API key and model selection are working"
      >
        <Button
          onClick={handleTestConnection}
          disabled={
            isTestingConnection ||
            isLoadingApiKey ||
            !localApiKey.trim() ||
            !selectedProvider ||
            !selectedModel
          }
          variant="outline"
        >
          {isTestingConnection ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Testing...
            </>
          ) : (
            "Test Connection"
          )}
        </Button>
      </SettingRow>
    </SettingsSection>
  );
}
