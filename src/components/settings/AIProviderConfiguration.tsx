/**
 * Reusable AI Provider Configuration component.
 * Used in both onboarding and settings pages.
 */
import { useEffect, useState } from "react";

import type {
  GlobalDefaultModel,
  InitialModelSpec,
  ProviderWithKeyStatus,
  ProviderWithModels,
  TestResult,
} from "@/lib/models";

import { AIModelQuickAccess } from "@/components/ai/AIModelQuickAccess";
import { AddProviderDialog } from "@/components/settings/AddProviderDialog";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import {
  AI_GATEWAY_PROVIDERS,
  AI_PROVIDERS,
  LOCAL_PROVIDERS,
  MODELS,
  updateOllamaModels,
  type ModelDefinition,
} from "@/lib/ai-models";
import {
  Check,
  ChevronsUpDown,
  Edit2,
  Eye,
  EyeOff,
  Loader2,
  Plus,
  RefreshCw,
  Trash2,
} from "@/lib/icons";
import { showError, showInfo, showSuccess } from "@/lib/toast";
import { useLLMStore } from "@/stores/llm";

interface ProviderCardProps {
  provider: ProviderWithKeyStatus;
  providerWithModels?: ProviderWithModels;
  isDefault: boolean;
  onSetDefault: (providerId: string) => void;
  onConfigure: (provider: ProviderWithKeyStatus) => void;
  onEdit: (provider: ProviderWithKeyStatus) => void;
  onAddCustomModel?: (
    providerId: string,
    modelId: string,
    displayName: string
  ) => Promise<void>;
  onDeleteModel?: (modelId: string) => Promise<void>;
  onRefreshOllamaModels?: () => Promise<void>;
  onDeleteProvider?: (providerId: string) => Promise<void>;
}

function ProviderCard({
  provider,
  providerWithModels,
  isDefault,
  onSetDefault,
  onConfigure,
  onEdit,
  onAddCustomModel,
  onDeleteModel,
  onRefreshOllamaModels,
  onDeleteProvider,
}: ProviderCardProps) {
  const [showApiKey, setShowApiKey] = useState(false);
  const [apiKey, setApiKey] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [testResult, setTestResult] = useState<TestResult | null>(null);
  const [isTesting, setIsTesting] = useState(false);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [showAddModelDialog, setShowAddModelDialog] = useState(false);
  const [newModelId, setNewModelId] = useState("");
  const [isAddingModel, setIsAddingModel] = useState(false);
  const {
    getApiKey,
    saveApiKeyForProvider,
    testProviderConnection,
    loadProvidersWithKeyStatus,
    addCustomModel,
    deleteModel,
  } = useLLMStore();

  useEffect(() => {
    if (provider.hasApiKey || !provider.requiresApiKey) {
      getApiKey(provider.id)
        .then(setApiKey)
        .catch(() => setApiKey(""));
    }
  }, [provider.id, provider.hasApiKey, provider.requiresApiKey, getApiKey]);

  const handleSaveApiKey = async () => {
    setIsSaving(true);
    try {
      await saveApiKeyForProvider(provider.id, apiKey);
      setTestResult(null);
      await loadProvidersWithKeyStatus();
      showSuccess("API key saved successfully");
    } catch (error) {
      console.error("Failed to save API key:", error);
      showError("Failed to save API key");
    } finally {
      setIsSaving(false);
    }
  };

  const handleTestConnection = async () => {
    if (!apiKey && provider.requiresApiKey) return;

    setIsTesting(true);
    setTestResult(null);
    try {
      const result = await testProviderConnection(provider.id, apiKey);
      setTestResult(result);
      if (result.success) {
        showSuccess(
          result.latencyMs
            ? `Connection successful (${result.latencyMs}ms)`
            : "Connection successful"
        );
      } else {
        showError(result.error || "Connection test failed");
      }
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : String(error);
      setTestResult({
        success: false,
        error: errorMessage,
      });
      showError(errorMessage || "Connection test failed");
    } finally {
      setIsTesting(false);
    }
  };

  const handleAddCustomModel = async () => {
    if (!newModelId.trim()) {
      showError("Please enter a model ID");
      return;
    }

    setIsAddingModel(true);
    try {
      await addCustomModel(provider.id, newModelId.trim(), newModelId.trim());
      showSuccess(
        `Successfully added model "${newModelId}" to ${provider.name}`
      );
      setNewModelId("");
      setShowAddModelDialog(false);
    } catch (error) {
      console.error("Failed to add custom model:", error);
      const errorMsg = error instanceof Error ? error.message : String(error);
      showError(errorMsg);
    } finally {
      setIsAddingModel(false);
    }
  };

  const handleDeleteModel = async (modelId: string) => {
    try {
      await deleteModel(modelId);
      showSuccess("Model deleted successfully");
    } catch (error) {
      console.error("Failed to delete model:", error);
      showError(
        error instanceof Error ? error.message : "Failed to delete model"
      );
    }
  };

  const customModels =
    providerWithModels?.models.filter(
      (m: { isCustom: boolean }) => m.isCustom
    ) || [];

  const discoveredModels =
    providerWithModels?.models.filter(
      (m: { isCustom: boolean }) => !m.isCustom
    ) || [];

  return (
    <div className="rounded-lg border bg-card p-4 space-y-4">
      {/* Header with status */}
      <div className="flex items-start justify-between gap-2">
        <div className="min-w-0 flex-1 space-y-1">
          <div className="flex items-center gap-2 flex-wrap">
            <h3 className="font-semibold truncate">{provider.name}</h3>
            {(provider.hasApiKey || !provider.requiresApiKey) && (
              <div className="flex items-center gap-1 text-xs text-green-600">
                <div className="h-2 w-2 rounded-full bg-green-600" />
                <span>{!provider.requiresApiKey ? "Local" : "Configured"}</span>
              </div>
            )}
          </div>
          <p className="text-sm text-muted-foreground truncate">
            {provider.baseUrl}
          </p>
        </div>

        <div className="flex items-center gap-1 shrink-0">
          {onRefreshOllamaModels && (
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => {
                setIsRefreshing(true);
                onRefreshOllamaModels().finally(() => {
                  setIsRefreshing(false);
                });
              }}
              disabled={isRefreshing}
              title="Refresh models"
            >
              {isRefreshing ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <RefreshCw className="h-4 w-4" />
              )}
            </Button>
          )}
          {onAddCustomModel && (
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8"
              onClick={() => setShowAddModelDialog(true)}
              title="Add custom model"
            >
              <Plus className="h-4 w-4" />
            </Button>
          )}
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={() => onEdit(provider)}
            title="Edit provider"
          >
            <Edit2 className="h-4 w-4" />
          </Button>
          {onDeleteProvider && provider.isUserDefined && (
            <Button
              variant="ghost"
              size="icon"
              className="h-8 w-8 text-destructive"
              onClick={() => onDeleteProvider(provider.id)}
              title="Delete provider"
            >
              <Trash2 className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      {/* API Key Input */}
      {provider.requiresApiKey && (
        <div className="space-y-2">
          <label className="text-sm font-medium">API Key</label>
          <div className="flex gap-2">
            <div className="relative flex-1">
              <Input
                type={showApiKey ? "text" : "password"}
                placeholder="Enter API key"
                value={apiKey}
                onChange={(e) => setApiKey(e.target.value)}
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
            <Button
              onClick={handleSaveApiKey}
              disabled={isSaving || apiKey === ""}
              variant="outline"
              size="sm"
            >
              {isSaving ? <Loader2 className="h-4 w-4 animate-spin" /> : "Save"}
            </Button>
          </div>
        </div>
      )}

      {/* Test Result */}
      {testResult && (
        <div
          className={`text-sm p-2 rounded ${
            testResult.success
              ? "bg-green-50 text-green-700 dark:bg-green-950 dark:text-green-400"
              : "bg-red-50 text-red-700 dark:bg-red-950 dark:text-red-400"
          }`}
        >
          {testResult.success
            ? `Connection successful!${testResult.latencyMs ? ` (${testResult.latencyMs}ms)` : ""}`
            : testResult.error || "Connection failed"}
        </div>
      )}

      {/* Custom Models Section */}
      {customModels.length > 0 && (
        <Collapsible defaultOpen={false}>
          <CollapsibleTrigger className="flex items-center justify-between w-full">
            <label className="text-sm font-bold">Custom Models</label>
            <div className="flex items-center gap-2">
              <span className="text-xs text-orange-600">
                {customModels.length} model(s)
              </span>
              <ChevronsUpDown className="h-4 w-4 text-muted-foreground" />
            </div>
          </CollapsibleTrigger>
          <CollapsibleContent className="space-y-1 mt-2">
            {customModels.map(
              (model: { id: string; modelId: string; displayName: string }) => (
                <div
                  key={model.id}
                  className="flex items-center justify-between rounded-md border bg-muted/50 p-2"
                >
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-medium truncate">
                      {model.displayName}
                    </p>
                    <p className="text-xs text-muted-foreground truncate">
                      {model.modelId}
                    </p>
                  </div>
                  {onDeleteModel && (
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-7 w-7 text-destructive"
                      onClick={() => handleDeleteModel(model.id)}
                      title="Delete model"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </Button>
                  )}
                </div>
              )
            )}
          </CollapsibleContent>
        </Collapsible>
      )}

      {/* Action Buttons */}
      <div className="flex gap-2">
        <Button
          variant="outline"
          size="sm"
          className="flex-1"
          onClick={() => onConfigure(provider)}
        >
          Configure
        </Button>
        <Button
          variant="outline"
          size="sm"
          className="flex-1"
          onClick={handleTestConnection}
          disabled={provider.requiresApiKey && !apiKey}
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
        {!isDefault && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onSetDefault(provider.id)}
            title="Set as default"
          >
            <Check className="h-4 w-4" />
          </Button>
        )}
      </div>

      {/* Discovered Models Section - Only for local providers like Ollama */}
      {provider.id === "ollama" && discoveredModels.length > 0 && (
        <div className="pt-2 border-t">
          <Collapsible defaultOpen={false}>
            <CollapsibleTrigger className="flex items-center justify-between w-full">
              <label className="text-sm font-bold">Discovered Models</label>
              <div className="flex items-center gap-2">
                <span className="text-xs text-blue-600">
                  {discoveredModels.length} model(s)
                </span>
                <ChevronsUpDown className="h-4 w-4 text-muted-foreground" />
              </div>
            </CollapsibleTrigger>
            <CollapsibleContent className="space-y-1 mt-2">
              {discoveredModels.map(
                (model: {
                  id: string;
                  modelId: string;
                  displayName: string;
                }) => (
                  <div
                    key={model.id}
                    className="flex items-center justify-between rounded-md border bg-muted/50 p-2"
                  >
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">
                        {model.displayName}
                      </p>
                      <p className="text-xs text-muted-foreground truncate">
                        {model.modelId}
                      </p>
                    </div>
                  </div>
                )
              )}
            </CollapsibleContent>
          </Collapsible>
        </div>
      )}

      {/* Add Custom Model Dialog */}
      <Dialog
        open={showAddModelDialog}
        onOpenChange={(open) => !open && setShowAddModelDialog(false)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Custom Model to {provider.name}</DialogTitle>
            <DialogDescription>
              Enter the model ID (e.g., claude-sonnet-4-5-20250929 or
              gpt-4-turbo)
            </DialogDescription>
          </DialogHeader>
          <div className="py-4 space-y-4">
            <div className="space-y-2">
              <label className="text-sm font-medium">Model ID</label>
              <Input
                placeholder="Enter model ID"
                value={newModelId}
                onChange={(e) => setNewModelId(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && newModelId.trim()) {
                    handleAddCustomModel();
                  }
                }}
                autoFocus
              />
            </div>
            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowAddModelDialog(false)}
                disabled={isAddingModel}
              >
                Cancel
              </Button>
              <Button
                size="sm"
                onClick={handleAddCustomModel}
                disabled={isAddingModel || !newModelId.trim()}
              >
                {isAddingModel ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  "Add Model"
                )}
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export interface AIProviderConfigurationProps {
  /** Show the Global Default Model section */
  showGlobalDefault?: boolean;
  /** Show the header with title and action buttons */
  showHeader?: boolean;
  /** Compact mode for onboarding (fewer features) */
  compact?: boolean;
}

export function AIProviderConfiguration({
  showGlobalDefault = true,
  showHeader = true,
  compact = false,
}: AIProviderConfigurationProps) {
  const {
    config,
    providersWithKeyStatus,
    providersWithModels,
    loadProvidersAndModels,
    loadProvidersWithKeyStatus,
    loadAllApiKeys,
    saveProviderConfig,
    discoverOllamaModels,
    seedStandardModels,
    resetAndSeedModels,
    addCustomModel,
    deleteModel,
    updateProvider,
    addUserProvider,
    deleteUserProvider,
    getGlobalDefaultModel,
    setGlobalDefaultModel,
  } = useLLMStore();

  const [selectedProvider, setSelectedProvider] =
    useState<ProviderWithKeyStatus | null>(null);
  const [showResetDialog, setShowResetDialog] = useState(false);
  const [showEditDialog, setShowEditDialog] = useState(false);
  const [editingBaseUrl, setEditingBaseUrl] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [activeTab, setActiveTab] = useState<
    "gateway" | "providers" | "local" | "user-defined"
  >("gateway");
  const [selectedModel, setSelectedModel] = useState("");
  const [refreshMessage, setRefreshMessage] = useState<{
    type: "success" | "error";
    message: string;
  } | null>(null);
  const [showAddProviderDialog, setShowAddProviderDialog] = useState(false);
  const [globalDefaultModel, setGlobalDefaultModelState] =
    useState<GlobalDefaultModel | null>(null);
  const [showDeleteProviderDialog, setShowDeleteProviderDialog] =
    useState(false);
  const [showGlobalModelSelector, setShowGlobalModelSelector] = useState(false);
  const [providerToDelete, setProviderToDelete] = useState<string | null>(null);

  const getExpectedModelCount = () => {
    let count = 0;
    for (const providerId in MODELS) {
      if (providerId !== "ollama") {
        count += MODELS[providerId].models.length;
      }
    }
    return count;
  };

  const getActualModelCount = () => {
    return providersWithModels.reduce((count, provider) => {
      if (provider.id !== "ollama") {
        return count + provider.models.length;
      }
      return count;
    }, 0);
  };

  useEffect(() => {
    const initializeProviders = async () => {
      await loadProvidersAndModels();
      await loadProvidersWithKeyStatus();
      await loadAllApiKeys();

      const expectedCount = getExpectedModelCount();
      const actualCount = getActualModelCount();

      if (actualCount < expectedCount) {
        try {
          const insertedCount = await seedStandardModels();
          if (insertedCount > 0) {
            showSuccess(
              `Updated model registry: ${insertedCount} new model(s) added`
            );
            await loadProvidersAndModels();
          }
        } catch (error) {
          console.error("Failed to auto-seed models:", error);
        }
      }

      const { providersWithModels: latestModels } = useLLMStore.getState();
      const ollamaProvider = latestModels.find((p) => p.id === "ollama");
      if (ollamaProvider && ollamaProvider.models.length === 0) {
        try {
          const addedCount = await discoverOllamaModels();
          if (addedCount > 0) {
            showSuccess(
              `Auto-discovered ${addedCount} Ollama model(s)`
            );
            const { providersWithModels: updatedModels } =
              useLLMStore.getState();
            const updatedOllamaProvider = updatedModels.find(
              (p) => p.id === "ollama"
            );
            if (
              updatedOllamaProvider &&
              updatedOllamaProvider.models.length > 0
            ) {
              const ollamaModelDefs: ModelDefinition[] =
                updatedOllamaProvider.models.map((m) => ({
                  id: m.modelId,
                  displayName: m.displayName,
                  contextLimit: m.contextLimit || 128000,
                }));
              updateOllamaModels(ollamaModelDefs);
            }
          }
        } catch {
          console.log("Ollama auto-discovery skipped");
        }
      } else if (ollamaProvider && ollamaProvider.models.length > 0) {
        const ollamaModelDefs: ModelDefinition[] = ollamaProvider.models.map(
          (m) => ({
            id: m.modelId,
            displayName: m.displayName,
            contextLimit: m.contextLimit || 128000,
          })
        );
        updateOllamaModels(ollamaModelDefs);
      }
    };

    initializeProviders();
  }, [
    loadProvidersAndModels,
    loadProvidersWithKeyStatus,
    loadAllApiKeys,
    discoverOllamaModels,
  ]);

  useEffect(() => {
    getGlobalDefaultModel().then(setGlobalDefaultModelState);
  }, [getGlobalDefaultModel]);

  const gatewayProviders = providersWithKeyStatus.filter((p) =>
    AI_GATEWAY_PROVIDERS.includes(p.id as any)
  );
  const aiProviders = providersWithKeyStatus.filter((p) =>
    AI_PROVIDERS.includes(p.id as any)
  );
  const localProviders = providersWithKeyStatus.filter((p) =>
    LOCAL_PROVIDERS.includes(p.id as any)
  );
  const userDefinedProviders = providersWithKeyStatus.filter(
    (p) => p.isUserDefined
  );

  const currentProviders =
    activeTab === "gateway"
      ? gatewayProviders
      : activeTab === "providers"
        ? aiProviders
        : activeTab === "local"
          ? localProviders
          : userDefinedProviders;

  const handleSetDefault = (providerId: string) => {
    const provider = providersWithModels.find((p) => p.id === providerId);
    if (provider && provider.models.length > 0) {
      saveProviderConfig(providerId, provider.models[0].modelId);
    }
  };

  const handleConfigure = (provider: ProviderWithKeyStatus) => {
    setSelectedProvider(provider);
    const providerWithModels = providersWithModels.find(
      (p) => p.id === provider.id
    );
    if (providerWithModels && providerWithModels.models.length > 0) {
      setSelectedModel(providerWithModels.models[0].modelId);
    }
  };

  const handleEdit = (provider: ProviderWithKeyStatus) => {
    setSelectedProvider(provider);
    setEditingBaseUrl(provider.baseUrl);
    setShowEditDialog(true);
  };

  const handleSaveBaseUrl = async () => {
    if (!selectedProvider) return;

    setIsSaving(true);
    try {
      await updateProvider(selectedProvider.id, editingBaseUrl);
      showSuccess(`Successfully updated ${selectedProvider.name} base URL`);
      setShowEditDialog(false);
      setSelectedProvider(null);
    } catch (error) {
      console.error("Failed to update provider:", error);
      showError(
        error instanceof Error ? error.message : "Failed to update provider"
      );
    } finally {
      setIsSaving(false);
    }
  };

  const confirmReset = async () => {
    setShowResetDialog(false);
    try {
      const seededCount = await resetAndSeedModels();
      showSuccess(`Reset and seeded ${seededCount} model(s)`);
      setRefreshMessage({
        type: "success",
        message: `Reset complete: ${seededCount} model(s)`,
      });
      setTimeout(() => setRefreshMessage(null), 5000);
    } catch (error) {
      console.error("Failed to reset models:", error);
      const errorMsg =
        error instanceof Error ? error.message : "Failed to reset models";
      showError(errorMsg);
    }
  };

  const handleConfigureModelSelect = (modelId: string) => {
    if (selectedProvider) {
      saveProviderConfig(selectedProvider.id, modelId);
      setSelectedProvider(null);
    }
  };

  const handleRefreshOllamaModels = async () => {
    try {
      const addedCount = await discoverOllamaModels();

      const ollamaProvider = providersWithModels.find((p) => p.id === "ollama");
      if (ollamaProvider && ollamaProvider.models.length > 0) {
        const ollamaModelDefs: ModelDefinition[] = ollamaProvider.models.map(
          (m) => ({
            id: m.modelId,
            displayName: m.displayName,
            contextLimit: m.contextLimit || 128000,
          })
        );
        updateOllamaModels(ollamaModelDefs);
      }

      if (addedCount > 0) {
        showSuccess(`Discovered ${addedCount} new Ollama model(s)`);
      } else {
        showInfo("Ollama models are already up to date");
      }
    } catch (error) {
      const errorMsg =
        error instanceof Error ? error.message : "Failed to discover models";
      showError(errorMsg);
    }
  };

  const handleAddUserProvider = async (
    id: string,
    name: string,
    baseUrl: string,
    requiresApiKey: boolean,
    initialModels: InitialModelSpec[],
    apiKey?: string
  ) => {
    try {
      await addUserProvider(
        id,
        name,
        baseUrl,
        requiresApiKey,
        initialModels,
        apiKey
      );
      showSuccess(`Successfully added provider "${name}"`);
      setActiveTab("user-defined");
    } catch (error) {
      const errorMsg =
        error instanceof Error ? error.message : "Failed to add provider";
      showError(errorMsg);
      throw error;
    }
  };

  const handleDeleteUserProvider = async (providerId: string) => {
    setProviderToDelete(providerId);
    setShowDeleteProviderDialog(true);
  };

  const confirmDeleteProvider = async () => {
    if (!providerToDelete) return;

    try {
      await deleteUserProvider(providerToDelete);
      showSuccess("Provider deleted successfully");
      setShowDeleteProviderDialog(false);
      setProviderToDelete(null);
    } catch (error) {
      const errorMsg =
        error instanceof Error ? error.message : "Failed to delete provider";
      showError(errorMsg);
    }
  };

  const handleSetGlobalDefaultModel = async (
    providerId: string,
    modelId: string
  ) => {
    try {
      await setGlobalDefaultModel(providerId, modelId);
      setGlobalDefaultModelState({ providerId, modelId });
      showSuccess("Global default model updated");
    } catch (error) {
      const errorMsg =
        error instanceof Error ? error.message : "Failed to set default model";
      showError(errorMsg);
    }
  };

  const getGlobalDefaultModelDisplayName = () => {
    if (!globalDefaultModel) return "Select Model";
    const provider = providersWithModels.find(
      (p) => p.id === globalDefaultModel.providerId
    );
    if (!provider) return globalDefaultModel.modelId;
    const model = provider.models.find(
      (m) => m.modelId === globalDefaultModel.modelId
    );
    return model?.displayName || globalDefaultModel.modelId;
  };

  return (
    <div className="space-y-6">
      {/* Global Default Model Section */}
      {showGlobalDefault && (
        <div className="mb-6 p-4 border rounded-lg bg-muted/30">
          <div className="flex items-center justify-between">
            <div>
              <h3 className="text-sm font-medium">Global Default Model</h3>
              <p className="text-xs text-muted-foreground">
                Used across all workspaces unless overridden
              </p>
            </div>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setShowGlobalModelSelector(true)}
            >
              {getGlobalDefaultModelDisplayName()}
            </Button>
          </div>
        </div>
      )}

      {/* Header */}
      {showHeader && (
        <div className="flex items-start justify-between gap-4">
          <div>
            <h2 className="text-lg font-semibold">AI Providers</h2>
            <p className="text-sm text-muted-foreground">
              Manage your AI provider configurations
            </p>
          </div>

          {!compact && (
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowAddProviderDialog(true)}
              >
                <Plus className="h-4 w-4 mr-1" />
                Add Provider
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowResetDialog(true)}
              >
                Reset Models
              </Button>
            </div>
          )}
        </div>
      )}

      {/* Provider Category Tabs */}
      <div className="border-b border-border">
        <div className="flex gap-4">
          <button
            type="button"
            className={`pb-2 px-1 text-sm font-bold border-b-2 transition-colors ${
              activeTab === "gateway"
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
            onClick={() => setActiveTab("gateway")}
          >
            AI Gateway
          </button>
          <button
            type="button"
            className={`pb-2 px-1 text-sm font-bold border-b-2 transition-colors ${
              activeTab === "providers"
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
            onClick={() => setActiveTab("providers")}
          >
            AI Providers
          </button>
          <button
            type="button"
            className={`pb-2 px-1 text-sm font-bold border-b-2 transition-colors ${
              activeTab === "local"
                ? "border-primary text-primary"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
            onClick={() => setActiveTab("local")}
          >
            Local
          </button>
          {userDefinedProviders.length > 0 && (
            <button
              type="button"
              className={`pb-2 px-1 text-sm font-bold border-b-2 transition-colors ${
                activeTab === "user-defined"
                  ? "border-primary text-primary"
                  : "border-transparent text-muted-foreground hover:text-foreground"
              }`}
              onClick={() => setActiveTab("user-defined")}
            >
              User Defined
            </button>
          )}
        </div>
      </div>

      {/* Provider Cards Grid */}
      <div className="grid gap-4 sm:grid-cols-2">
        {currentProviders.map((provider) => {
          const providerWithModels = providersWithModels.find(
            (p) => p.id === provider.id
          );
          return (
            <ProviderCard
              key={provider.id}
              provider={provider}
              providerWithModels={providerWithModels}
              isDefault={config?.providerId === provider.id}
              onSetDefault={handleSetDefault}
              onConfigure={handleConfigure}
              onEdit={handleEdit}
              onAddCustomModel={async (providerId, modelId, displayName) => {
                await addCustomModel(providerId, modelId, displayName);
                await loadProvidersAndModels();
              }}
              onDeleteModel={async (modelId) => {
                await deleteModel(modelId);
                await loadProvidersAndModels();
              }}
              onRefreshOllamaModels={
                provider.id === "ollama"
                  ? handleRefreshOllamaModels
                  : undefined
              }
              onDeleteProvider={
                provider.isUserDefined ? handleDeleteUserProvider : undefined
              }
            />
          );
        })}
      </div>

      {/* Refresh Message */}
      {refreshMessage && (
        <div
          className={`text-sm p-3 rounded ${
            refreshMessage.type === "success"
              ? "bg-green-50 text-green-700 dark:bg-green-950 dark:text-green-400"
              : "bg-red-50 text-red-700 dark:bg-red-950 dark:text-red-400"
          }`}
        >
          {refreshMessage.message}
        </div>
      )}

      {currentProviders.length === 0 && (
        <div className="text-center py-8 text-muted-foreground">
          No providers found in this category.
        </div>
      )}

      {/* Configure Dialog */}
      {selectedProvider && (
        <Dialog
          open={!!selectedProvider}
          onOpenChange={(open) => !open && setSelectedProvider(null)}
        >
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Configure {selectedProvider.name}</DialogTitle>
              <DialogDescription>
                Select the model to use with {selectedProvider.name}
              </DialogDescription>
            </DialogHeader>
            <div className="py-4">
              <Select
                value={selectedModel}
                onValueChange={handleConfigureModelSelect}
                options={
                  providersWithModels
                    .find((p) => p.id === selectedProvider.id)
                    ?.models.map((model) => ({
                      value: model.modelId,
                      label: model.displayName,
                    })) || []
                }
                placeholder="Select a model"
              />
            </div>
          </DialogContent>
        </Dialog>
      )}

      {/* Edit Provider Dialog */}
      {selectedProvider && (
        <Dialog
          open={showEditDialog}
          onOpenChange={(open) => {
            if (!open) {
              setShowEditDialog(false);
              setSelectedProvider(null);
            }
          }}
        >
          <DialogContent>
            <DialogHeader>
              <DialogTitle>Edit {selectedProvider.name}</DialogTitle>
              <DialogDescription>
                Update the base URL for {selectedProvider.name}
              </DialogDescription>
            </DialogHeader>
            <div className="py-4 space-y-4">
              <div className="space-y-2">
                <label className="text-sm font-medium">Base URL</label>
                <Input
                  value={editingBaseUrl}
                  onChange={(e) => setEditingBaseUrl(e.target.value)}
                  placeholder="https://api.example.com/v1"
                />
                <p className="text-xs text-muted-foreground">
                  The API endpoint URL for this provider
                </p>
              </div>
            </div>
            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setShowEditDialog(false)}
                disabled={isSaving}
              >
                Cancel
              </Button>
              <Button
                size="sm"
                onClick={handleSaveBaseUrl}
                disabled={isSaving || !editingBaseUrl.trim()}
              >
                {isSaving ? (
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                ) : (
                  "Save"
                )}
              </Button>
            </div>
          </DialogContent>
        </Dialog>
      )}

      {/* Reset Models Confirmation Dialog */}
      <AlertDialog
        open={showResetDialog}
        onOpenChange={(open) => !open && setShowResetDialog(false)}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Reset Models</AlertDialogTitle>
            <AlertDialogDescription>
              This will delete all standard models and re-seed from models.json.
              Custom models will be preserved. Continue?
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmReset}>Reset</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Add Provider Dialog */}
      {!compact && (
        <AddProviderDialog
          open={showAddProviderDialog}
          onOpenChange={setShowAddProviderDialog}
          onAdd={handleAddUserProvider}
        />
      )}

      {/* Delete Provider Confirmation Dialog */}
      <AlertDialog
        open={showDeleteProviderDialog}
        onOpenChange={(open) => {
          if (!open) {
            setShowDeleteProviderDialog(false);
            setProviderToDelete(null);
          }
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Delete Provider</AlertDialogTitle>
            <AlertDialogDescription>
              This will permanently delete the provider and all its models.
              This action cannot be undone.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={confirmDeleteProvider}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
            >
              Delete
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Global Default Model Selector Dialog */}
      {showGlobalDefault && (
        <AIModelQuickAccess
          open={showGlobalModelSelector}
          onOpenChange={setShowGlobalModelSelector}
          mode="global-default"
          filterAvailable={true}
          onSelectGlobalDefault={handleSetGlobalDefaultModel}
        />
      )}
    </div>
  );
}
