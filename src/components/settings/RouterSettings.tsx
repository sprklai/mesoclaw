/**
 * Router Settings Tab
 *
 * Manages the MesoClaw routing system for intelligent model selection.
 * Allows users to configure routing profiles, task overrides, and model discovery.
 */
import { useEffect, useState } from "react";
import { toast } from "sonner";

import { SettingsSection } from "@/components/settings-section";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { useLLMStore } from "@/stores/llm";
import {
  getProfileDisplayName,
  getTaskDisplayName,
  type DiscoveredModel,
  type RoutingProfile,
  type TaskType,
  useRouterStore,
} from "@/stores/routerStore";

/**
 * Task types available for override
 */
const TASK_TYPES: TaskType[] = ["code", "general", "fast", "creative", "analysis"];

/**
 * Profile options for the select dropdown
 */
const PROFILE_OPTIONS = [
  { value: "eco", label: "Eco (Cost-Effective)" },
  { value: "balanced", label: "Balanced (Recommended)" },
  { value: "premium", label: "Premium (Best Quality)" },
];


/**
 * Get badge variant for cost tier
 */
function getCostTierBadge(tier: string): "default" | "secondary" | "outline" {
  switch (tier) {
    case "low":
      return "secondary";
    case "high":
      return "default";
    default:
      return "outline";
  }
}

/**
 * Format model count display
 */
function formatModelCount(count: number): string {
  return `${count} model${count !== 1 ? "s" : ""}`;
}

export function RouterSettings() {
  const {
    config,
    models,
    isLoading,
    isLoadingModels,
    isDiscovering,
    error,
    initialize,
    setProfile,
    setTaskOverride,
    clearTaskOverride,
    discoverModels,
    clearError,
  } = useRouterStore();

  const { getApiKey, loadApiKeyForProvider, providersWithModels, loadProvidersAndModels } = useLLMStore();

  const [searchQuery, setSearchQuery] = useState("");
  const [providerFilter, setProviderFilter] = useState<string>("all");
  const [discoveringProvider, setDiscoveringProvider] = useState<string | null>(null);

  // Initialize on mount - chain calls to avoid race condition
  // loadProvidersAndModels needs providers loaded first for discoverModels to work
  useEffect(() => {
    initialize().then(() => loadProvidersAndModels());
  }, [initialize, loadProvidersAndModels]);

  // Show error toast if error occurs
  useEffect(() => {
    if (error) {
      toast.error(error);
      clearError();
    }
  }, [error, clearError]);

  // Handle profile change
  const handleProfileChange = async (profile: string) => {
    try {
      await setProfile(profile as RoutingProfile);
      toast.success(`Routing profile set to ${getProfileDisplayName(profile as RoutingProfile)}`);
    } catch (err) {
      toast.error(`Failed to set profile: ${err}`);
    }
  };

  // Handle task override change
  const handleTaskOverride = async (task: TaskType, modelId: string) => {
    try {
      if (modelId === "") {
        await clearTaskOverride(task);
        toast.success(`Cleared override for ${getTaskDisplayName(task)}`);
      } else {
        await setTaskOverride(task, modelId);
        toast.success(`Set ${getTaskDisplayName(task)} to use ${modelId}`);
      }
    } catch (err) {
      toast.error(`Failed to update task override: ${err}`);
    }
  };

  // Handle model discovery
  const handleDiscoverModels = async (providerId: string) => {
    setDiscoveringProvider(providerId);
    try {
      // Look up provider configuration from LLM store
      const provider = providersWithModels.find((p) => p.id === providerId);
      if (!provider) {
        toast.error(`Provider ${providerId} not found in settings. Please configure it in Settings > AI Providers.`);
        return;
      }

      let apiKey: string | undefined;
      if (provider.requiresApiKey) {
        // Load API key from keychain if not already cached
        await loadApiKeyForProvider(providerId);
        try {
          apiKey = await getApiKey(providerId);
        } catch {
          toast.error(`API key not configured for ${providerId}. Please add it in Settings > AI Providers.`);
          return;
        }
      }

      // Use the base URL from the provider configuration
      const count = await discoverModels(providerId, provider.baseUrl, apiKey);
      toast.success(`Discovered ${formatModelCount(count)} from ${providerId}`);
    } catch (err) {
      toast.error(`Failed to discover models from ${providerId}: ${err}`);
    } finally {
      setDiscoveringProvider(null);
    }
  };

  // Filter models based on search and provider filter
  const filteredModels = models.filter((model) => {
    const matchesSearch =
      searchQuery === "" ||
      model.displayName.toLowerCase().includes(searchQuery.toLowerCase()) ||
      model.modelId.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesProvider = providerFilter === "all" || model.providerId === providerFilter;
    return matchesSearch && matchesProvider && model.isActive;
  });

  // Get unique providers from models
  const providers = [...new Set(models.map((m) => m.providerId))].sort();

  // Build model options for task override dropdowns
  const modelOptions = [
    { value: "", label: "Use default routing" },
    ...filteredModels.map((m) => ({
      value: m.id,
      label: `${m.displayName} (${m.providerId})`,
    })),
  ];

  // Get current override for a task
  const getTaskOverride = (task: TaskType): string => {
    return config?.taskOverrides?.[task] ?? "";
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-12">
        <p className="text-muted-foreground">Loading router settings…</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Routing Profile Section */}
      <SettingsSection
        title="Routing Profile"
        description="Select the routing profile that balances quality and cost for your needs"
      >
        <div className="grid gap-4">
          {PROFILE_OPTIONS.map((option) => (
            <label
              key={option.value}
              className={`flex cursor-pointer items-start gap-3 rounded-lg border p-4 transition-colors ${
                config?.activeProfile === option.value
                  ? "border-primary bg-primary/5"
                  : "border-border hover:border-primary/50"
              }`}
            >
              <input
                type="radio"
                name="profile"
                value={option.value}
                checked={config?.activeProfile === option.value}
                onChange={() => handleProfileChange(option.value)}
                className="mt-0.5 size-4 shrink-0"
              />
              <div className="min-w-0 flex-1">
                <div className="font-medium">{option.label}</div>
                <p className="text-sm text-muted-foreground">
                  {option.value === "eco" &&
                    "Uses cost-effective models like gemini-2.0-flash and gpt-4o-mini"}
                  {option.value === "balanced" &&
                    "Balances quality and cost with claude-sonnet-4.5 and gpt-4o"}
                  {option.value === "premium" &&
                    "Uses the most capable models like claude-opus-4.5 and o3"}
                </p>
              </div>
            </label>
          ))}
        </div>
      </SettingsSection>

      {/* Task Routing Overrides Section */}
      <SettingsSection
        title="Task Routing"
        description="Override the default model selection for specific task types"
      >
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Task Type</TableHead>
              <TableHead>Description</TableHead>
              <TableHead className="w-64">Model Override</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {TASK_TYPES.map((task) => (
              <TableRow key={task}>
                <TableCell className="font-medium">{getTaskDisplayName(task)}</TableCell>
                <TableCell className="text-muted-foreground">
                  {task === "code" && "Programming, debugging, code generation"}
                  {task === "general" && "General conversation and questions"}
                  {task === "fast" && "Quick responses for simple queries"}
                  {task === "creative" && "Writing, brainstorming, creative content"}
                  {task === "analysis" && "Analysis, research, explanations"}
                </TableCell>
                <TableCell>
                  <Select
                    value={getTaskOverride(task)}
                    onValueChange={(value) => handleTaskOverride(task, value)}
                    options={modelOptions}
                    disabled={isLoadingModels}
                    placeholder="Use default routing"
                    className="w-full"
                  />
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </SettingsSection>

      {/* Model Discovery Section */}
      <SettingsSection
        title="Model Discovery"
        description="Discover available models from each provider's API"
      >
        <div className="space-y-3">
          {providersWithModels.map((provider) => (
            <div
              key={provider.id}
              className="flex items-center justify-between rounded-lg border border-border bg-card p-3"
            >
              <div className="min-w-0">
                <div className="font-medium">{provider.name ?? provider.id}</div>
                <p className="truncate text-sm text-muted-foreground">{provider.baseUrl}</p>
              </div>
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleDiscoverModels(provider.id)}
                disabled={isDiscovering && discoveringProvider !== provider.id}
              >
                {discoveringProvider === provider.id ? "Discovering..." : "Sync Models"}
              </Button>
            </div>
          ))}
        </div>

        {config?.lastDiscovery && (
          <p className="text-sm text-muted-foreground">
            Last discovery: {new Date(config.lastDiscovery).toLocaleString()}
          </p>
        )}
      </SettingsSection>

      {/* Discovered Models Section */}
      <SettingsSection
        title="Discovered Models"
        description={`${formatModelCount(filteredModels.length)} available`}
      >
        {/* Filters */}
        <div className="flex flex-col gap-3 sm:flex-row">
          <Input
            placeholder="Search models..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="sm:w-64"
          />
          <Select
            value={providerFilter}
            onValueChange={setProviderFilter}
            options={[
              { value: "all", label: "All Providers" },
              ...providers.map((p) => ({ value: p, label: p })),
            ]}
            className="sm:w-40"
          />
        </div>

        {/* Model List */}
        <div className="max-h-96 space-y-2 overflow-y-auto">
          {filteredModels.length === 0 ? (
            <div className="rounded-lg border border-dashed border-border p-6 text-center">
              <p className="text-muted-foreground">
                {models.length === 0
                  ? "No models discovered yet. Use the discovery buttons above to sync models from providers."
                  : "No models match your search criteria."}
              </p>
            </div>
          ) : (
            filteredModels.map((model) => (
              <ModelListItem key={model.id} model={model} />
            ))
          )}
        </div>
      </SettingsSection>
    </div>
  );
}

/**
 * Model list item component
 */
function ModelListItem({ model }: { model: DiscoveredModel }) {
  return (
    <div className="flex items-center justify-between rounded-lg border border-border bg-card p-3">
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate font-medium">{model.displayName}</span>
          <Badge variant={getCostTierBadge(model.costTier)} className="shrink-0">
            {model.costTier}
          </Badge>
        </div>
        <p className="truncate text-sm text-muted-foreground">
          {model.providerId}/{model.modelId}
        </p>
        {model.contextLimit && (
          <p className="text-xs text-muted-foreground">
            {model.contextLimit.toLocaleString()} tokens context
          </p>
        )}
      </div>
      <div className="ml-3 flex shrink-0 gap-1">
        {model.modalities.map((modality) => (
          <ModalityBadge key={modality} modality={modality} />
        ))}
      </div>
    </div>
  );
}

/**
 * Modality badge component
 */
function ModalityBadge({ modality }: { modality: string }) {
  const icons: Record<string, string> = {
    text: "📝",
    image: "📷",
    image_generation: "🎨",
    audio_transcription: "🎤",
    audio_generation: "🔊",
    video: "🎬",
    embedding: "🔢",
  };

  const labels: Record<string, string> = {
    text: "Text",
    image: "Vision",
    image_generation: "Image Gen",
    audio_transcription: "Audio",
    audio_generation: "TTS",
    video: "Video",
    embedding: "Embedding",
  };

  return (
    <span
      className="inline-flex items-center rounded-full bg-muted px-2 py-0.5 text-xs"
      title={labels[modality] ?? modality}
    >
      {icons[modality] ?? "❓"}
    </span>
  );
}
