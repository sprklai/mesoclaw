import { Plus, Trash2 } from "lucide-react";
import { useState } from "react";

import type { InitialModelSpec } from "@/lib/models";

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
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";

interface AddProviderDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdd: (
    id: string,
    name: string,
    baseUrl: string,
    requiresApiKey: boolean,
    initialModels: InitialModelSpec[],
    apiKey?: string
  ) => Promise<void>;
}

interface ModelRow {
  id: string;
  modelId: string;
  displayName: string;
}

export function AddProviderDialog({
  open,
  onOpenChange,
  onAdd,
}: AddProviderDialogProps) {
  const [providerId, setProviderId] = useState("");
  const [providerName, setProviderName] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [requiresApiKey, setRequiresApiKey] = useState(true);
  const [apiKey, setApiKey] = useState("");
  const [models, setModels] = useState<ModelRow[]>([
    { id: crypto.randomUUID(), modelId: "", displayName: "" },
  ]);
  const [isLoading, setIsLoading] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  const validateProviderId = (id: string): string | null => {
    if (id.length < 3 || id.length > 50) {
      return "Provider ID must be 3-50 characters";
    }
    if (!/^[a-z0-9-]+$/.test(id)) {
      return "Provider ID must be lowercase alphanumeric with hyphens only";
    }
    return null;
  };

  const validateBaseUrl = (url: string): string | null => {
    try {
      const parsed = new URL(url);
      if (!["http:", "https:"].includes(parsed.protocol)) {
        return "Base URL must use HTTP or HTTPS";
      }
      return null;
    } catch {
      return "Invalid URL format";
    }
  };

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    const providerIdError = validateProviderId(providerId);
    if (providerIdError) {
      newErrors.providerId = providerIdError;
    }

    if (!providerName.trim()) {
      newErrors.providerName = "Provider name is required";
    }

    const baseUrlError = validateBaseUrl(baseUrl);
    if (baseUrlError) {
      newErrors.baseUrl = baseUrlError;
    }

    // At least one model with a model ID is required
    const validModels = models.filter((m) => m.modelId.trim());
    if (validModels.length === 0) {
      newErrors.models = "At least one model is required";
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) {
      return;
    }

    setIsLoading(true);
    try {
      const initialModels: InitialModelSpec[] = models
        .filter((m) => m.modelId.trim())
        .map((m) => ({
          modelId: m.modelId.trim(),
          displayName: m.displayName.trim() || undefined,
        }));

      await onAdd(
        providerId.trim(),
        providerName.trim(),
        baseUrl.trim(),
        requiresApiKey,
        initialModels,
        requiresApiKey && apiKey.trim() ? apiKey.trim() : undefined
      );

      // Reset form on success
      resetForm();
      onOpenChange(false);
    } finally {
      setIsLoading(false);
    }
  };

  const resetForm = () => {
    setProviderId("");
    setProviderName("");
    setBaseUrl("");
    setRequiresApiKey(true);
    setApiKey("");
    setModels([{ id: crypto.randomUUID(), modelId: "", displayName: "" }]);
    setErrors({});
  };

  const handleCancel = () => {
    resetForm();
    onOpenChange(false);
  };

  const addModelRow = () => {
    setModels([
      ...models,
      { id: crypto.randomUUID(), modelId: "", displayName: "" },
    ]);
  };

  const removeModelRow = (id: string) => {
    if (models.length > 1) {
      setModels(models.filter((m) => m.id !== id));
    }
  };

  const updateModelRow = (
    id: string,
    field: "modelId" | "displayName",
    value: string
  ) => {
    setModels(models.map((m) => (m.id === id ? { ...m, [field]: value } : m)));
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>Add Custom Provider</DialogTitle>
          <DialogDescription>
            Add a user-defined AI provider with your own API endpoint and
            models.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit}>
          <div className="flex flex-col gap-4 py-4">
            {/* Provider ID */}
            <div className="flex flex-col gap-2">
              <Label htmlFor="provider-id">Provider ID</Label>
              <Input
                id="provider-id"
                placeholder="e.g., my-custom-api"
                value={providerId}
                onChange={(e) => setProviderId(e.target.value.toLowerCase())}
                disabled={isLoading}
                required
              />
              {errors.providerId && (
                <p className="text-xs text-destructive">{errors.providerId}</p>
              )}
              <p className="text-xs text-muted-foreground">
                Lowercase alphanumeric with hyphens, 3-50 characters.
              </p>
            </div>

            {/* Provider Name */}
            <div className="flex flex-col gap-2">
              <Label htmlFor="provider-name">Provider Name</Label>
              <Input
                id="provider-name"
                placeholder="e.g., My Custom API"
                value={providerName}
                onChange={(e) => setProviderName(e.target.value)}
                disabled={isLoading}
                required
              />
              {errors.providerName && (
                <p className="text-xs text-destructive">
                  {errors.providerName}
                </p>
              )}
              <p className="text-xs text-muted-foreground">
                Display name for the provider in the UI.
              </p>
            </div>

            {/* Base URL */}
            <div className="flex flex-col gap-2">
              <Label htmlFor="base-url">Base URL</Label>
              <Input
                id="base-url"
                placeholder="e.g., https://api.example.com/v1"
                value={baseUrl}
                onChange={(e) => setBaseUrl(e.target.value)}
                disabled={isLoading}
                required
              />
              {errors.baseUrl && (
                <p className="text-xs text-destructive">{errors.baseUrl}</p>
              )}
              <p className="text-xs text-muted-foreground">
                The base URL for API requests (OpenAI-compatible endpoint).
              </p>
            </div>

            {/* Requires API Key */}
            <div className="flex items-center justify-between gap-4">
              <div className="flex flex-col gap-1">
                <Label htmlFor="requires-api-key">Requires API Key</Label>
                <p className="text-xs text-muted-foreground">
                  Enable if the provider requires authentication.
                </p>
              </div>
              <Switch
                id="requires-api-key"
                checked={requiresApiKey}
                onCheckedChange={setRequiresApiKey}
                disabled={isLoading}
              />
            </div>

            {/* API Key (conditional) */}
            {requiresApiKey && (
              <div className="flex flex-col gap-2">
                <Label htmlFor="api-key">API Key (optional)</Label>
                <Input
                  id="api-key"
                  type="password"
                  placeholder="Enter API key..."
                  value={apiKey}
                  onChange={(e) => setApiKey(e.target.value)}
                  disabled={isLoading}
                />
                <p className="text-xs text-muted-foreground">
                  You can add the API key now or configure it later in settings.
                </p>
              </div>
            )}

            {/* Initial Models */}
            <div className="flex flex-col gap-2">
              <div className="flex items-center justify-between">
                <Label>Initial Models</Label>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={addModelRow}
                  disabled={isLoading}
                >
                  <Plus className="h-4 w-4 mr-1" />
                  Add Model
                </Button>
              </div>
              {errors.models && (
                <p className="text-xs text-destructive">{errors.models}</p>
              )}
              <div className="space-y-3">
                {models.map((model) => (
                  <div key={model.id} className="flex gap-2 items-start">
                    <div className="flex-1 space-y-2">
                      <Input
                        placeholder="Model ID (required)"
                        value={model.modelId}
                        onChange={(e) =>
                          updateModelRow(model.id, "modelId", e.target.value)
                        }
                        disabled={isLoading}
                      />
                      <Input
                        placeholder="Display name (optional)"
                        value={model.displayName}
                        onChange={(e) =>
                          updateModelRow(
                            model.id,
                            "displayName",
                            e.target.value
                          )
                        }
                        disabled={isLoading}
                      />
                    </div>
                    {models.length > 1 && (
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        onClick={() => removeModelRow(model.id)}
                        disabled={isLoading}
                        className="mt-1"
                      >
                        <Trash2 className="h-4 w-4 text-muted-foreground" />
                      </Button>
                    )}
                  </div>
                ))}
              </div>
              <p className="text-xs text-muted-foreground">
                Add at least one model. You can add more models later.
              </p>
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={handleCancel}
              disabled={isLoading}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isLoading}>
              {isLoading ? "Adding..." : "Add Provider"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
