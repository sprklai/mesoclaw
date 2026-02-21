import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import { AddCustomModelDialog } from "@/components/settings/AddCustomModelDialog";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Eye,
  EyeOff,
  ExternalLink,
  Plus,
  Server,
  Cloud,
  Cpu,
  Trash2,
  Loader2,
  X,
  CheckCircle2,
  XCircle,
} from "@/lib/icons";
import { showError, showSuccess } from "@/lib/toast";
import { APP_IDENTITY } from "@/config/app-identity";
import { useLLMStore } from "@/stores/llm";

interface AIProviderDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

type TabType = "local" | "gateway" | "providers";
type Provider =
  | "ollama"
  | "vercel_gateway"
  | "openrouter"
  | "openai"
  | "anthropic"
  | "groq";

const LOCAL_PROVIDERS: Provider[] = ["ollama"];
const GATEWAY_PROVIDERS: Provider[] = ["vercel_gateway", "openrouter"];
const AI_PROVIDERS: Provider[] = ["openai", "anthropic", "groq"];

const PROVIDER_NAMES: Record<Provider, string> = {
  ollama: "Ollama",
  vercel_gateway: "Vercel AI",
  openrouter: "OpenRouter",
  openai: "OpenAI",
  anthropic: "Anthropic",
  groq: "Groq",
};

export function AIProviderDialog({
  open,
  onOpenChange,
}: AIProviderDialogProps) {
  const { config, providersWithModels, getApiKey, loadProvidersWithKeyStatus } =
    useLLMStore();

  const [activeTab, setActiveTab] = useState<TabType>(() => {
    if (config?.providerId) {
      if (config.providerId === "ollama") return "local";
      if (
        config.providerId === "vercel-ai-gateway" ||
        config.providerId === "openrouter"
      )
        return "gateway";
      return "providers";
    }
    return "gateway";
  });

  const [selectedProvider, setSelectedProvider] = useState<Provider>(() => {
    if (config?.providerId === "ollama") return "ollama";
    if (config?.providerId === "vercel-ai-gateway") return "vercel_gateway";
    if (config?.providerId === "openrouter") return "openrouter";
    return "openai";
  });

  const [model, setModel] = useState(config?.modelId || "");
  const [ollamaBaseUrl, setOllamaBaseUrl] = useState(
    "http://localhost:11434/v1"
  );
  const [vercelGatewayUrl, setVercelGatewayUrl] = useState(
    "https://ai-gateway.vercel.sh/v1"
  );
  const [openrouterGatewayUrl, setOpenrouterGatewayUrl] = useState(
    "https://openrouter.ai/api/v1"
  );
  const [customModel, setCustomModel] = useState("");
  const [showApiKey, setShowApiKey] = useState(false);
  const [customModels, setCustomModels] = useState<Record<string, string[]>>(
    {}
  );
  const [showAddModel, setShowAddModel] = useState(false);

  // API key state per provider
  const [apiKeys, setApiKeys] = useState<Record<string, string>>({});
  const [isLoadingApiKey, setIsLoadingApiKey] = useState(false);

  // Test connection state
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    success: boolean;
    error?: string;
    latencyMs?: number;
  } | null>(null);

  // Helper function to convert provider type to storage ID
  const getProviderIdForStorage = (provider: Provider): string => {
    switch (provider) {
      case "vercel_gateway":
        return "vercel-ai-gateway";
      case "openrouter":
        return "openrouter";
      default:
        return provider;
    }
  };

  // Load API key when dialog opens or provider changes
  useEffect(() => {
    const loadApiKey = async () => {
      if (!open) return;

      const providerId = getProviderIdForStorage(selectedProvider);
      setIsLoadingApiKey(true);
      try {
        const key = await getApiKey(providerId);
        setApiKeys((prev) => ({ ...prev, [selectedProvider]: key || "" }));
      } catch {
        // No API key saved yet, that's okay
        setApiKeys((prev) => ({ ...prev, [selectedProvider]: "" }));
      } finally {
        setIsLoadingApiKey(false);
      }
    };

    loadApiKey();
  }, [open, selectedProvider, getApiKey]);

  // Load custom models from backend
  useEffect(() => {
    if (!open) return;
    const providerModels = providersWithModels.find(
      (p) => p.id === getProviderIdForStorage(selectedProvider)
    );
    if (providerModels) {
      const custom = providerModels.models.filter((m) => m.isCustom);
      setCustomModels((prev) => ({
        ...prev,
        [selectedProvider]: custom.map((m) => m.modelId),
      }));
    }
  }, [open, selectedProvider, providersWithModels]);

  const handleSave = async () => {
    try {
      const providerId = getProviderIdForStorage(selectedProvider);
      const apiKey = apiKeys[selectedProvider] || "";

      // Save API key to keychain
      if (apiKey.trim()) {
        await invoke("keychain_set", {
          service: APP_IDENTITY.keychainService,
          key: `api_key:${providerId}`,
          value: apiKey,
        });
      }

      // Save model preference
      await invoke("configure_llm_provider_command", {
        providerId,
        modelId: customModel.trim() || model,
      });

      // Save gateway URLs
      if (selectedProvider === "vercel_gateway") {
        await invoke("keychain_set", {
          service: APP_IDENTITY.keychainService,
          key: "gateway-url-vercel-ai-gateway",
          value: vercelGatewayUrl,
        });
      } else if (selectedProvider === "openrouter") {
        await invoke("keychain_set", {
          service: APP_IDENTITY.keychainService,
          key: "gateway-url-openrouter",
          value: openrouterGatewayUrl,
        });
      }

      // Refresh provider status to show the API key is saved
      await loadProvidersWithKeyStatus();

      onOpenChange(false);
      showSuccess("API key saved successfully");
    } catch (error) {
      console.error("Failed to save configuration:", error);
      showError("Failed to save configuration");
    }
  };

  const handleTestConnection = async () => {
    const providerId = getProviderIdForStorage(selectedProvider);
    const apiKey = apiKeys[selectedProvider] || "";

    setIsTesting(true);
    setTestResult(null);

    try {
      const result = await useLLMStore
        .getState()
        .testProviderConnection(providerId, apiKey);
      setTestResult(result);
    } catch (error) {
      setTestResult({
        success: false,
        error: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setIsTesting(false);
    }
  };

  const handleAddCustomModel = async () => {
    const modelName = customModel.trim();
    if (!modelName) return;

    const providerModels = customModels[selectedProvider] || [];
    if (providerModels.includes(modelName)) return;

    const providerId = getProviderIdForStorage(selectedProvider);

    try {
      await useLLMStore
        .getState()
        .addCustomModel(providerId, modelName, modelName);
      setCustomModels((prev) => ({
        ...prev,
        [selectedProvider]: [...(prev[selectedProvider] || []), modelName],
      }));
      setModel(modelName);
      setCustomModel("");
    } catch (error) {
      console.error("Failed to add custom model:", error);
    }
  };

  const handleDeleteCustomModel = async (modelName: string) => {
    try {
      await useLLMStore.getState().deleteModel(modelName);
      setCustomModels((prev) => ({
        ...prev,
        [selectedProvider]: (prev[selectedProvider] || []).filter(
          (m) => m !== modelName
        ),
      }));

      if (model === modelName) {
        const providerModels = providersWithModels.find(
          (p) => p.id === getProviderIdForStorage(selectedProvider)
        );
        const firstModel = providerModels?.models.find((m) => !m.isCustom);
        if (firstModel) {
          setModel(firstModel.modelId);
        }
      }
    } catch (error) {
      console.error("Failed to delete custom model:", error);
    }
  };

  const handleProviderChange = (newProvider: Provider) => {
    setSelectedProvider(newProvider);

    // Set first available model
    const providerId = getProviderIdForStorage(newProvider);
    const providerModels = providersWithModels.find((p) => p.id === providerId);
    if (providerModels && providerModels.models.length > 0) {
      const firstModel = providerModels.models[0];
      setModel(firstModel.modelId);
    }

    setCustomModel("");
    setTestResult(null);
  };

  const handleTabChange = (tab: string) => {
    setActiveTab(tab as TabType);
    let newProvider: Provider;

    if (tab === "local") {
      newProvider = LOCAL_PROVIDERS[0];
    } else if (tab === "gateway") {
      newProvider = GATEWAY_PROVIDERS[0];
    } else {
      newProvider = AI_PROVIDERS[0];
    }

    handleProviderChange(newProvider);
  };

  const handleApiKeyChange = (value: string) => {
    if (selectedProvider !== "ollama") {
      setApiKeys((prev) => ({ ...prev, [selectedProvider]: value }));
    }
  };

  const getCurrentApiKey = () => {
    if (selectedProvider === "ollama") return "";
    return apiKeys[selectedProvider] || "";
  };

  const getApiKeyPlaceholder = () => {
    switch (selectedProvider) {
      case "openai":
        return "sk-...";
      case "anthropic":
        return "sk-ant-...";
      case "openrouter":
        return "sk-or-...";
      case "groq":
        return "gsk_...";
      case "vercel_gateway":
        return "API key";
      default:
        return "API key";
    }
  };

  const getProvidersForTab = (tab: TabType): Provider[] => {
    switch (tab) {
      case "local":
        return LOCAL_PROVIDERS;
      case "gateway":
        return GATEWAY_PROVIDERS;
      case "providers":
        return AI_PROVIDERS;
    }
  };

  const getProviderIcon = (provider: Provider) => {
    switch (provider) {
      case "ollama":
        return <span className="text-lg">🦙</span>;
      case "vercel_gateway":
        return <span className="text-lg">▲</span>;
      case "openrouter":
        return <span className="text-lg">→</span>;
      case "openai":
        return <span className="text-lg">🤖</span>;
      case "anthropic":
        return <span className="text-lg">🧠</span>;
      case "groq":
        return <span className="text-lg">⚡</span>;
    }
  };

  const getAvailableModels = () => {
    const providerId = getProviderIdForStorage(selectedProvider);
    const providerModels = providersWithModels.find((p) => p.id === providerId);
    return providerModels?.models || [];
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px] max-w-[95vw] max-h-[85vh] overflow-y-auto p-4 sm:p-6">
        <DialogHeader>
          <div className="flex items-center justify-between">
            <div>
              <DialogTitle>AI Provider & Model</DialogTitle>
              <DialogDescription>
                Configure your AI provider, API keys, and model selection.
                {isLoadingApiKey && (
                  <span className="text-xs text-muted-foreground ml-2">
                    (Loading API keys...)
                  </span>
                )}
              </DialogDescription>
            </div>
            <button
              onClick={() => onOpenChange(false)}
              className="rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <Tabs
            value={activeTab}
            onValueChange={handleTabChange}
            className="w-full"
          >
            <TabsList className="grid w-full grid-cols-3 h-11 sm:h-10">
              <TabsTrigger
                value="local"
                className="gap-1.5 data-[state=active]:bg-primary data-[state=active]:text-primary-foreground data-[state=active]:shadow-sm min-h-[44px] sm:min-h-0"
              >
                <Server className="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                <span className="hidden sm:inline">Local</span>
              </TabsTrigger>
              <TabsTrigger
                value="gateway"
                className="gap-1.5 data-[state=active]:bg-primary data-[state=active]:text-primary-foreground data-[state=active]:shadow-sm min-h-[44px] sm:min-h-0"
              >
                <Cloud className="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                <span className="hidden sm:inline">AI Gateway</span>
              </TabsTrigger>
              <TabsTrigger
                value="providers"
                className="gap-1.5 data-[state=active]:bg-primary data-[state=active]:text-primary-foreground data-[state=active]:shadow-sm min-h-[44px] sm:min-h-0"
              >
                <Cpu className="h-4 w-4 sm:h-3.5 sm:w-3.5" />
                <span className="hidden sm:inline">AI Providers</span>
              </TabsTrigger>
            </TabsList>

            <TabsContent value="local" className="mt-4 space-y-4">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">Local Provider</label>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-xs text-muted-foreground hover:text-foreground"
                  onClick={() => window.open("https://ollama.com", "_blank")}
                >
                  <ExternalLink className="h-3 w-3 mr-1" />
                  Resources
                </Button>
              </div>
              <div className="flex flex-wrap gap-1.5">
                {getProvidersForTab("local").map((p) => (
                  <Button
                    key={p}
                    variant={selectedProvider === p ? "default" : "outline"}
                    onClick={() => handleProviderChange(p)}
                    className={`text-xs h-8 px-2.5 gap-1.5 ${selectedProvider === p ? "" : "text-muted-foreground"}`}
                    size="sm"
                  >
                    {getProviderIcon(p)}
                    <span className="truncate">{PROVIDER_NAMES[p]}</span>
                  </Button>
                ))}
              </div>
              <div className="space-y-1.5">
                <label className="text-xs text-muted-foreground font-medium">
                  Server URL
                </label>
                <Input
                  type="text"
                  placeholder="http://localhost:11434/v1"
                  value={ollamaBaseUrl}
                  onChange={(e) => setOllamaBaseUrl(e.target.value)}
                  className="h-9 text-sm"
                />
              </div>
            </TabsContent>

            <TabsContent value="gateway" className="mt-4 space-y-4">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">AI Gateway</label>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-xs text-muted-foreground hover:text-foreground"
                  onClick={() =>
                    window.open("https://vercel.com/docs/ai-gateway", "_blank")
                  }
                >
                  <ExternalLink className="h-3 w-3 mr-1" />
                  Resources
                </Button>
              </div>
              <div className="flex flex-wrap gap-1.5">
                {getProvidersForTab("gateway").map((p) => (
                  <Button
                    key={p}
                    variant={selectedProvider === p ? "default" : "outline"}
                    onClick={() => handleProviderChange(p)}
                    className={`text-xs h-8 px-2.5 gap-1.5 ${selectedProvider === p ? "" : "text-muted-foreground"}`}
                    size="sm"
                  >
                    {getProviderIcon(p)}
                    <span className="truncate">{PROVIDER_NAMES[p]}</span>
                  </Button>
                ))}
              </div>
              <div className="space-y-1.5">
                <label className="text-xs text-muted-foreground font-medium">
                  API Key
                </label>
                <div className="relative">
                  <Input
                    type={showApiKey ? "text" : "password"}
                    placeholder={getApiKeyPlaceholder()}
                    value={getCurrentApiKey()}
                    onChange={(e) => handleApiKeyChange(e.target.value)}
                    disabled={isLoadingApiKey}
                    className="pr-9 h-9 text-sm"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="absolute right-0 top-0 h-full px-2.5 hover:bg-transparent"
                    onClick={() => setShowApiKey(!showApiKey)}
                  >
                    {isLoadingApiKey ? (
                      <Loader2 className="h-3.5 w-3.5 text-muted-foreground animate-spin" />
                    ) : showApiKey ? (
                      <EyeOff className="h-3.5 w-3.5 text-muted-foreground" />
                    ) : (
                      <Eye className="h-3.5 w-3.5 text-muted-foreground" />
                    )}
                  </Button>
                </div>
              </div>
              {selectedProvider === "vercel_gateway" && (
                <div className="space-y-1.5">
                  <label className="text-xs text-muted-foreground font-medium">
                    Gateway URL
                  </label>
                  <Input
                    type="text"
                    placeholder="https://ai-gateway.vercel.sh/v1"
                    value={vercelGatewayUrl}
                    onChange={(e) => setVercelGatewayUrl(e.target.value)}
                    className="h-9 text-sm"
                  />
                </div>
              )}
              {selectedProvider === "openrouter" && (
                <div className="space-y-1.5">
                  <label className="text-xs text-muted-foreground font-medium">
                    Gateway URL
                  </label>
                  <Input
                    type="text"
                    placeholder="https://openrouter.ai/api/v1"
                    value={openrouterGatewayUrl}
                    onChange={(e) => setOpenrouterGatewayUrl(e.target.value)}
                    className="h-9 text-sm"
                  />
                </div>
              )}
            </TabsContent>

            <TabsContent value="providers" className="mt-4 space-y-4">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">AI Provider</label>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-xs text-muted-foreground hover:text-foreground"
                  onClick={() => {
                    const url =
                      selectedProvider === "openai"
                        ? "https://platform.openai.com"
                        : selectedProvider === "anthropic"
                          ? "https://console.anthropic.com"
                          : "https://groq.com";
                    window.open(url, "_blank");
                  }}
                >
                  <ExternalLink className="h-3 w-3 mr-1" />
                  Resources
                </Button>
              </div>
              <div className="flex flex-wrap gap-1.5">
                {getProvidersForTab("providers").map((p) => (
                  <Button
                    key={p}
                    variant={selectedProvider === p ? "default" : "outline"}
                    onClick={() => handleProviderChange(p)}
                    className={`text-xs h-8 px-2.5 gap-1.5 ${selectedProvider === p ? "" : "text-muted-foreground"}`}
                    size="sm"
                  >
                    {getProviderIcon(p)}
                    <span className="truncate">{PROVIDER_NAMES[p]}</span>
                  </Button>
                ))}
              </div>
              <div className="space-y-1.5">
                <label className="text-xs text-muted-foreground font-medium">
                  API Key
                </label>
                <div className="relative">
                  <Input
                    type={showApiKey ? "text" : "password"}
                    placeholder={getApiKeyPlaceholder()}
                    value={getCurrentApiKey()}
                    onChange={(e) => handleApiKeyChange(e.target.value)}
                    disabled={isLoadingApiKey}
                    className="pr-9 h-9 text-sm"
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    className="absolute right-0 top-0 h-full px-2.5 hover:bg-transparent"
                    onClick={() => setShowApiKey(!showApiKey)}
                  >
                    {isLoadingApiKey ? (
                      <Loader2 className="h-3.5 w-3.5 text-muted-foreground animate-spin" />
                    ) : showApiKey ? (
                      <EyeOff className="h-3.5 w-3.5 text-muted-foreground" />
                    ) : (
                      <Eye className="h-3.5 w-3.5 text-muted-foreground" />
                    )}
                  </Button>
                </div>
              </div>
            </TabsContent>
          </Tabs>

          <div className="space-y-1.5">
            <label className="text-xs text-muted-foreground font-medium">
              Model
            </label>
            <select
              value={model}
              onChange={(e) => {
                setModel(e.target.value);
                setCustomModel("");
              }}
              className="w-full h-9 px-3 text-sm rounded-md border bg-background focus:outline-none focus:ring-1 focus:ring-primary/50"
            >
              {getAvailableModels().map((m) => (
                <option key={m.id} value={m.modelId}>
                  {m.displayName}
                </option>
              ))}
              {(customModels[selectedProvider] || []).map((m) => (
                <option key={m} value={m}>
                  {m} (custom)
                </option>
              ))}
            </select>
            <div className="flex gap-1.5 mt-1.5">
              <Input
                type="text"
                placeholder="Custom model ID..."
                value={customModel}
                onChange={(e) => setCustomModel(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    handleAddCustomModel();
                  }
                }}
                className="h-8 text-xs"
              />
              <Button
                variant="outline"
                size="sm"
                onClick={handleAddCustomModel}
                disabled={!customModel.trim()}
                className="h-8 px-2"
              >
                <Plus className="h-3.5 w-3.5" />
              </Button>
            </div>
            {(customModels[selectedProvider] || []).length > 0 && (
              <div className="space-y-1 mt-2">
                <div className="text-xs text-muted-foreground font-bold">
                  Custom Models:
                </div>
                <div className="space-y-1">
                  {(customModels[selectedProvider] || []).map((m) => (
                    <div
                      key={m}
                      className="flex items-center justify-between p-1.5 rounded bg-muted/50 border"
                    >
                      <span className="text-xs truncate">{m}</span>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleDeleteCustomModel(m)}
                        className="h-5 w-5 p-0 ml-1 shrink-0 text-muted-foreground hover:text-destructive"
                      >
                        <Trash2 className="h-3 w-3" />
                      </Button>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>

          {/* Test Connection Section */}
          {selectedProvider !== "ollama" && (
            <div className="space-y-1.5 pt-2 border-t">
              <div className="flex items-center justify-between">
                <label className="text-xs text-muted-foreground font-medium">
                  Test Connection
                </label>
              </div>
              <div className="flex gap-2">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={handleTestConnection}
                  disabled={isTesting || !getCurrentApiKey()}
                  className="flex-1"
                >
                  {isTesting ? (
                    <>
                      <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                      Testing...
                    </>
                  ) : (
                    "Test Connection"
                  )}
                </Button>
              </div>
              {testResult && (
                <div
                  className={`flex items-start gap-2 p-2 rounded-md text-xs ${
                    testResult.success
                      ? "bg-green-50 text-green-800 border border-green-200"
                      : "bg-red-50 text-red-800 border border-red-200"
                  }`}
                >
                  {testResult.success ? (
                    <CheckCircle2 className="h-4 w-4 shrink-0 mt-0.5" />
                  ) : (
                    <XCircle className="h-4 w-4 shrink-0 mt-0.5" />
                  )}
                  <div className="flex-1">
                    <div className="font-medium">
                      {testResult.success
                        ? "Connection successful"
                        : "Connection failed"}
                    </div>
                    {testResult.error && (
                      <div className="text-xs opacity-90 mt-0.5">
                        {testResult.error}
                      </div>
                    )}
                    {testResult.latencyMs !== undefined && (
                      <div className="text-xs opacity-75 mt-0.5">
                        Latency: {testResult.latencyMs}ms
                      </div>
                    )}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSave}>Save</Button>
        </DialogFooter>

        {/* Add Custom Model Dialog */}
        <AddCustomModelDialog
          open={showAddModel}
          onOpenChange={setShowAddModel}
          providerId={getProviderIdForStorage(selectedProvider)}
          providerName={PROVIDER_NAMES[selectedProvider]}
          onAdd={async (modelId, displayName) => {
            await useLLMStore
              .getState()
              .addCustomModel(
                getProviderIdForStorage(selectedProvider),
                modelId,
                displayName
              );
          }}
        />
      </DialogContent>
    </Dialog>
  );
}
