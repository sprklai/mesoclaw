import { createFileRoute, useNavigate } from "@tanstack/react-router";
import {
  Bot,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Key,
  MessageSquare,
  SkipForward,
  Zap,
} from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { APP_IDENTITY } from "@/config/app-identity";
import { CHANNEL_REGISTRY } from "@/lib/channel-registry";
import { cn } from "@/lib/utils";
import { useChannelStore } from "@/stores/channelStore";
import { useAppSettingsStore } from "@/stores/appSettingsStore";
import { useLLMStore } from "@/stores/llm";
import type { TelegramChannelConfig } from "@/stores/channelStore";

export const Route = createFileRoute("/onboarding")({
  component: OnboardingPage,
});

type Step = "welcome" | "ai-provider" | "channels" | "done";

const STEPS: Step[] = ["welcome", "ai-provider", "channels", "done"];

const PROGRESS_STEPS: Array<{ id: Step; label: string }> = [
  { id: "welcome", label: "Welcome" },
  { id: "ai-provider", label: "AI Provider" },
  { id: "channels", label: "Channels" },
];

function OnboardingPage() {
  const [currentStep, setCurrentStep] = useState<Step>("welcome");
  const completeOnboarding = useAppSettingsStore((s) => s.completeOnboarding);
  const navigate = useNavigate();

  function goNext() {
    const idx = STEPS.indexOf(currentStep);
    if (idx < STEPS.length - 1) {
      setCurrentStep(STEPS[idx + 1]);
    }
  }

  function goBack() {
    const idx = STEPS.indexOf(currentStep);
    if (idx > 0) {
      setCurrentStep(STEPS[idx - 1]);
    }
  }

  function handleComplete() {
    completeOnboarding();
    navigate({ to: "/" });
  }

  return (
    <div className="flex min-h-screen flex-col bg-background">
      {currentStep !== "done" && (
        <ProgressHeader currentStep={currentStep} />
      )}

      <div className="flex flex-1 items-center justify-center p-6">
        {currentStep === "welcome" && (
          <WelcomeStep onNext={goNext} />
        )}
        {currentStep === "ai-provider" && (
          <AIProviderStep onNext={goNext} onBack={goBack} />
        )}
        {currentStep === "channels" && (
          <ChannelsStep onNext={goNext} onBack={goBack} />
        )}
        {currentStep === "done" && (
          <DoneStep onComplete={handleComplete} />
        )}
      </div>
    </div>
  );
}

// ─── Progress Header ───────────────────────────────────────────────────────────

interface ProgressHeaderProps {
  currentStep: Step;
}

function ProgressHeader({ currentStep }: ProgressHeaderProps) {
  const currentIndex = PROGRESS_STEPS.findIndex((s) => s.id === currentStep);

  return (
    <header className="border-b border-border px-6 py-4">
      <div className="mx-auto flex max-w-lg items-center justify-between">
        {PROGRESS_STEPS.map((step, idx) => {
          const isCompleted = idx < currentIndex;
          const isCurrent = idx === currentIndex;

          return (
            <div key={step.id} className="flex items-center gap-2">
              <div
                className={cn(
                  "flex h-8 w-8 items-center justify-center rounded-full text-sm font-semibold transition-colors",
                  isCompleted &&
                    "bg-primary text-primary-foreground",
                  isCurrent &&
                    "border-2 border-primary bg-background text-primary",
                  !isCompleted &&
                    !isCurrent &&
                    "border-2 border-muted bg-background text-muted-foreground"
                )}
              >
                {isCompleted ? (
                  <CheckCircle2 className="h-4 w-4" aria-hidden="true" />
                ) : (
                  idx + 1
                )}
              </div>
              <span
                className={cn(
                  "hidden text-sm font-medium sm:block",
                  isCurrent ? "text-foreground" : "text-muted-foreground"
                )}
              >
                {step.label}
              </span>
              {idx < PROGRESS_STEPS.length - 1 && (
                <div className="mx-3 h-px w-8 bg-border sm:w-16" />
              )}
            </div>
          );
        })}
      </div>
    </header>
  );
}

// ─── Welcome Step ──────────────────────────────────────────────────────────────

interface WelcomeStepProps {
  onNext: () => void;
}

function WelcomeStep({ onNext }: WelcomeStepProps) {
  return (
    <div className="flex w-full max-w-md flex-col items-center gap-8 text-center">
      <div className="flex h-20 w-20 items-center justify-center rounded-2xl bg-primary/10">
        <Bot className="h-10 w-10 text-primary" aria-hidden="true" />
      </div>

      <div className="space-y-2">
        <h1 className="text-3xl font-bold tracking-tight">
          Welcome to {APP_IDENTITY.productName}
        </h1>
        <p className="text-muted-foreground">
          Your AI-powered desktop assistant. Let's get you set up in just a few
          steps.
        </p>
      </div>

      <div className="w-full rounded-lg border border-border bg-card p-5 text-left">
        <p className="mb-3 text-sm font-medium text-foreground">
          Here's what we'll set up:
        </p>
        <ul className="space-y-3">
          <li className="flex items-start gap-3">
            <Zap
              className="mt-0.5 h-4 w-4 shrink-0 text-primary"
              aria-hidden="true"
            />
            <span className="text-sm text-muted-foreground">
              Connect an AI provider so the app can respond to you
            </span>
          </li>
          <li className="flex items-start gap-3">
            <MessageSquare
              className="mt-0.5 h-4 w-4 shrink-0 text-primary"
              aria-hidden="true"
            />
            <span className="text-sm text-muted-foreground">
              Add a messaging channel to interact from your favourite apps
            </span>
          </li>
        </ul>
      </div>

      <Button onClick={onNext} className="w-full sm:w-auto">
        Get started
        <ChevronRight aria-hidden="true" />
      </Button>
    </div>
  );
}

// ─── AI Provider Step ──────────────────────────────────────────────────────────

interface AIProviderStepProps {
  onNext: () => void;
  onBack: () => void;
}

function AIProviderStep({ onNext, onBack }: AIProviderStepProps) {
  const {
    providersWithKeyStatus,
    loadProvidersWithKeyStatus,
    saveApiKeyForProvider,
    saveProviderConfig,
    providersWithModels,
    loadProvidersAndModels,
  } = useLLMStore();

  const [selectedProviderId, setSelectedProviderId] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    loadProvidersWithKeyStatus();
    loadProvidersAndModels();
  }, [loadProvidersWithKeyStatus, loadProvidersAndModels]);

  useEffect(() => {
    if (providersWithKeyStatus.length > 0 && !selectedProviderId) {
      setSelectedProviderId(providersWithKeyStatus[0].id);
    }
  }, [providersWithKeyStatus, selectedProviderId]);

  const selectedProvider = providersWithKeyStatus.find(
    (p) => p.id === selectedProviderId
  );
  const requiresApiKey = selectedProvider?.requiresApiKey ?? false;

  async function handleSave() {
    if (!selectedProviderId) {
      onNext();
      return;
    }
    setIsSaving(true);
    try {
      if (requiresApiKey && apiKey.trim()) {
        try {
          await saveApiKeyForProvider(selectedProviderId, apiKey.trim());
        } catch (err) {
          console.error("[Onboarding] Failed to save API key:", err);
        }
      }
      // Use first available model for selected provider, or empty string
      const providerWithModels = providersWithModels.find(
        (p) => p.id === selectedProviderId
      );
      const modelId = providerWithModels?.models?.[0]?.id ?? "";
      try {
        await saveProviderConfig(selectedProviderId, modelId);
      } catch (err) {
        console.error("[Onboarding] Failed to save provider config:", err);
      }
      onNext();
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <div className="w-full max-w-md space-y-6">
      <div className="space-y-1">
        <h2 className="text-2xl font-bold tracking-tight">Set up AI</h2>
        <p className="text-sm text-muted-foreground">
          Choose an AI provider and enter your API key to power the assistant.
        </p>
      </div>

      <div className="space-y-4">
        <div className="space-y-1.5">
          <Label htmlFor="provider-select">Provider</Label>
          {providersWithKeyStatus.length === 0 ? (
            <p className="text-sm text-muted-foreground">Loading providers…</p>
          ) : (
            <select
              id="provider-select"
              value={selectedProviderId}
              onChange={(e) => {
                setSelectedProviderId(e.target.value);
                setApiKey("");
              }}
              className={cn(
                "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground ring-offset-background",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                "disabled:cursor-not-allowed disabled:opacity-50"
              )}
            >
              {providersWithKeyStatus.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                </option>
              ))}
            </select>
          )}
        </div>

        {requiresApiKey && (
          <div className="space-y-1.5">
            <Label htmlFor="api-key">API Key</Label>
            <Input
              id="api-key"
              type="password"
              placeholder="sk-…"
              value={apiKey}
              onChange={(e) => setApiKey(e.target.value)}
            />
            <p className="flex items-center gap-1.5 text-xs text-muted-foreground">
              <Key className="h-3 w-3" aria-hidden="true" />
              API key is stored securely in your OS keyring
            </p>
          </div>
        )}
      </div>

      <div className="flex items-center justify-between gap-3">
        <Button variant="ghost" size="sm" onClick={onBack}>
          <ChevronLeft aria-hidden="true" />
          Back
        </Button>

        <div className="flex items-center gap-3">
          <Button variant="link" size="sm" onClick={onNext}>
            <SkipForward className="h-3.5 w-3.5" aria-hidden="true" />
            Skip for now
          </Button>
          <Button onClick={handleSave} disabled={isSaving}>
            {isSaving ? "Saving…" : "Save & continue"}
            <ChevronRight aria-hidden="true" />
          </Button>
        </div>
      </div>
    </div>
  );
}

// ─── Channels Step ─────────────────────────────────────────────────────────────

interface ChannelsStepProps {
  onNext: () => void;
  onBack: () => void;
}

function ChannelsStep({ onNext, onBack }: ChannelsStepProps) {
  const { updateTelegramConfig, testConnection } = useChannelStore();
  const [selectedChannelId, setSelectedChannelId] = useState<string | null>(
    null
  );
  const [telegramToken, setTelegramToken] = useState("");
  const [telegramChatIds, setTelegramChatIds] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<boolean | null>(null);

  async function handleTestConnection() {
    setIsTesting(true);
    setTestResult(null);
    try {
      const result = await testConnection("telegram");
      setTestResult(result);
    } finally {
      setIsTesting(false);
    }
  }

  async function handleSaveTelegram() {
    setIsSaving(true);
    try {
      const config: TelegramChannelConfig = {
        token: telegramToken,
        allowedChatIds: telegramChatIds,
        pollingTimeoutSecs: 30,
      };
      await updateTelegramConfig(config);
      onNext();
    } finally {
      setIsSaving(false);
    }
  }

  return (
    <div className="w-full max-w-lg space-y-6">
      <div className="space-y-1">
        <h2 className="text-2xl font-bold tracking-tight">
          Connect a channel{" "}
          <span className="text-base font-normal text-muted-foreground">
            (optional)
          </span>
        </h2>
        <p className="text-sm text-muted-foreground">
          Channels let you interact with your AI agent from messaging apps.
        </p>
      </div>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        {CHANNEL_REGISTRY.map((channel) => (
          <button
            key={channel.id}
            type="button"
            disabled={!channel.available}
            onClick={() => {
              if (!channel.available) return;
              setSelectedChannelId(
                selectedChannelId === channel.id ? null : channel.id
              );
            }}
            className={cn(
              "relative flex flex-col items-start gap-1 rounded-lg border p-4 text-left transition-colors",
              channel.available
                ? "cursor-pointer hover:bg-accent"
                : "cursor-default opacity-60",
              selectedChannelId === channel.id
                ? "border-primary bg-primary/5 ring-2 ring-primary"
                : "border-border bg-card"
            )}
          >
            {channel.comingSoonLabel && (
              <span className="absolute right-3 top-3 rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
                {channel.comingSoonLabel}
              </span>
            )}
            <span className="text-2xl" aria-hidden="true">
              {channel.iconEmoji}
            </span>
            <span className="font-semibold text-foreground">
              {channel.displayName}
            </span>
            <span className="text-xs text-muted-foreground">
              {channel.description}
            </span>
          </button>
        ))}
      </div>

      {selectedChannelId === "telegram" && (
        <div className="space-y-4 rounded-lg border border-border bg-card p-4">
          <h3 className="font-semibold">Telegram Bot Configuration</h3>

          <div className="space-y-1.5">
            <Label htmlFor="tg-token">Bot Token</Label>
            <Input
              id="tg-token"
              type="password"
              placeholder="123456:ABC-DEF…"
              value={telegramToken}
              onChange={(e) => setTelegramToken(e.target.value)}
            />
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="tg-chat-ids">Allowed Chat IDs</Label>
            <Input
              id="tg-chat-ids"
              type="text"
              placeholder="123456789, -1001234567890"
              value={telegramChatIds}
              onChange={(e) => setTelegramChatIds(e.target.value)}
            />
            <p className="text-xs text-muted-foreground">
              Comma-separated Telegram chat IDs allowed to interact with the bot
            </p>
          </div>

          <div className="flex items-center gap-3">
            <Button
              variant="outline"
              size="sm"
              onClick={handleTestConnection}
              disabled={isTesting || !telegramToken.trim()}
            >
              {isTesting ? "Testing…" : "Test Connection"}
            </Button>
            {testResult !== null && (
              <span
                className={cn(
                  "text-sm",
                  testResult ? "text-green-600" : "text-destructive"
                )}
              >
                {testResult ? "Connected successfully" : "Connection failed"}
              </span>
            )}
          </div>

          <Button
            onClick={handleSaveTelegram}
            disabled={isSaving || !telegramToken.trim()}
            className="w-full"
          >
            {isSaving ? "Saving…" : "Save"}
          </Button>
        </div>
      )}

      <div className="flex items-center justify-between gap-3">
        <Button variant="ghost" size="sm" onClick={onBack}>
          <ChevronLeft aria-hidden="true" />
          Back
        </Button>

        <div className="flex items-center gap-3">
          <Button variant="link" size="sm" onClick={onNext}>
            <SkipForward className="h-3.5 w-3.5" aria-hidden="true" />
            Skip for now
          </Button>
          <Button onClick={onNext}>
            Continue
            <ChevronRight aria-hidden="true" />
          </Button>
        </div>
      </div>
    </div>
  );
}

// ─── Done Step ─────────────────────────────────────────────────────────────────

interface DoneStepProps {
  onComplete: () => void;
}

function DoneStep({ onComplete }: DoneStepProps) {
  return (
    <div className="flex w-full max-w-md flex-col items-center gap-8 text-center">
      <div className="flex h-20 w-20 items-center justify-center rounded-full bg-green-100 dark:bg-green-900/30">
        <CheckCircle2
          className="h-10 w-10 text-green-600 dark:text-green-400"
          aria-hidden="true"
        />
      </div>

      <div className="space-y-2">
        <h1 className="text-3xl font-bold tracking-tight">You're all set!</h1>
        <p className="text-muted-foreground">
          {APP_IDENTITY.productName} is ready. You can adjust all settings
          anytime from the Settings page.
        </p>
      </div>

      <Button onClick={onComplete} size="lg" className="w-full sm:w-auto">
        Open {APP_IDENTITY.productName}
        <ChevronRight aria-hidden="true" />
      </Button>
    </div>
  );
}
