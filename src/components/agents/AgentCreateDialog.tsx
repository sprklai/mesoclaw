/**
 * AgentCreateDialog - Dialog for creating and editing agent configurations.
 *
 * Features:
 * - Form fields for name, role, system prompt, model selection
 * - Temperature and max tokens sliders
 * - Provider and model dropdowns
 * - Form validation with error display
 * - Supports both create and edit modes
 */
import { Loader2, Plus } from "@/lib/icons";
import type { AgentConfig, CreateAgentRequest, UpdateAgentRequest } from "@/lib/agent-config";
import { DEFAULT_AGENT_CONFIG } from "@/lib/agent-config";
import type { ProviderWithModels } from "@/lib/models";

import { Badge } from "@/components/ui/badge";
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
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

import { useState } from "react";

// ─── Types ────────────────────────────────────────────────────────────────

interface AgentCreateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (request: CreateAgentRequest | UpdateAgentRequest) => Promise<void>;
  agent?: AgentConfig | null;
  providers: ProviderWithModels[];
  isLoading?: boolean;
}

interface FormErrors {
  name?: string;
  role?: string;
  systemPrompt?: string;
  providerId?: string;
  modelId?: string;
  temperature?: string;
  maxTokens?: string;
}

// ─── Predefined Roles ──────────────────────────────────────────────────────

const PREDEFINED_ROLES = [
  { value: "developer", label: "Developer" },
  { value: "architect", label: "Architect" },
  { value: "designer", label: "Designer" },
  { value: "analyst", label: "Analyst" },
  { value: "researcher", label: "Researcher" },
  { value: "tester", label: "QA Tester" },
  { value: "writer", label: "Writer" },
  { value: "custom", label: "Custom" },
];

// ─── Component ────────────────────────────────────────────────────────────

export function AgentCreateDialog({
  open,
  onOpenChange,
  onSubmit,
  agent,
  providers,
  isLoading = false,
}: AgentCreateDialogProps) {
  const isEditing = !!agent;

  // Form state
  const [name, setName] = useState(agent?.name ?? "");
  const [role, setRole] = useState(agent?.role ?? "developer");
  const [customRole, setCustomRole] = useState("");
  const [systemPrompt, setSystemPrompt] = useState(agent?.systemPrompt ?? "");
  const [providerId, setProviderId] = useState(agent?.providerId ?? "");
  const [modelId, setModelId] = useState(agent?.modelId ?? "");
  const [temperature, setTemperature] = useState(agent?.temperature ?? DEFAULT_AGENT_CONFIG.temperature ?? 0.7);
  const [maxTokens, setMaxTokens] = useState(agent?.maxTokens ?? DEFAULT_AGENT_CONFIG.maxTokens ?? 4096);
  const [maxIterations, setMaxIterations] = useState(agent?.maxIterations ?? DEFAULT_AGENT_CONFIG.maxIterations ?? 20);
  const [isEnabled, setIsEnabled] = useState(agent?.isEnabled ?? true);

  const [errors, setErrors] = useState<FormErrors>({});

  // Get models for selected provider
  const selectedProvider = providers.find((p) => p.id === providerId);
  const availableModels = selectedProvider?.models ?? [];

  // Build options for Select components
  const providerOptions = providers.map((p) => ({ value: p.id, label: p.name }));
  const modelOptions = availableModels.map((m) => ({ value: m.modelId, label: m.displayName }));

  // ─── Validation ──────────────────────────────────────────────────────────

  const validate = (): boolean => {
    const newErrors: FormErrors = {};

    if (!name.trim()) {
      newErrors.name = "Name is required";
    } else if (name.length < 2) {
      newErrors.name = "Name must be at least 2 characters";
    }

    if (!role.trim()) {
      newErrors.role = "Role is required";
    }

    if (!systemPrompt.trim()) {
      newErrors.systemPrompt = "System prompt is required";
    } else if (systemPrompt.length < 10) {
      newErrors.systemPrompt = "System prompt must be at least 10 characters";
    }

    if (!providerId) {
      newErrors.providerId = "Provider is required";
    }

    if (!modelId) {
      newErrors.modelId = "Model is required";
    }

    if (temperature < 0 || temperature > 2) {
      newErrors.temperature = "Temperature must be between 0 and 2";
    }

    if (maxTokens < 1 || maxTokens > 128000) {
      newErrors.maxTokens = "Max tokens must be between 1 and 128000";
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  // ─── Handlers ────────────────────────────────────────────────────────────

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) {
      return;
    }

    const actualRole = role === "custom" ? customRole : role;

    if (isEditing && agent) {
      await onSubmit({
        id: agent.id,
        name,
        role: actualRole,
        systemPrompt,
        providerId,
        modelId,
        temperature,
        maxTokens,
        maxIterations,
        isEnabled,
      } as UpdateAgentRequest);
    } else {
      await onSubmit({
        name,
        role: actualRole,
        systemPrompt,
        providerId,
        modelId,
        temperature,
        maxTokens,
        maxIterations,
      } as CreateAgentRequest);
    }

    // Reset form on success
    resetForm();
    onOpenChange(false);
  };

  const resetForm = () => {
    if (!isEditing) {
      setName("");
      setRole("developer");
      setCustomRole("");
      setSystemPrompt("");
      setProviderId("");
      setModelId("");
      setTemperature(DEFAULT_AGENT_CONFIG.temperature ?? 0.7);
      setMaxTokens(DEFAULT_AGENT_CONFIG.maxTokens ?? 4096);
      setMaxIterations(DEFAULT_AGENT_CONFIG.maxIterations ?? 20);
      setIsEnabled(true);
    }
    setErrors({});
  };

  const handleCancel = () => {
    resetForm();
    onOpenChange(false);
  };

  const handleProviderChange = (value: string) => {
    setProviderId(value);
    // Reset model when provider changes
    setModelId("");
  };

  // ─── Render ──────────────────────────────────────────────────────────────

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{isEditing ? "Edit Agent" : "Create New Agent"}</DialogTitle>
          <DialogDescription>
            {isEditing
              ? "Update the agent configuration and settings."
              : "Configure a new agent with its role, system prompt, and model settings."}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit}>
          <div className="flex flex-col gap-6 py-4">
            {/* Name */}
            <div className="flex flex-col gap-2">
              <Label htmlFor="name">Agent Name</Label>
              <Input
                id="name"
                placeholder="e.g., Frontend Developer"
                value={name}
                onChange={(e) => setName(e.target.value)}
                disabled={isLoading}
                className={errors.name ? "border-destructive" : ""}
              />
              {errors.name && (
                <p className="text-xs text-destructive">{errors.name}</p>
              )}
            </div>

            {/* Role */}
            <div className="flex flex-col gap-2">
              <Label htmlFor="role">Role</Label>
              <Select
                value={role}
                onValueChange={setRole}
                options={PREDEFINED_ROLES}
                disabled={isLoading}
                className={errors.role ? "border-destructive" : ""}
              />
              {role === "custom" && (
                <Input
                  placeholder="Enter custom role"
                  value={customRole}
                  onChange={(e) => setCustomRole(e.target.value)}
                  disabled={isLoading}
                  className="mt-2"
                />
              )}
              {errors.role && (
                <p className="text-xs text-destructive">{errors.role}</p>
              )}
            </div>

            {/* System Prompt */}
            <div className="flex flex-col gap-2">
              <Label htmlFor="systemPrompt">System Prompt</Label>
              <Textarea
                id="systemPrompt"
                placeholder="You are a helpful assistant that..."
                value={systemPrompt}
                onChange={(e) => setSystemPrompt(e.target.value)}
                disabled={isLoading}
                className={cn(
                  "min-h-[120px]",
                  errors.systemPrompt && "border-destructive"
                )}
              />
              {errors.systemPrompt && (
                <p className="text-xs text-destructive">{errors.systemPrompt}</p>
              )}
              <p className="text-xs text-muted-foreground">
                Define the agent's behavior, skills, and constraints.
              </p>
            </div>

            {/* Provider and Model */}
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div className="flex flex-col gap-2">
                <Label htmlFor="provider">Provider</Label>
                <Select
                  value={providerId}
                  onValueChange={handleProviderChange}
                  options={providerOptions}
                  disabled={isLoading}
                  placeholder="Select provider"
                  className={errors.providerId ? "border-destructive" : ""}
                />
                {errors.providerId && (
                  <p className="text-xs text-destructive">{errors.providerId}</p>
                )}
              </div>

              <div className="flex flex-col gap-2">
                <Label htmlFor="model">Model</Label>
                <Select
                  value={modelId}
                  onValueChange={setModelId}
                  options={modelOptions}
                  disabled={isLoading || !providerId}
                  placeholder="Select model"
                  className={errors.modelId ? "border-destructive" : ""}
                />
                {errors.modelId && (
                  <p className="text-xs text-destructive">{errors.modelId}</p>
                )}
              </div>
            </div>

            {/* Temperature */}
            <div className="flex flex-col gap-2">
              <div className="flex items-center justify-between">
                <Label htmlFor="temperature">Temperature: {temperature.toFixed(1)}</Label>
                <Badge variant="secondary" className="text-xs">
                  {temperature < 0.3 ? "Focused" : temperature < 0.7 ? "Balanced" : "Creative"}
                </Badge>
              </div>
              <Input
                id="temperature"
                type="range"
                min="0"
                max="2"
                step="0.1"
                value={temperature}
                onChange={(e) => setTemperature(parseFloat(e.target.value))}
                disabled={isLoading}
                className="w-full"
              />
              {errors.temperature && (
                <p className="text-xs text-destructive">{errors.temperature}</p>
              )}
              <p className="text-xs text-muted-foreground">
                Lower values are more focused; higher values are more creative.
              </p>
            </div>

            {/* Max Tokens */}
            <div className="flex flex-col gap-2">
              <Label htmlFor="maxTokens">Max Tokens</Label>
              <Input
                id="maxTokens"
                type="number"
                min={1}
                max={128000}
                value={maxTokens}
                onChange={(e) => setMaxTokens(parseInt(e.target.value, 10))}
                disabled={isLoading}
                className={errors.maxTokens ? "border-destructive" : ""}
              />
              {errors.maxTokens && (
                <p className="text-xs text-destructive">{errors.maxTokens}</p>
              )}
              <p className="text-xs text-muted-foreground">
                Maximum number of tokens per response (1 - 128,000).
              </p>
            </div>

            {/* Max Iterations */}
            <div className="flex flex-col gap-2">
              <Label htmlFor="maxIterations">Max Iterations</Label>
              <Input
                id="maxIterations"
                type="number"
                min={1}
                max={100}
                value={maxIterations}
                onChange={(e) => setMaxIterations(parseInt(e.target.value, 10))}
                disabled={isLoading}
              />
              <p className="text-xs text-muted-foreground">
                Maximum number of tool-call iterations before stopping.
              </p>
            </div>

            {/* Enabled (Edit mode only) */}
            {isEditing && (
              <div className="flex items-center justify-between gap-4">
                <div className="flex flex-col gap-1">
                  <Label htmlFor="isEnabled">Enabled</Label>
                  <p className="text-xs text-muted-foreground">
                    Disabled agents won't participate in workflows.
                  </p>
                </div>
                <Switch
                  id="isEnabled"
                  checked={isEnabled}
                  onCheckedChange={setIsEnabled}
                  disabled={isLoading}
                />
              </div>
            )}
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
              {isLoading ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {isEditing ? "Updating..." : "Creating..."}
                </>
              ) : isEditing ? (
                "Update Agent"
              ) : (
                <>
                  <Plus className="mr-2 h-4 w-4" />
                  Create Agent
                </>
              )}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
