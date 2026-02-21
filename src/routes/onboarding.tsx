import { createFileRoute, useNavigate } from "@tanstack/react-router";
import {
  Bot,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  MessageSquare,
  SkipForward,
  User,
  Zap,
} from "lucide-react";
import { useEffect, useState } from "react";

import { AIProviderConfiguration } from "@/components/settings/AIProviderConfiguration";
import { ChannelList } from "@/components/settings/ChannelList";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { APP_IDENTITY } from "@/config/app-identity";
import { cn } from "@/lib/utils";
import { useAppSettingsStore, useAppIdentity } from "@/stores/appSettingsStore";

export const Route = createFileRoute("/onboarding")({
  component: OnboardingPage,
});

type Step = "welcome" | "identity" | "ai-provider" | "channels" | "done";

const STEPS: Step[] = [
  "welcome",
  "identity",
  "ai-provider",
  "channels",
  "done",
];

const PROGRESS_STEPS: Array<{ id: Step; label: string }> = [
  { id: "welcome", label: "Welcome" },
  { id: "identity", label: "Identity" },
  { id: "ai-provider", label: "AI Provider" },
  { id: "channels", label: "Channels" },
];

function OnboardingPage() {
  const [currentStep, setCurrentStep] = useState<Step>("welcome");
  const [isSaving, setIsSaving] = useState(false);
  const completeOnboarding = useAppSettingsStore((s) => s.completeOnboarding);
  const navigate = useNavigate();

  // Note: AIProviderManagement and ChannelList handle their own state persistence

  function goNext() {
    const idx = STEPS.indexOf(currentStep);
    if (idx < STEPS.length - 1) {
      setNextStep(STEPS[idx + 1]);
    }
  }

  function setNextStep(step: Step) {
    setIsSaving(false); // Reset saving state when changing steps
    setCurrentStep(step);
  }

  function goBack() {
    const idx = STEPS.indexOf(currentStep);
    if (idx > 0) {
      setIsSaving(false); // Reset saving state when changing steps
      setCurrentStep(STEPS[idx - 1]);
    }
  }

  async function handleContinueClick() {
    // AIProviderManagement and ChannelList handle their own persistence
    // so we just proceed to the next step
    goNext();
  }

  function handleComplete() {
    completeOnboarding();
    navigate({ to: "/" });
  }

  // Determine if Continue button should be enabled
  const canContinue = () => {
    if (currentStep === "welcome") return true;
    if (currentStep === "identity") return true;
    if (currentStep === "ai-provider") return true; // Can skip
    if (currentStep === "channels") return true; // Can skip
    return true;
  };

  // Check if we should show Skip button
  const canSkip = () => {
    return currentStep === "ai-provider" || currentStep === "channels";
  };

  return (
    <div className="flex min-h-screen flex-col bg-background">
      {currentStep !== "done" && (
        <ProgressHeader
          currentStep={currentStep}
          onBack={currentStep !== "welcome" ? goBack : undefined}
          onContinue={handleContinueClick}
          onSkip={canSkip() ? goNext : undefined}
          canContinue={canContinue()}
          isSaving={isSaving}
          continueLabel={
            currentStep === "ai-provider" || currentStep === "channels"
              ? "Save & Continue"
              : undefined
          }
        />
      )}

      <div className="flex flex-1 items-center justify-center p-6">
        {currentStep === "welcome" && <WelcomeStep />}
        {currentStep === "identity" && (
          <IdentityStep onNext={goNext} />
        )}
        {currentStep === "ai-provider" && (
          <AIProviderStep />
        )}
        {currentStep === "channels" && (
          <ChannelsStep />
        )}
        {currentStep === "done" && <DoneStep onComplete={handleComplete} />}
      </div>
    </div>
  );
}

// ─── Progress Header with Navigation ───────────────────────────────────────────────────────────

interface ProgressHeaderProps {
  currentStep: Step;
  onBack?: () => void;
  onContinue: () => void;
  onSkip?: () => void;
  canContinue: boolean;
  isSaving: boolean;
  continueLabel?: string;
}

function ProgressHeader({
  currentStep,
  onBack,
  onContinue,
  onSkip,
  canContinue,
  isSaving,
  continueLabel,
}: ProgressHeaderProps) {
  const currentIndex = PROGRESS_STEPS.findIndex((s) => s.id === currentStep);

  return (
    <header className="border-b border-border">
      {/* Progress indicators */}
      <div className="px-6 py-4">
        <div className="mx-auto flex max-w-lg items-center justify-between">
          {PROGRESS_STEPS.map((step, idx) => {
            const isCompleted = idx < currentIndex;
            const isCurrent = idx === currentIndex;

            return (
              <div key={step.id} className="flex items-center gap-2">
                <div
                  className={cn(
                    "flex h-8 w-8 items-center justify-center rounded-full text-sm font-semibold transition-colors",
                    isCompleted && "bg-primary text-primary-foreground",
                    isCurrent &&
                      "border-2 border-primary bg-background text-primary",
                    !isCompleted &&
                      !isCurrent &&
                      "border-2 border-muted bg-background text-muted-foreground",
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
                    isCurrent ? "text-foreground" : "text-muted-foreground",
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
      </div>

      {/* Navigation buttons in header */}
      <div className="flex items-center justify-between border-t border-border/50 bg-muted/30 px-6 py-3">
        <div className="flex items-center gap-2">
          {onBack && (
            <Button variant="ghost" size="sm" onClick={onBack}>
              <ChevronLeft aria-hidden="true" />
              Back
            </Button>
          )}
        </div>

        <div className="flex items-center gap-3">
          {onSkip && (
            <Button variant="link" size="sm" onClick={onSkip}>
              <SkipForward className="h-3.5 w-3.5" aria-hidden="true" />
              Skip for now
            </Button>
          )}
          <Button
            onClick={onContinue}
            disabled={!canContinue || isSaving}
          >
            {isSaving ? (
              "Saving..."
            ) : (
              <>
                {continueLabel || "Continue"}
                <ChevronRight aria-hidden="true" />
              </>
            )}
          </Button>
        </div>
      </div>
    </header>
  );
}

// ─── Welcome Step ──────────────────────────────────────────────────────────────

function WelcomeStep() {
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
            <User
              className="mt-0.5 h-4 w-4 shrink-0 text-primary"
              aria-hidden="true"
            />
            <span className="text-sm text-muted-foreground">
              Personalize your experience with your name and a custom app name
            </span>
          </li>
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
    </div>
  );
}

// ─── Identity Step ─────────────────────────────────────────────────────────────

interface IdentityStepProps {
  onNext: () => void;
}

function IdentityStep({ onNext }: IdentityStepProps) {
  const { userName, appDisplayName, setUserIdentity, loadUserIdentity } =
    useAppSettingsStore();
  const [nameInput, setNameInput] = useState<string>("");
  const [displayNameInput, setDisplayNameInput] = useState<string>(
    APP_IDENTITY.productName,
  );

  // Load existing identity on mount
  useEffect(() => {
    loadUserIdentity();
  }, [loadUserIdentity]);

  // Update form when identity loads
  useEffect(() => {
    setNameInput(userName ?? "");
    setDisplayNameInput(appDisplayName ?? APP_IDENTITY.productName);
  }, [userName, appDisplayName]);

  async function handleSave() {
    try {
      await setUserIdentity(
        nameInput.trim() || null,
        displayNameInput.trim() || null,
      );
      onNext();
    } catch (error) {
      console.error("Failed to save identity:", error);
    }
  }

  // Expose save function to parent via window (hacky but works for this case)
  useEffect(() => {
    (window as unknown as { __identitySave: () => void }).__identitySave = handleSave;
    return () => {
      delete (window as unknown as { __identitySave?: () => void }).__identitySave;
    };
  }, [handleSave]);

  return (
    <div className="w-full max-w-md space-y-6">
      <div className="space-y-1">
        <h2 className="text-2xl font-bold tracking-tight">
          Personalize your experience
        </h2>
        <p className="text-sm text-muted-foreground">
          Tell us a bit about yourself. This helps personalize your
          interactions.
        </p>
      </div>

      <div className="space-y-4">
        <div className="space-y-1.5">
          <Label htmlFor="user-name">Your Name</Label>
          <Input
            id="user-name"
            type="text"
            placeholder="Enter your name for personalization"
            value={nameInput}
            onChange={(e) => setNameInput(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            Optional. Used to personalize responses.
          </p>
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="app-display-name">App Display Name</Label>
          <Input
            id="app-display-name"
            type="text"
            placeholder="What would you like to call me?"
            value={displayNameInput}
            onChange={(e) => setDisplayNameInput(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            This is how the assistant will refer to itself. Defaults to{" "}
            {APP_IDENTITY.productName}.
          </p>
        </div>
      </div>
    </div>
  );
}

// ─── AI Provider Step ──────────────────────────────────────────────────────────

function AIProviderStep() {
  return (
    <div className="w-full max-w-3xl space-y-6">
      <div className="space-y-1">
        <h2 className="text-2xl font-bold tracking-tight">Set up AI</h2>
        <p className="text-sm text-muted-foreground">
          Choose an AI provider and enter your API key to power the assistant.
        </p>
      </div>

      <AIProviderConfiguration
        showGlobalDefault={false}
        showHeader={false}
        compact={true}
      />
    </div>
  );
}

// ─── Channels Step ─────────────────────────────────────────────────────────────

function ChannelsStep() {
  return (
    <div className="w-full max-w-2xl space-y-6">
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

      <ChannelList />
    </div>
  );
}

// ─── Done Step ─────────────────────────────────────────────────────────────────

interface DoneStepProps {
  onComplete: () => void;
}

function DoneStep({ onComplete }: DoneStepProps) {
  const identity = useAppIdentity();

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
          {identity.productName} is ready. You can adjust all settings anytime
          from the Settings page.
        </p>
      </div>

      <Button onClick={onComplete} size="lg" className="w-full sm:w-auto">
        Open {identity.productName}
        <ChevronRight aria-hidden="true" />
      </Button>
    </div>
  );
}
