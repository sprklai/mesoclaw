<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import { providersStore } from '$lib/stores/providers.svelte';
	import { channelsStore } from '$lib/stores/channels.svelte';
	import { configStore } from '$lib/stores/config.svelte';
	import ProvidersSettings from '$lib/components/settings/ProvidersSettings.svelte';
	import ChannelsSettings from '$lib/components/settings/ChannelsSettings.svelte';
	import {
		PromptInputModelSelect,
		PromptInputModelSelectTrigger,
		PromptInputModelSelectContent,
		PromptInputModelSelectItem,
		PromptInputModelSelectValue
	} from '$lib/components/ai-elements/prompt-input';
	import ChevronLeft from '@lucide/svelte/icons/chevron-left';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { onMount } from 'svelte';

	let {
		detectedTimezone = '',
		missing = [] as string[],
		oncomplete
	}: {
		detectedTimezone: string;
		missing?: string[];
		oncomplete: () => void;
	} = $props();

	const TOTAL_STEPS = 4;
	let step = $state(1);
	let userName = $state('');
	let userLocation = $state('');
	let userTimezone = $state('');
	let saving = $state(false);
	let error = $state('');
	let disclaimerAccepted = $state(false);

	const stepLabels = ['AI Provider', 'Default Model', 'Channels', 'Your Profile'];

	const currentModelLabel = $derived(
		providersStore.configuredModels.find((m) => m.value === providersStore.selectedModel)?.label ??
			''
	);

	function canAdvance(fromStep: number): boolean {
		if (fromStep === 1) return providersStore.hasUsableModel;
		if (fromStep === 2) return !!providersStore.selectedModel;
		if (fromStep === 3) return true;
		return false;
	}

	$effect(() => {
		if (step === 2 && !providersStore.selectedModel && providersStore.configuredModels.length > 0) {
			providersStore.selectedModel = providersStore.configuredModels[0].value;
		}
	});

	async function handleModelNext() {
		const selected = providersStore.selectedModel;
		if (!selected) return;
		const [providerId, ...rest] = selected.split(':');
		const modelId = rest.join(':');
		try {
			await providersStore.setDefault(providerId, modelId);
			await configStore.update({
				provider_name: providerId,
				provider_type: providerId,
				provider_model_id: modelId
			});
			step = 3;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		}
	}

	onMount(async () => {
		userTimezone = detectedTimezone;
		await providersStore.load();
		await channelsStore.load();

		// Auto-skip to first incomplete step based on missing fields
		if (missing.length > 0) {
			const needsApiKey = missing.includes('api_key');
			const needsProfile = missing.includes('user_name') || missing.includes('user_location');

			if (needsApiKey) {
				step = 1; // Start at provider setup
			} else if (needsProfile) {
				step = 4; // Skip directly to profile
			}
		}
	});

	function stepState(i: number): 'done' | 'active' | 'upcoming' {
		if (i + 1 < step) return 'done';
		if (i + 1 === step) return 'active';
		return 'upcoming';
	}

	async function handleFinish() {
		if (!userName.trim()) {
			error = 'Your name is required';
			return;
		}
		if (!userLocation.trim()) {
			error = 'Your location is required';
			return;
		}
		saving = true;
		error = '';
		try {
			const updates: Record<string, string | boolean | null> = {
				user_name: userName.trim(),
				user_location: userLocation.trim(),
				onboarding_completed: true
			};
			if (userTimezone.trim()) {
				updates.user_timezone = userTimezone.trim();
			}
			await configStore.update(updates as Record<string, unknown>);
			oncomplete();
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}
</script>

<div class="flex min-h-screen items-start justify-center overflow-y-auto bg-background p-4 py-12">
	<div class="w-full max-w-2xl space-y-6">
		<!-- Header -->
		<div class="text-center space-y-2">
			<h1 class="text-3xl font-bold tracking-tight">Welcome to Zenii</h1>
			<p class="text-muted-foreground">Let's get you set up so you can start chatting.</p>
		</div>

		<!-- Step Indicator -->
		<div class="flex items-center justify-center gap-3">
			<Button
				variant="ghost"
				size="icon"
				onclick={() => (step = step - 1)}
				disabled={step === 1}
				aria-label="Previous step"
			>
				<ChevronLeft class="h-5 w-5" />
			</Button>

			{#each stepLabels as label, i (label)}
				{@const state = stepState(i)}
				<div class="flex items-center gap-2">
					<button
						type="button"
						class="flex h-8 w-8 items-center justify-center rounded-full text-sm font-medium transition-colors {state === 'active'
							? 'bg-primary text-primary-foreground'
							: state === 'done'
								? 'bg-green-600 text-white hover:bg-green-500 cursor-pointer'
								: 'bg-muted text-muted-foreground'}"
						disabled={state !== 'done'}
						onclick={() => { if (state === 'done') step = i + 1; }}
					>
						{state === 'done' ? '\u2713' : i + 1}
					</button>
					<span class="text-sm font-medium {state === 'active' ? 'text-foreground' : 'text-muted-foreground'}">
						{label}
					</span>
				</div>
				{#if i < TOTAL_STEPS - 1}
					<div class="h-px w-12 bg-muted-foreground/40"></div>
				{/if}
			{/each}

			<Button
				variant="ghost"
				size="icon"
				onclick={() => (step = step + 1)}
				disabled={step === TOTAL_STEPS || !canAdvance(step)}
				aria-label="Next step"
			>
				<ChevronRight class="h-5 w-5" />
			</Button>
		</div>

		<!-- Step Content -->
		{#if step === 1}
			<div class="space-y-4">
				<Card.Root>
					<Card.Header>
						<div class="flex items-center justify-between">
							<div>
								<Card.Title>Set up an AI Provider</Card.Title>
								<Card.Description>
									Add an API key for at least one provider to enable chat. Expand a provider below, enter
									your key, and save it.
								</Card.Description>
								<p class="mt-2 text-xs text-muted-foreground">
									Need a different provider? Any OpenAI API-compatible service can be added via the
									<strong>+ Add Provider</strong> button above the list.
								</p>
							</div>
							<Button
								onclick={() => (step = 2)}
								disabled={!providersStore.hasUsableModel}
								size="lg"
								class="shrink-0 ml-4"
							>
								{providersStore.hasUsableModel ? 'Next' : 'Add a key first'}
							</Button>
						</div>
					</Card.Header>
				</Card.Root>

				<ProvidersSettings hideDefaultModel />
			</div>
		{:else if step === 2}
			<div class="space-y-4">
				<Card.Root>
					<Card.Header>
						<Card.Title>Choose Your Default Model</Card.Title>
						<Card.Description>
							Select the AI model Zenii will use by default. You can always switch models later in the
							chat toolbar.
						</Card.Description>
					</Card.Header>
					<Card.Content>
						<PromptInputModelSelect
							value={providersStore.selectedModel}
							onValueChange={(v) => {
								if (v) providersStore.selectedModel = v;
							}}
						>
							<PromptInputModelSelectTrigger class="w-full border border-border">
								<PromptInputModelSelectValue
									value={currentModelLabel}
									placeholder="Select a model"
								/>
							</PromptInputModelSelectTrigger>
							<PromptInputModelSelectContent>
								{#each providersStore.configuredModels as model}
									<PromptInputModelSelectItem value={model.value}>
										{model.label}
									</PromptInputModelSelectItem>
								{/each}
							</PromptInputModelSelectContent>
						</PromptInputModelSelect>
					</Card.Content>
				</Card.Root>

				{#if currentModelLabel}
					<div class="rounded-md border border-green-500/50 bg-green-500/10 px-4 py-3 text-sm text-green-700 dark:text-green-400">
						Default model set to <strong>{currentModelLabel}</strong>
					</div>
				{/if}

				<div class="flex justify-between">
					<Button variant="ghost" onclick={() => (step = 1)}>Back</Button>
					<Button onclick={handleModelNext} disabled={!providersStore.selectedModel} size="lg">
						Next
					</Button>
				</div>
			</div>
		{:else if step === 3}
			<div class="space-y-4">
				<Card.Root>
					<Card.Header>
						<div class="flex items-center justify-between">
							<div>
								<Card.Title>Connect Channels</Card.Title>
								<Card.Description>
									Optionally connect messaging channels like Telegram, Slack, or Discord so Zenii can
									reach you there. You can skip this and set them up later in Settings.
								</Card.Description>
							</div>
							<div class="flex gap-2 shrink-0 ml-4">
								<Button variant="ghost" onclick={() => (step = 4)}>
									Skip
								</Button>
								<Button onclick={() => (step = 4)}>
									Next
								</Button>
							</div>
						</div>
					</Card.Header>
				</Card.Root>

				<ChannelsSettings />
			</div>
		{:else}
			<Card.Root>
				<Card.Header>
					<Card.Title>Your Profile</Card.Title>
					<Card.Description>
						Tell Zenii a bit about yourself so it can give you personalized, context-aware responses.
					</Card.Description>
				</Card.Header>
				<Card.Content class="space-y-4">
					<div class="space-y-1">
						<label class="text-sm font-medium" for="onboard-name">Your Name</label>
						<Input
							id="onboard-name"
							bind:value={userName}
							placeholder="e.g., John"
						/>
					</div>
					<div class="space-y-1">
						<label class="text-sm font-medium" for="onboard-location">Location</label>
						<Input
							id="onboard-location"
							bind:value={userLocation}
							placeholder="e.g., Toronto, Canada"
						/>
					</div>
					<div class="space-y-1">
						<label class="text-sm font-medium" for="onboard-timezone">Timezone</label>
						<Input
							id="onboard-timezone"
							bind:value={userTimezone}
							placeholder="e.g., America/Toronto"
						/>
						<p class="text-xs text-muted-foreground">
							Auto-detected from your system. Edit if incorrect.
						</p>
					</div>
					{#if error}
						<p class="text-sm text-destructive">{error}</p>
					{/if}
				</Card.Content>
			</Card.Root>

			<div class="rounded-md border border-border bg-muted/50 px-4 py-3 space-y-2">
				<p class="text-xs text-muted-foreground leading-relaxed">
					Zenii uses large language models (LLMs) to generate responses and can execute system-level
					actions (shell commands, file operations) on your behalf. LLM outputs may be inaccurate,
					incomplete, or inappropriate. System actions run with your user permissions. Always review
					AI-suggested actions before confirming. Use at your own risk.
				</p>
				<label class="flex items-center gap-2 text-sm cursor-pointer">
					<input type="checkbox" bind:checked={disclaimerAccepted} class="accent-primary h-4 w-4" />
					<span>I understand and accept</span>
				</label>
			</div>

			<div class="flex justify-between">
				<Button variant="ghost" onclick={() => (step = 3)}>Back</Button>
				<Button onclick={handleFinish} disabled={saving || !disclaimerAccepted} size="lg">
					{saving ? 'Saving...' : 'Get Started'}
				</Button>
			</div>
		{/if}
	</div>
</div>
