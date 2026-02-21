/**
 * AIProviderSetupForm - Reusable AI provider configuration form
 *
 * Used in both onboarding and settings for configuring AI providers.
 * Provides provider selection, API key input, and connection testing.
 */

import { forwardRef, useEffect, useImperativeHandle, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { TestResult } from "@/lib/models";
import { cn } from "@/lib/utils";
import { useLLMStore } from "@/stores/llm";
import { Eye, Key, Loader2, EyeOff } from "@/lib/icons";

export interface AIProviderSetupFormRef {
  /** Save the current configuration */
  save: () => Promise<void>;
  /** Check if at least one provider is configured */
  hasConfiguredProvider: () => boolean;
}

interface AIProviderSetupFormProps {
  /** Called when provider is successfully configured */
  onConfigured?: () => void;
  /** Additional class name */
  className?: string;
  /** Compact mode for onboarding */
  compact?: boolean;
}

export const AIProviderSetupForm = forwardRef<AIProviderSetupFormRef, AIProviderSetupFormProps>(
  function AIProviderSetupForm({ onConfigured, className, compact = false }, ref) {
  const {
    providersWithKeyStatus,
    loadProvidersWithKeyStatus,
    saveApiKeyForProvider,
    saveProviderConfig,
    providersWithModels,
    loadProvidersAndModels,
    testProviderConnection,
    getApiKey,
  } = useLLMStore();

  const [selectedProviderId, setSelectedProviderId] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [showApiKey, setShowApiKey] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<TestResult | null>(null);

  useEffect(() => {
    loadProvidersWithKeyStatus();
    loadProvidersAndModels();
  }, [loadProvidersWithKeyStatus, loadProvidersAndModels]);

  useEffect(() => {
    if (providersWithKeyStatus.length > 0 && !selectedProviderId) {
      setSelectedProviderId(providersWithKeyStatus[0].id);
    }
  }, [providersWithKeyStatus, selectedProviderId]);

  // Load existing API key when provider changes
  useEffect(() => {
    const selectedProvider = providersWithKeyStatus.find(
      (p) => p.id === selectedProviderId,
    );
    if (selectedProvider?.hasApiKey) {
      getApiKey(selectedProviderId)
        .then(setApiKey)
        .catch(() => setApiKey(""));
    } else {
      setApiKey("");
    }
    setTestResult(null);
  }, [selectedProviderId, providersWithKeyStatus, getApiKey]);

  const selectedProvider = providersWithKeyStatus.find(
    (p) => p.id === selectedProviderId,
  );
  const requiresApiKey = selectedProvider?.requiresApiKey ?? false;

  async function handleTestConnection() {
    if (!selectedProviderId) return;
    if (requiresApiKey && !apiKey.trim()) return;

    setIsTesting(true);
    setTestResult(null);
    try {
      const result = await testProviderConnection(
        selectedProviderId,
        apiKey.trim(),
      );
      setTestResult(result);
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      setTestResult({
        success: false,
        error: errorMessage,
      });
    } finally {
      setIsTesting(false);
    }
  }

  async function handleSave() {
    if (!selectedProviderId) return;

    setIsSaving(true);
    try {
      if (requiresApiKey && apiKey.trim()) {
        try {
          await saveApiKeyForProvider(selectedProviderId, apiKey.trim());
        } catch (err) {
          console.error("[AIProviderSetupForm] Failed to save API key:", err);
        }
      }
      // Use first available model for selected provider
      const providerWithModels = providersWithModels.find(
        (p) => p.id === selectedProviderId,
      );
      const modelId = providerWithModels?.models?.[0]?.id ?? "";
      try {
        await saveProviderConfig(selectedProviderId, modelId);
      } catch (err) {
        console.error("[AIProviderSetupForm] Failed to save provider config:", err);
      }
      onConfigured?.();
    } finally {
      setIsSaving(false);
    }
  }

  /** Check if at least one provider is configured */
  const hasConfiguredProvider = providersWithKeyStatus.some(
    (p) => !p.requiresApiKey || p.hasApiKey,
  );

  // Expose save function and status check via ref
  useImperativeHandle(ref, () => ({
    save: handleSave,
    hasConfiguredProvider: () => hasConfiguredProvider,
  }), [handleSave, hasConfiguredProvider]);

  return (
    <div className={cn("space-y-4", className)}>
      <div className="space-y-1.5">
        <Label htmlFor="provider-select">Provider</Label>
        {providersWithKeyStatus.length === 0 ? (
          <p className="text-sm text-muted-foreground">Loading providers...</p>
        ) : (
          <select
            id="provider-select"
            value={selectedProviderId}
            onChange={(e) => {
              setSelectedProviderId(e.target.value);
              setTestResult(null);
            }}
            className={cn(
              "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground ring-offset-background",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
              "disabled:cursor-not-allowed disabled:opacity-50",
            )}
          >
            {providersWithKeyStatus.map((p) => (
              <option key={p.id} value={p.id}>
                {p.name} {p.hasApiKey ? "(configured)" : ""}
              </option>
            ))}
          </select>
        )}
      </div>

      {requiresApiKey && (
        <div className="space-y-1.5">
          <Label htmlFor="api-key">API Key</Label>
          <div className="relative">
            <Input
              id="api-key"
              type={showApiKey ? "text" : "password"}
              placeholder="sk-..."
              value={apiKey}
              onChange={(e) => {
                setApiKey(e.target.value);
                setTestResult(null);
              }}
              className="pr-10"
            />
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="absolute right-0 top-0 h-full px-3 py-2 hover:bg-transparent"
              onClick={() => setShowApiKey(!showApiKey)}
              tabIndex={-1}
            >
              {showApiKey ? (
                <EyeOff className="h-4 w-4 text-muted-foreground" />
              ) : (
                <Eye className="h-4 w-4 text-muted-foreground" />
              )}
            </Button>
          </div>
          <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <Key className="h-3 w-3" aria-hidden="true" />
            API key is stored securely in your OS keyring
          </p>
        </div>
      )}

      {/* Test Connection */}
      {selectedProviderId && (!requiresApiKey || apiKey.trim()) && (
        <div className="flex items-center gap-3">
          <Button
            variant="outline"
            size={compact ? "sm" : "default"}
            onClick={handleTestConnection}
            disabled={isTesting}
          >
            {isTesting ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Testing...
              </>
            ) : (
              "Test Connection"
            )}
          </Button>
          {testResult && (
            <span
              className={cn(
                "text-sm font-medium",
                testResult.success ? "text-green-600" : "text-destructive",
              )}
            >
              {testResult.success
                ? `Connected${testResult.latencyMs ? ` (${testResult.latencyMs}ms)` : ""}`
                : "Connection failed"}
            </span>
          )}
        </div>
      )}

      {/* Test Result Details */}
      {testResult && !testResult.success && testResult.error && (
        <div className="rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {testResult.error}
        </div>
      )}

      {/* Save Button - only show if not in compact mode (onboarding handles its own save) */}
      {!compact && (
        <Button onClick={handleSave} disabled={isSaving}>
          {isSaving ? "Saving..." : "Save Configuration"}
        </Button>
      )}
    </div>
  );
  }
);
