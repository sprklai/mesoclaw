import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

import type { ProviderWithModels } from "@/lib/models";

import { SettingRow } from "@/components/setting-row";
import { SettingsSection } from "@/components/settings-section";
import { AddCustomModelDialog } from "@/components/settings/AddCustomModelDialog";
import { AISettings } from "@/components/settings/AISettings";
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
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  ChevronRight,
  Globe,
  Key,
  Loader2,
  Plus as PlusIcon,
  Trash2,
} from "@/lib/icons";
import { showError, showSuccess } from "@/lib/toast";
import { APP_IDENTITY } from "@/config/app-identity";
import { useLLMStore } from "@/stores/llm";

interface DeleteModelDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  modelId: string;
  displayName: string;
  isCustom: boolean;
  onConfirm: () => void;
}

function DeleteModelDialog({
  open,
  onOpenChange,
  modelId: _modelId,
  displayName,
  isCustom,
  onConfirm,
}: DeleteModelDialogProps) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Delete Model</AlertDialogTitle>
          <AlertDialogDescription>
            {isCustom
              ? `Are you sure you want to delete "${displayName}"? This action cannot be undone.`
              : "Built-in models cannot be deleted."}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={!isCustom}>Cancel</AlertDialogCancel>
          <AlertDialogAction
            onClick={onConfirm}
            disabled={!isCustom}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

interface ModelListDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  provider: ProviderWithModels | null;
}

function ModelListDialog({
  open,
  onOpenChange,
  provider,
}: ModelListDialogProps) {
  const [deleteModelId, setDeleteModelId] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  if (!provider) return null;

  const handleDelete = async (
    modelId: string,
    _displayName: string,
    isCustom: boolean
  ) => {
    if (!isCustom) {
      setDeleteModelId(modelId);
      return;
    }

    setIsDeleting(true);
    try {
      await useLLMStore.getState().deleteModel(modelId);
      // Reload providers
      await useLLMStore.getState().loadProvidersAndModels();
    } finally {
      setIsDeleting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Available Models</DialogTitle>
          <DialogDescription>
            {provider.name} has {provider.models.length} models configured
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-2 py-4">
          {provider.models.map((model) => (
            <div
              key={model.id}
              className="flex items-center justify-between rounded-lg border p-3"
            >
              <div className="flex flex-col gap-1">
                <span className="font-medium">{model.displayName}</span>
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <span className="font-mono">{model.modelId}</span>
                  {model.contextLimit && (
                    <>
                      <span>•</span>
                      <span>
                        {(model.contextLimit / 1000).toFixed(0)}K context
                      </span>
                    </>
                  )}
                  {model.isCustom && (
                    <>
                      <span>•</span>
                      <span className="text-orange-600">Custom</span>
                    </>
                  )}
                </div>
              </div>

              {model.isCustom && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() =>
                    handleDelete(model.id, model.displayName, model.isCustom)
                  }
                  disabled={isDeleting}
                >
                  {isDeleting ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Trash2 className="h-4 w-4 text-destructive" />
                  )}
                </Button>
              )}
            </div>
          ))}
        </div>

        <DeleteModelDialog
          open={deleteModelId !== null}
          onOpenChange={(open) => !open && setDeleteModelId(null)}
          modelId={deleteModelId || ""}
          displayName={
            provider.models.find((m) => m.id === deleteModelId)?.displayName ||
            ""
          }
          isCustom={
            provider.models.find((m) => m.id === deleteModelId)?.isCustom ||
            false
          }
          onConfirm={() => {
            if (deleteModelId) {
              handleDelete(
                deleteModelId,
                provider.models.find((m) => m.id === deleteModelId)
                  ?.displayName || "",
                true
              );
              setDeleteModelId(null);
              onOpenChange(false);
            }
          }}
        />
      </DialogContent>
    </Dialog>
  );
}

export function AIProviderManagement() {
  const { config, providersWithModels, providersWithKeyStatus } = useLLMStore();

  // State for UI
  const [selectedProviderId, setSelectedProviderId] = useState<string | null>(
    null
  );
  const [selectedProvider, setSelectedProvider] =
    useState<ProviderWithModels | null>(null);
  const [showAddModel, setShowAddModel] = useState(false);
  const [showModels, setShowModels] = useState(false);

  // Load providers on mount
  useEffect(() => {
    useLLMStore.getState().loadProvidersAndModels();
    useLLMStore.getState().loadProvidersWithKeyStatus();
  }, []);

  // Update selected provider when provider ID changes
  useEffect(() => {
    if (selectedProviderId) {
      const provider = providersWithModels.find(
        (p) => p.id === selectedProviderId
      );
      setSelectedProvider(provider || null);
    }
  }, [selectedProviderId, providersWithModels]);

  const handleAddCustomModel = async (modelId: string, displayName: string) => {
    if (!selectedProvider) return;

    await useLLMStore
      .getState()
      .addCustomModel(selectedProvider.id, modelId, displayName);
    await useLLMStore.getState().loadProvidersAndModels();
  };

  const handleDeleteApiKey = async (providerId: string) => {
    try {
      await invoke("keychain_delete", {
        service: APP_IDENTITY.keychainService,
        key: `api_key:${providerId}`,
      });
      await useLLMStore.getState().loadProvidersWithKeyStatus();
    } catch (error) {
      console.error(`Failed to delete API key for ${providerId}:`, error);
    }
  };

  const handleProviderClick = (providerId: string) => {
    setSelectedProviderId(providerId);
  };

  return (
    <div className="space-y-6">
      {/* Provider List with Key Status */}
      <SettingsSection
        title="AI Providers"
        description="Manage your AI provider configurations"
      >
        <div className="flex flex-col gap-2">
          {providersWithKeyStatus.map((provider) => (
            <div
              key={provider.id}
              className="flex items-center justify-between rounded-lg border p-4 hover:bg-accent/50 transition-colors cursor-pointer"
              onClick={() => handleProviderClick(provider.id)}
            >
              <div className="flex flex-col gap-1">
                <div className="flex items-center gap-2">
                  <Globe className="h-4 w-4 text-muted-foreground" />
                  <span className="font-medium">{provider.name}</span>
                </div>
                <div className="flex items-center gap-3 text-xs text-muted-foreground">
                  <span>{provider.baseUrl}</span>
                  <span>•</span>
                  <span>
                    {provider.requiresApiKey
                      ? "Requires API key"
                      : "Local provider"}
                  </span>
                  <span>•</span>
                  <span>{provider.models.length} models</span>
                </div>
              </div>

              <div className="flex items-center gap-3">
                {provider.hasApiKey ? (
                  <div
                    className="flex items-center gap-1 text-xs text-green-600"
                    onClick={(e) => {
                      e.stopPropagation();
                    }}
                  >
                    <Key className="h-3 w-3" />
                    <span>Configured</span>
                  </div>
                ) : provider.requiresApiKey ? (
                  <div
                    className="flex items-center gap-1 text-xs text-orange-600"
                    onClick={(e) => {
                      e.stopPropagation();
                    }}
                  >
                    <Key className="h-3 w-3" />
                    <span>Not configured</span>
                  </div>
                ) : (
                  <div
                    className="flex items-center gap-1 text-xs text-blue-600"
                    onClick={(e) => {
                      e.stopPropagation();
                    }}
                  >
                    <Globe className="h-3 w-3" />
                    <span>Local</span>
                  </div>
                )}

                <ChevronRight className="h-4 w-4 text-muted-foreground" />
              </div>
            </div>
          ))}
        </div>
      </SettingsSection>

      {/* Selected Provider Configuration */}
      {selectedProvider && config && (
        <AISettings
          providers={providersWithModels}
          selectedProvider={config.providerId}
          selectedModel={config.modelId}
          apiKey={""} // API key is now managed separately via keychain
          isLoadingApiKey={false}
          onProviderChange={(providerId) =>
            useLLMStore
              .getState()
              .saveProviderConfig(providerId, config.modelId)
          }
          onModelChange={(modelId) =>
            useLLMStore
              .getState()
              .saveProviderConfig(config.providerId, modelId)
          }
          onApiKeyChange={async (apiKey) => {
            try {
              await useLLMStore
                .getState()
                .saveApiKeyForProvider(config.providerId, apiKey);
              await useLLMStore.getState().loadProvidersWithKeyStatus();
              showSuccess("API key saved successfully");
            } catch (error) {
              console.error("Failed to save API key:", error);
              showError("Failed to save API key");
            }
          }}
          onAddCustomModel={() => setShowAddModel(true)}
        />
      )}

      {/* View Models Button */}
      {selectedProvider && (
        <SettingRow
          label="Manage Models"
          description={`View all ${selectedProvider.models.length} models for ${selectedProvider.name}`}
        >
          <Button variant="outline" onClick={() => setShowModels(true)}>
            <PlusIcon className="mr-2 h-4 w-4" />
            View Models
          </Button>
        </SettingRow>
      )}

      {/* Delete API Key (if configured) */}
      {selectedProvider && selectedProvider.requiresApiKey && (
        <SettingRow
          label="Remove API Key"
          description={`Delete the stored API key for ${selectedProvider.name}`}
        >
          <Button
            variant="destructive"
            onClick={() => handleDeleteApiKey(selectedProvider.id)}
          >
            <Trash2 className="mr-2 h-4 w-4" />
            Remove API Key
          </Button>
        </SettingRow>
      )}

      {/* Add Custom Model Dialog */}
      {selectedProvider && (
        <AddCustomModelDialog
          open={showAddModel}
          onOpenChange={setShowAddModel}
          providerId={selectedProvider.id}
          providerName={selectedProvider.name}
          onAdd={handleAddCustomModel}
        />
      )}

      {/* Model List Dialog */}
      <ModelListDialog
        open={showModels}
        onOpenChange={setShowModels}
        provider={selectedProvider}
      />
    </div>
  );
}
