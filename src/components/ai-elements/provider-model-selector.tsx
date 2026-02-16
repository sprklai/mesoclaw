import { memo, useEffect, useState } from "react";

import type { ProviderWithModels } from "@/lib/models";

import { Button } from "@/components/ui/button";
import { Select, SelectOption } from "@/components/ui/select";
import { Plus as PlusIcon } from "@/lib/icons";

export interface ProviderModelSelectorProps {
  providers: ProviderWithModels[];
  selectedProvider: string;
  selectedModel: string;
  onProviderChange: (providerId: string) => void;
  onModelChange: (modelId: string) => void;
  onAddCustomModel?: () => void;
  disabled?: boolean;
}

export const ProviderModelSelector = memo(function ProviderModelSelector({
  providers,
  selectedProvider,
  selectedModel,
  onProviderChange,
  onModelChange,
  onAddCustomModel,
  disabled = false,
}: ProviderModelSelectorProps) {
  const [localProvider, setLocalProvider] = useState(selectedProvider);

  // Get models for the currently selected provider
  const selectedProviderData = providers.find((p) => p.id === localProvider);
  const availableModels = selectedProviderData?.models || [];

  // Convert models to SelectOptions
  const modelOptions: SelectOption[] = availableModels.map((model) => ({
    value: model.modelId,
    label: model.displayName,
  }));

  // When provider changes, update local state and reset model
  const handleProviderChange = (providerId: string) => {
    setLocalProvider(providerId);
    onProviderChange(providerId);
    // Clear model selection when provider changes
    onModelChange("");
  };

  // Sync local state when provider changes externally
  useEffect(() => {
    setLocalProvider(selectedProvider);
  }, [selectedProvider]);

  // Provider options
  const providerOptions: SelectOption[] = providers.map((provider) => ({
    value: provider.id,
    label: `${provider.name} (${provider.models.length} models)`,
  }));

  return (
    <div className="flex flex-col gap-3">
      {/* Provider Selection */}
      <div className="flex flex-col gap-1.5">
        <label className="text-sm font-medium text-foreground">Provider</label>
        <Select
          value={localProvider}
          onValueChange={handleProviderChange}
          options={providerOptions}
          placeholder="Select a provider"
          disabled={disabled}
        />
      </div>

      {/* Model Selection */}
      <div className="flex flex-col gap-1.5">
        <div className="flex items-center justify-between">
          <label className="text-sm font-medium text-foreground">Model</label>
          {onAddCustomModel && selectedProvider && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="h-6 px-2 text-xs"
              onClick={onAddCustomModel}
              disabled={disabled}
            >
              <PlusIcon className="h-3 w-3 mr-1" />
              Add Custom Model
            </Button>
          )}
        </div>
        <Select
          value={selectedModel}
          onValueChange={onModelChange}
          options={modelOptions}
          placeholder="Select a model"
          disabled={disabled || !selectedProvider}
        />
      </div>
    </div>
  );
});
