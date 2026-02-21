import { createFileRoute, useNavigate } from "@tanstack/react-router";
import {
	Bot,
	CheckCircle2,
	ChevronLeft,
	ChevronRight,
	Eye,
	EyeOff,
	Key,
	Loader2,
	MessageSquare,
	SkipForward,
	User,
	Zap,
} from "lucide-react";
import { useEffect, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { APP_IDENTITY } from "@/config/app-identity";
import { getAppIdentityWithDisplayName } from "@/lib/app-display-name";
import { CHANNEL_REGISTRY } from "@/lib/channel-registry";
import type { TestResult } from "@/lib/models";
import { cn } from "@/lib/utils";
import { useAppSettingsStore } from "@/stores/appSettingsStore";
import {
	type DiscordChannelConfig,
	type MatrixChannelConfig,
	type SlackChannelConfig,
	type TelegramChannelConfig,
	useChannelStore,
} from "@/stores/channelStore";
import { useLLMStore } from "@/stores/llm";

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
			{currentStep !== "done" && <ProgressHeader currentStep={currentStep} />}

			<div className="flex flex-1 items-center justify-center p-6">
				{currentStep === "welcome" && <WelcomeStep onNext={goNext} />}
				{currentStep === "identity" && (
					<IdentityStep onNext={goNext} onBack={goBack} />
				)}
				{currentStep === "ai-provider" && (
					<AIProviderStep onNext={goNext} onBack={goBack} />
				)}
				{currentStep === "channels" && (
					<ChannelsStep onNext={goNext} onBack={goBack} />
				)}
				{currentStep === "done" && <DoneStep onComplete={handleComplete} />}
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

			<Button onClick={onNext} className="w-full sm:w-auto">
				Get started
				<ChevronRight aria-hidden="true" />
			</Button>
		</div>
	);
}

// ─── Identity Step ─────────────────────────────────────────────────────────────

interface IdentityStepProps {
	onNext: () => void;
	onBack: () => void;
}

function IdentityStep({ onNext, onBack }: IdentityStepProps) {
	const { userName, appDisplayName, setUserIdentity, loadUserIdentity } =
		useAppSettingsStore();
	const [nameInput, setNameInput] = useState<string>("");
	const [displayNameInput, setDisplayNameInput] = useState<string>(
		APP_IDENTITY.productName,
	);
	const [isSaving, setIsSaving] = useState(false);

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
		setIsSaving(true);
		try {
			await setUserIdentity(
				nameInput.trim() || null,
				displayNameInput.trim() || null,
			);
			onNext();
		} catch (error) {
			console.error("Failed to save identity:", error);
		} finally {
			setIsSaving(false);
		}
	}

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
						{isSaving ? "Saving..." : "Save & continue"}
						<ChevronRight aria-hidden="true" />
					</Button>
				</div>
			</div>
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
		testProviderConnection,
		getApiKey,
	} = useLLMStore();

	const [selectedProviderId, setSelectedProviderId] = useState("");
	const [apiKey, setApiKey] = useState("");
	const [showApiKey, setShowApiKey] = useState(false);
	const [isSaving, setIsSaving] = useState(false);
	const [isTesting, setIsTesting] = useState(false);
	const [testResult, setTestResult] = useState<TestResult | null>(null);

	useEffect(() => {
		loadProvidersWithKeyStatus();
		loadProvidersAndModels();
	}, [loadProvidersWithKeyStatus, loadProvidersAndModels]);

	useEffect(() => {
		if (providersWithKeyStatus.length > 0 && !selectedProviderId) {
			setSelectedProviderId(providersWithKeyStatus[0].id);
		}
	}, [providersWithKeyStatus, selectedProviderId]);

	// Load existing API key when provider changes
	useEffect(() => {
		const selectedProvider = providersWithKeyStatus.find(
			(p) => p.id === selectedProviderId,
		);
		if (selectedProvider?.hasApiKey) {
			getApiKey(selectedProviderId)
				.then(setApiKey)
				.catch(() => setApiKey(""));
		} else {
			setApiKey("");
		}
		setTestResult(null);
	}, [selectedProviderId, providersWithKeyStatus, getApiKey]);

	const selectedProvider = providersWithKeyStatus.find(
		(p) => p.id === selectedProviderId,
	);
	const requiresApiKey = selectedProvider?.requiresApiKey ?? false;

	async function handleTestConnection() {
		if (!selectedProviderId) return;
		if (requiresApiKey && !apiKey.trim()) return;

		setIsTesting(true);
		setTestResult(null);
		try {
			const result = await testProviderConnection(
				selectedProviderId,
				apiKey.trim(),
			);
			setTestResult(result);
		} catch (error) {
			const errorMessage =
				error instanceof Error ? error.message : String(error);
			setTestResult({
				success: false,
				error: errorMessage,
			});
		} finally {
			setIsTesting(false);
		}
	}

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
				(p) => p.id === selectedProviderId,
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
						<p className="text-sm text-muted-foreground">
							Loading providers...
						</p>
					) : (
						<select
							id="provider-select"
							value={selectedProviderId}
							onChange={(e) => {
								setSelectedProviderId(e.target.value);
								setTestResult(null);
							}}
							className={cn(
								"flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm text-foreground ring-offset-background",
								"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
								"disabled:cursor-not-allowed disabled:opacity-50",
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
						<div className="relative">
							<Input
								id="api-key"
								type={showApiKey ? "text" : "password"}
								placeholder="sk-..."
								value={apiKey}
								onChange={(e) => {
									setApiKey(e.target.value);
									setTestResult(null);
								}}
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
						<p className="flex items-center gap-1.5 text-xs text-muted-foreground">
							<Key className="h-3 w-3" aria-hidden="true" />
							API key is stored securely in your OS keyring
						</p>
					</div>
				)}

				{/* Test Connection Button and Result */}
				{selectedProviderId && (!requiresApiKey || apiKey.trim()) && (
					<div className="flex items-center gap-3">
						<Button
							variant="outline"
							size="sm"
							onClick={handleTestConnection}
							disabled={isTesting}
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
						{testResult && (
							<span
								className={cn(
									"text-sm font-medium",
									testResult.success ? "text-green-600" : "text-destructive",
								)}
							>
								{testResult.success
									? `Connected${testResult.latencyMs ? ` (${testResult.latencyMs}ms)` : ""}`
									: "Connection failed"}
							</span>
						)}
					</div>
				)}

				{/* Test Result Details */}
				{testResult && !testResult.success && testResult.error && (
					<div className="rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive">
						{testResult.error}
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
						{isSaving ? "Saving..." : "Save & continue"}
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
	const {
		updateTelegramConfig,
		updateDiscordConfig,
		updateMatrixConfig,
		updateSlackConfig,
		testConnection,
	} = useChannelStore();

	const [selectedChannelId, setSelectedChannelId] = useState<string | null>(
		null,
	);

	// Telegram state
	const [telegramToken, setTelegramToken] = useState("");
	const [telegramChatIds, setTelegramChatIds] = useState("");
	const [telegramTimeout, setTelegramTimeout] = useState("30");

	// Discord state
	const [discordBotToken, setDiscordBotToken] = useState("");
	const [discordGuildIds, setDiscordGuildIds] = useState("");
	const [discordChannelIds, setDiscordChannelIds] = useState("");

	// Matrix state
	const [matrixHomeserver, setMatrixHomeserver] = useState("");
	const [matrixUsername, setMatrixUsername] = useState("");
	const [matrixAccessToken, setMatrixAccessToken] = useState("");
	const [matrixRoomIds, setMatrixRoomIds] = useState("");

	// Slack state
	const [slackBotToken, setSlackBotToken] = useState("");
	const [slackAppToken, setSlackAppToken] = useState("");
	const [slackChannelIds, setSlackChannelIds] = useState("");

	const [isSaving, setIsSaving] = useState(false);
	const [isTesting, setIsTesting] = useState(false);
	const [testResult, setTestResult] = useState<"ok" | "fail" | null>(null);

	function resetForm() {
		setTestResult(null);
	}

	async function handleTestConnection() {
		if (!selectedChannelId) return;

		setIsTesting(true);
		setTestResult(null);
		try {
			// For Telegram, we need to save the token first before testing
			if (selectedChannelId === "telegram" && telegramToken.trim()) {
				const config: TelegramChannelConfig = {
					token: telegramToken.trim(),
					allowedChatIds: telegramChatIds.trim(),
					pollingTimeoutSecs: Number(telegramTimeout) || 30,
				};
				await updateTelegramConfig(config);
			}

			// For Discord, save token first
			if (selectedChannelId === "discord" && discordBotToken.trim()) {
				const config: DiscordChannelConfig = {
					botToken: discordBotToken.trim(),
					allowedGuildIds: discordGuildIds.trim(),
					allowedChannelIds: discordChannelIds.trim(),
				};
				await updateDiscordConfig(config);
			}

			// For Matrix, save config first
			if (selectedChannelId === "matrix" && matrixAccessToken.trim()) {
				const config: MatrixChannelConfig = {
					homeserverUrl: matrixHomeserver.trim() || "https://matrix.org",
					username: matrixUsername.trim(),
					accessToken: matrixAccessToken.trim(),
					allowedRoomIds: matrixRoomIds.trim(),
				};
				await updateMatrixConfig(config);
			}

			// For Slack, save config first
			if (selectedChannelId === "slack" && slackBotToken.trim()) {
				const config: SlackChannelConfig = {
					botToken: slackBotToken.trim(),
					appToken: slackAppToken.trim(),
					allowedChannelIds: slackChannelIds.trim(),
				};
				await updateSlackConfig(config);
			}

			const result = await testConnection(selectedChannelId);
			setTestResult(result ? "ok" : "fail");
		} catch {
			setTestResult("fail");
		} finally {
			setIsTesting(false);
		}
	}

	async function handleSaveChannel() {
		if (!selectedChannelId) {
			onNext();
			return;
		}

		setIsSaving(true);
		try {
			if (selectedChannelId === "telegram") {
				const config: TelegramChannelConfig = {
					token: telegramToken.trim(),
					allowedChatIds: telegramChatIds.trim(),
					pollingTimeoutSecs: Number(telegramTimeout) || 30,
				};
				await updateTelegramConfig(config);
			} else if (selectedChannelId === "discord") {
				const config: DiscordChannelConfig = {
					botToken: discordBotToken.trim(),
					allowedGuildIds: discordGuildIds.trim(),
					allowedChannelIds: discordChannelIds.trim(),
				};
				await updateDiscordConfig(config);
			} else if (selectedChannelId === "matrix") {
				const config: MatrixChannelConfig = {
					homeserverUrl: matrixHomeserver.trim() || "https://matrix.org",
					username: matrixUsername.trim(),
					accessToken: matrixAccessToken.trim(),
					allowedRoomIds: matrixRoomIds.trim(),
				};
				await updateMatrixConfig(config);
			} else if (selectedChannelId === "slack") {
				const config: SlackChannelConfig = {
					botToken: slackBotToken.trim(),
					appToken: slackAppToken.trim(),
					allowedChannelIds: slackChannelIds.trim(),
				};
				await updateSlackConfig(config);
			}
			onNext();
		} finally {
			setIsSaving(false);
		}
	}

	const canSave = (): boolean => {
		switch (selectedChannelId) {
			case "telegram":
				return telegramToken.trim().length > 0;
			case "discord":
				return discordBotToken.trim().length > 0;
			case "matrix":
				return (
					matrixAccessToken.trim().length > 0 &&
					matrixHomeserver.trim().length > 0
				);
			case "slack":
				return slackBotToken.trim().length > 0;
			default:
				return false;
		}
	};

	const canTest = (): boolean => {
		return canSave();
	};

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
								selectedChannelId === channel.id ? null : channel.id,
							);
							resetForm();
						}}
						className={cn(
							"relative flex flex-col items-start gap-1 rounded-lg border p-4 text-left transition-colors",
							channel.available
								? "cursor-pointer hover:bg-accent"
								: "cursor-default opacity-60",
							selectedChannelId === channel.id
								? "border-primary bg-primary/5 ring-2 ring-primary"
								: "border-border bg-card",
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

			{/* Telegram Config */}
			{selectedChannelId === "telegram" && (
				<ChannelConfigCard
					title="Telegram Bot Configuration"
					icon="✈️"
					testResult={testResult}
					isTesting={isTesting}
					isSaving={isSaving}
					canTest={canTest()}
					canSave={canSave()}
					onTest={handleTestConnection}
					onSave={handleSaveChannel}
				>
					<div className="space-y-4">
						<div className="space-y-1.5">
							<Label htmlFor="tg-token">Bot Token</Label>
							<Input
								id="tg-token"
								type="password"
								placeholder="123456:ABC-DEF..."
								value={telegramToken}
								onChange={(e) => {
									setTelegramToken(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								Obtained from @BotFather. Stored securely in the OS keyring.
							</p>
						</div>

						<div className="space-y-1.5">
							<Label htmlFor="tg-chat-ids">Allowed Chat IDs</Label>
							<Input
								id="tg-chat-ids"
								type="text"
								placeholder="123456789, -1001234567890"
								value={telegramChatIds}
								onChange={(e) => {
									setTelegramChatIds(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								Comma-separated Telegram chat IDs allowed to interact with the
								bot
							</p>
						</div>

						<div className="space-y-1.5">
							<Label htmlFor="tg-timeout">Polling Timeout (seconds)</Label>
							<Input
								id="tg-timeout"
								type="number"
								min={5}
								max={60}
								value={telegramTimeout}
								onChange={(e) => setTelegramTimeout(e.target.value)}
								className="w-32"
							/>
						</div>
					</div>
				</ChannelConfigCard>
			)}

			{/* Discord Config */}
			{selectedChannelId === "discord" && (
				<ChannelConfigCard
					title="Discord Bot Configuration"
					icon="🎮"
					testResult={testResult}
					isTesting={isTesting}
					isSaving={isSaving}
					canTest={canTest()}
					canSave={canSave()}
					onTest={handleTestConnection}
					onSave={handleSaveChannel}
				>
					<div className="space-y-4">
						<div className="space-y-1.5">
							<Label htmlFor="dc-token">Bot Token</Label>
							<Input
								id="dc-token"
								type="password"
								placeholder="MTExMjM0NTY3ODkwMTIzNDU2.Gh1234.abc..."
								value={discordBotToken}
								onChange={(e) => {
									setDiscordBotToken(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								From Discord Developer Portal. Enable Message Content Intent.
							</p>
						</div>

						<div className="space-y-1.5">
							<Label htmlFor="dc-guilds">Allowed Server (Guild) IDs</Label>
							<Input
								id="dc-guilds"
								type="text"
								placeholder="123456789012345678, 987654321098765432"
								value={discordGuildIds}
								onChange={(e) => {
									setDiscordGuildIds(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								Comma-separated Discord server IDs. Leave empty for all servers.
							</p>
						</div>

						<div className="space-y-1.5">
							<Label htmlFor="dc-channels">Allowed Channel IDs</Label>
							<Input
								id="dc-channels"
								type="text"
								placeholder="111222333444555666, 666555444333222111"
								value={discordChannelIds}
								onChange={(e) => {
									setDiscordChannelIds(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								Comma-separated Discord channel IDs. Leave empty for all
								channels.
							</p>
						</div>
					</div>
				</ChannelConfigCard>
			)}

			{/* Matrix Config */}
			{selectedChannelId === "matrix" && (
				<ChannelConfigCard
					title="Matrix Configuration"
					icon="🔷"
					testResult={testResult}
					isTesting={isTesting}
					isSaving={isSaving}
					canTest={canTest()}
					canSave={canSave()}
					onTest={handleTestConnection}
					onSave={handleSaveChannel}
				>
					<div className="space-y-4">
						<div className="space-y-1.5">
							<Label htmlFor="mx-homeserver">Homeserver URL</Label>
							<Input
								id="mx-homeserver"
								type="url"
								placeholder="https://matrix.org"
								value={matrixHomeserver}
								onChange={(e) => {
									setMatrixHomeserver(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								Full URL of your Matrix homeserver including https://
							</p>
						</div>

						<div className="space-y-1.5">
							<Label htmlFor="mx-username">Username (MXID)</Label>
							<Input
								id="mx-username"
								type="text"
								placeholder="@mybot:matrix.org"
								value={matrixUsername}
								onChange={(e) => {
									setMatrixUsername(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								Full Matrix ID including the server part
							</p>
						</div>

						<div className="space-y-1.5">
							<Label htmlFor="mx-token">Access Token</Label>
							<Input
								id="mx-token"
								type="password"
								placeholder="syt_dXNlcm5hbWU_abc123..."
								value={matrixAccessToken}
								onChange={(e) => {
									setMatrixAccessToken(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								From Element Settings &gt; Help &amp; About. Stored securely.
							</p>
						</div>

						<div className="space-y-1.5">
							<Label htmlFor="mx-rooms">Allowed Room IDs</Label>
							<Input
								id="mx-rooms"
								type="text"
								placeholder="!abc123:matrix.org, !xyz789:matrix.org"
								value={matrixRoomIds}
								onChange={(e) => {
									setMatrixRoomIds(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								Comma-separated room IDs. Leave empty for all joined rooms.
							</p>
						</div>
					</div>
				</ChannelConfigCard>
			)}

			{/* Slack Config */}
			{selectedChannelId === "slack" && (
				<ChannelConfigCard
					title="Slack Configuration"
					icon="💬"
					testResult={testResult}
					isTesting={isTesting}
					isSaving={isSaving}
					canTest={canTest()}
					canSave={canSave()}
					onTest={handleTestConnection}
					onSave={handleSaveChannel}
				>
					<div className="space-y-4">
						<div className="space-y-1.5">
							<Label htmlFor="sl-bot-token">Bot Token</Label>
							<Input
								id="sl-bot-token"
								type="password"
								placeholder="xoxb-xxxxxxxxxxxx-xxxxxxxxxxxx-xxxxxxxxxxxx"
								value={slackBotToken}
								onChange={(e) => {
									setSlackBotToken(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								Bot User OAuth Token from OAuth &amp; Permissions. Starts with
								xoxb-
							</p>
						</div>

						<div className="space-y-1.5">
							<Label htmlFor="sl-app-token">App Token (Socket Mode)</Label>
							<Input
								id="sl-app-token"
								type="password"
								placeholder="xapp-1-XXXXXXXXX-0000000000000-abc..."
								value={slackAppToken}
								onChange={(e) => {
									setSlackAppToken(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								App-Level Token for Socket Mode. Starts with xapp-
							</p>
						</div>

						<div className="space-y-1.5">
							<Label htmlFor="sl-channels">Allowed Channel IDs</Label>
							<Input
								id="sl-channels"
								type="text"
								placeholder="C01234567AB, C09876543ZZ"
								value={slackChannelIds}
								onChange={(e) => {
									setSlackChannelIds(e.target.value);
									setTestResult(null);
								}}
							/>
							<p className="text-xs text-muted-foreground">
								Comma-separated Slack channel IDs. Leave empty for all channels.
							</p>
						</div>
					</div>
				</ChannelConfigCard>
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

// ─── Channel Config Card (shared component) ───────────────────────────────────────

interface ChannelConfigCardProps {
	title: string;
	icon: string;
	children: React.ReactNode;
	testResult: "ok" | "fail" | null;
	isTesting: boolean;
	isSaving: boolean;
	canTest: boolean;
	canSave: boolean;
	onTest: () => void;
	onSave: () => void;
}

function ChannelConfigCard({
	title,
	icon,
	children,
	testResult,
	isTesting,
	isSaving,
	canTest,
	canSave,
	onTest,
	onSave,
}: ChannelConfigCardProps) {
	return (
		<div className="space-y-4 rounded-lg border border-border bg-card p-4">
			<h3 className="flex items-center gap-2 font-semibold">
				<span aria-hidden="true">{icon}</span>
				{title}
			</h3>

			{children}

			<div className="flex items-center gap-3 pt-2">
				<Button
					variant="outline"
					size="sm"
					onClick={onTest}
					disabled={isTesting || !canTest}
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

				{testResult === "ok" && (
					<span className="text-sm font-medium text-green-600">
						Connected successfully
					</span>
				)}
				{testResult === "fail" && (
					<span className="text-sm font-medium text-destructive">
						Connection failed
					</span>
				)}

				<div className="flex-1" />

				<Button size="sm" onClick={onSave} disabled={isSaving || !canSave}>
					{isSaving ? (
						<>
							<Loader2 className="mr-2 h-4 w-4 animate-spin" />
							Saving...
						</>
					) : (
						"Save"
					)}
				</Button>
			</div>
		</div>
	);
}

// ─── Done Step ─────────────────────────────────────────────────────────────────

interface DoneStepProps {
	onComplete: () => void;
}

function DoneStep({ onComplete }: DoneStepProps) {
	const appDisplayName = useAppSettingsStore((s) => s.appDisplayName);
	const identity = getAppIdentityWithDisplayName(appDisplayName);

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
