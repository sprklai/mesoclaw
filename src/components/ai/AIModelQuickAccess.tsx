import { useEffect, useMemo } from "react";

import type { ProviderWithModels } from "@/lib/models";

import {
  ModelSelectorDialog,
  ModelSelectorContent,
  ModelSelector,
  ModelSelectorInput,
  ModelSelectorList,
  ModelSelectorGroup,
  ModelSelectorItem,
  ModelSelectorEmpty,
  ModelSelectorLogo,
  ModelSelectorName,
} from "@/components/ai-elements/model-selector";
import { Button } from "@/components/ui/button";
import { Brain } from "@/lib/icons";
import { useLLMStore } from "@/stores/llm";

// Provider category definitions
const AI_GATEWAY_IDS = ["vercel-ai-gateway", "openrouter"];
const LOCAL_PROVIDER_IDS = ["ollama"];

interface AIModelQuickAccessProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workspaceId?: string;
  mode?: "workspace" | "global-default";
  filterAvailable?: boolean;
  onSelectGlobalDefault?: (
    providerId: string,
    modelId: string
  ) => Promise<void>;
}

interface ProviderCategory {
  name: string;
  providers: ProviderWithModels[];
}

export function AIModelQuickAccess({
  open,
  onOpenChange,
  workspaceId,
  mode = "workspace",
  filterAvailable = false,
  onSelectGlobalDefault,
}: AIModelQuickAccessProps) {
  const {
    providersWithModels,
    providersWithKeyStatus,
    config,
    saveProviderConfig,
    loadProvidersAndModels,
    loadProvidersWithKeyStatus,
  } = useLLMStore();

  // Load providers on mount if not already loaded
  useEffect(() => {
    if (providersWithModels.length === 0) {
      loadProvidersAndModels();
    }
    if (providersWithKeyStatus.length === 0) {
      loadProvidersWithKeyStatus();
    }
  }, [
    providersWithModels.length,
    providersWithKeyStatus.length,
    loadProvidersAndModels,
    loadProvidersWithKeyStatus,
  ]);

  // Filter and categorize providers for global-default mode
  const categorizedProviders = useMemo((): ProviderCategory[] => {
    let filteredProviders = providersWithModels;

    // Filter to only show available models if requested
    if (filterAvailable) {
      filteredProviders = providersWithModels.filter((provider) => {
        // Check if provider has models
        if (provider.models.length === 0) return false;

        // Get key status
        const keyStatus = providersWithKeyStatus.find(
          (p) => p.id === provider.id
        );

        // Local providers: only show if models are discovered
        if (LOCAL_PROVIDER_IDS.includes(provider.id)) {
          return provider.models.length > 0;
        }

        // Remote providers: only show if has API key
        if (provider.requiresApiKey) {
          return keyStatus?.hasApiKey ?? false;
        }

        return true;
      });
    }

    if (mode === "global-default") {
      // Group by category for global default mode
      const gatewayProviders = filteredProviders.filter((p) =>
        AI_GATEWAY_IDS.includes(p.id)
      );
      const localProviders = filteredProviders.filter((p) =>
        LOCAL_PROVIDER_IDS.includes(p.id)
      );
      const userDefinedProviders = filteredProviders.filter(
        (p) => p.isUserDefined
      );
      const regularProviders = filteredProviders.filter(
        (p) =>
          !AI_GATEWAY_IDS.includes(p.id) &&
          !LOCAL_PROVIDER_IDS.includes(p.id) &&
          !p.isUserDefined
      );

      const categories: ProviderCategory[] = [];
      if (gatewayProviders.length > 0) {
        categories.push({ name: "AI Gateway", providers: gatewayProviders });
      }
      if (regularProviders.length > 0) {
        categories.push({ name: "AI Providers", providers: regularProviders });
      }
      if (userDefinedProviders.length > 0) {
        categories.push({
          name: "User Defined",
          providers: userDefinedProviders,
        });
      }
      if (localProviders.length > 0) {
        categories.push({ name: "Local", providers: localProviders });
      }
      return categories;
    }

    // Default: single category with all providers
    return [{ name: "", providers: filteredProviders }];
  }, [providersWithModels, providersWithKeyStatus, mode, filterAvailable]);

  const handleModelSelect = async (providerId: string, modelId: string) => {
    if (mode === "global-default" && onSelectGlobalDefault) {
      await onSelectGlobalDefault(providerId, modelId);
    } else {
      await saveProviderConfig(providerId, modelId, workspaceId);
    }
    onOpenChange(false);
  };

  const selectedProviderId = config?.providerId ?? "";
  const selectedModelId = config?.modelId ?? "";

  const hasAnyModels = categorizedProviders.some((cat) =>
    cat.providers.some((p) => p.models.length > 0)
  );

  return (
    <ModelSelectorDialog open={open} onOpenChange={onOpenChange}>
      <ModelSelectorContent>
        <ModelSelector>
          <ModelSelectorInput placeholder="Search models..." />
          <ModelSelectorList>
            {hasAnyModels ? (
              categorizedProviders.map((category) =>
                category.providers.map((provider) => (
                  <ModelSelectorGroup
                    key={provider.id}
                    heading={
                      mode === "global-default" && category.name
                        ? `${category.name} / ${provider.name}`
                        : provider.name
                    }
                  >
                    {provider.models.map((model) => {
                      const isSelected =
                        selectedProviderId === provider.id &&
                        selectedModelId === model.modelId;

                      return (
                        <ModelSelectorItem
                          key={model.modelId}
                          value={model.modelId}
                          onSelect={() =>
                            handleModelSelect(provider.id, model.modelId)
                          }
                        >
                          <div className="flex items-center gap-3 flex-1 min-w-0">
                            <ModelSelectorLogo provider={provider.id as any} />
                            <ModelSelectorName className="truncate">
                              {model.displayName}
                            </ModelSelectorName>
                          </div>
                          {isSelected && (
                            <span className="ml-auto text-xs text-muted-foreground shrink-0">
                              ✓
                            </span>
                          )}
                        </ModelSelectorItem>
                      );
                    })}
                  </ModelSelectorGroup>
                ))
              )
            ) : (
              <ModelSelectorEmpty>
                {filterAvailable
                  ? "No configured providers. Add API keys to see available models."
                  : "No models found"}
              </ModelSelectorEmpty>
            )}
          </ModelSelectorList>
        </ModelSelector>
      </ModelSelectorContent>
    </ModelSelectorDialog>
  );
}

export function AIModelQuickAccessTrigger({
  onClick,
  showLabel = false,
}: {
  onClick: () => void;
  showLabel?: boolean;
}) {
  const { providersWithModels, config } = useLLMStore();

  const currentModel = providersWithModels
    .flatMap((p) => p.models)
    .find((m) => m.modelId === config?.modelId);

  const noModelSelected = !currentModel;

  return (
    <Button
      variant="ghost"
      size={showLabel ? "default" : "icon"}
      onClick={onClick}
      title={`AI Model: ${currentModel?.displayName || "None"}`}
      className={showLabel ? "gap-2 px-3" : ""}
    >
      {showLabel && (
        <span className="truncate text-sm">
          {currentModel?.displayName || "No model"}
        </span>
      )}
      {noModelSelected ? (
        <div className="relative">
          <div className="absolute inset-0 animate-ping rounded-full bg-red-500 opacity-75" />
          <Brain className="relative h-4 w-4 shrink-0 text-red-500" />
        </div>
      ) : (
        <Brain className="h-4 w-4 shrink-0" />
      )}
    </Button>
  );
}
