<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import * as Select from '$lib/components/ui/select';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Switch } from '$lib/components/ui/switch';
	import { configStore } from '$lib/stores/config.svelte';
	import { getBaseUrl, setBaseUrl, getToken, setToken, isValidBaseUrl } from '$lib/api/client';
	import { isTauri } from '$lib/tauri';
	import { themeStore, type Theme } from '$lib/stores/theme.svelte';
	import Sun from '@lucide/svelte/icons/sun';
	import Moon from '@lucide/svelte/icons/moon';
	import Monitor from '@lucide/svelte/icons/monitor';
	import { onMount } from 'svelte';

	const themeOptions: { value: Theme; label: string; icon: typeof Sun }[] = [
		{ value: 'light', label: 'Light', icon: Sun },
		{ value: 'dark', label: 'Dark', icon: Moon },
		{ value: 'system', label: 'System', icon: Monitor },
	];

	let baseUrl = $state(getBaseUrl());
	let token = $state(getToken() ?? '');
	let urlError = $state('');
	let userName = $state('');
	let userLocation = $state('');
	let userTimezone = $state('');
	let profileSaving = $state(false);
	let profileSaved = $state(false);

	interface NotificationRouting {
		scheduler_notification: string[];
		scheduler_job_completed: string[];
		channel_message: string[];
	}

	const DEFAULT_ROUTING: NotificationRouting = {
		scheduler_notification: ['toast', 'desktop'],
		scheduler_job_completed: ['toast', 'desktop'],
		channel_message: ['toast', 'desktop'],
	};

	function getRouting(): NotificationRouting {
		return (configStore.config.notification_routing ?? DEFAULT_ROUTING) as NotificationRouting;
	}

	function routingHasTarget(eventType: keyof NotificationRouting, target: string): boolean {
		const routing = getRouting();
		return (routing[eventType] ?? []).includes(target);
	}

	async function toggleRoutingTarget(eventType: keyof NotificationRouting, target: string, enabled: boolean) {
		const routing = { ...getRouting() };
		const current = [...(routing[eventType] ?? [])];
		if (enabled && !current.includes(target)) {
			current.push(target);
		} else if (!enabled) {
			const idx = current.indexOf(target);
			if (idx >= 0) current.splice(idx, 1);
		}
		routing[eventType] = current;
		try {
			await configStore.update({ notification_routing: routing });
			await configStore.load();
		} catch (e) {
			console.error('[Settings] Failed to update notification routing:', e);
			await configStore.load();
		}
	}

	onMount(async () => {
		await configStore.load();
		userName = String(configStore.config.user_name ?? '');
		userLocation = String(configStore.config.user_location ?? '');
		userTimezone = String(configStore.config.user_timezone ?? '');
	});

	function handleSaveConnection() {
		urlError = '';
		if (baseUrl && !isValidBaseUrl(baseUrl)) {
			urlError = 'Invalid URL. Must be a valid http:// or https:// address.';
			return;
		}
		setBaseUrl(baseUrl);
		setToken(token);
	}

	async function toggleConfig(key: string, value: boolean) {
		try {
			await configStore.update({ [key]: value });
			await configStore.load();
		} catch (e) {
			console.error(`[Settings] Failed to update ${key}:`, e);
			await configStore.load();
		}
	}

	async function saveProfile() {
		profileSaving = true;
		profileSaved = false;
		try {
			const updates: Record<string, string | null> = {
				user_name: userName.trim() || null,
	
				user_location: userLocation.trim() || null,
				user_timezone: userTimezone.trim() || null,
			};
			await configStore.update(updates as Record<string, unknown>);
			await configStore.load();
			profileSaved = true;
			setTimeout(() => { profileSaved = false; }, 2000);
		} catch (e) {
			console.error('[Settings] Failed to save profile:', e);
		} finally {
			profileSaving = false;
		}
	}

	async function updateStrategy(value: string) {
		try {
			await configStore.update({ context_strategy: value });
			await configStore.load();
		} catch (e) {
			console.error('[Settings] Failed to update context_strategy:', e);
			await configStore.load();
		}
	}

	const EVENT_TYPES: { key: keyof NotificationRouting; label: string }[] = [
		{ key: 'scheduler_notification', label: 'Scheduler Notifications' },
		{ key: 'scheduler_job_completed', label: 'Job Completed' },
		{ key: 'channel_message', label: 'Channel Messages' },
	];

	const TARGETS = [
		{ id: 'toast', label: 'Toast' },
		{ id: 'desktop', label: 'Desktop', requiresTauri: true },
	];
</script>

<Card.Root>
	<Card.Header>
		<Card.Title>Appearance</Card.Title>
		<Card.Description>Choose your preferred color theme</Card.Description>
	</Card.Header>
	<Card.Content>
		<div class="flex gap-2">
			{#each themeOptions as opt (opt.value)}
				<button
					class="flex items-center gap-2 px-4 py-2 rounded-md text-sm font-medium transition-colors
						{themeStore.theme === opt.value
							? 'bg-primary text-primary-foreground'
							: 'bg-muted text-muted-foreground hover:bg-muted/80'}"
					onclick={() => themeStore.set(opt.value)}
				>
					<opt.icon class="h-4 w-4" />
					{opt.label}
				</button>
			{/each}
		</div>
	</Card.Content>
</Card.Root>

<Card.Root>
	<Card.Header>
		<Card.Title>Connection</Card.Title>
		<Card.Description>Gateway connection settings</Card.Description>
	</Card.Header>
	<Card.Content class="space-y-3">
		<div class="space-y-1">
			<label class="text-sm font-medium" for="base-url">Gateway URL</label>
			<Input id="base-url" bind:value={baseUrl} placeholder="http://127.0.0.1:18981" />
		</div>
		<div class="space-y-1">
			<label class="text-sm font-medium" for="token">Auth Token</label>
			<Input id="token" type="password" bind:value={token} placeholder="Bearer token" />
		</div>
		{#if urlError}
			<p class="text-sm text-red-500">{urlError}</p>
		{/if}
		<Button onclick={handleSaveConnection}>Save Connection</Button>
	</Card.Content>
</Card.Root>

{#if configStore.loading}
	<Skeleton class="h-40 w-full" />
{:else if Object.keys(configStore.config).length > 0}
	<Card.Root>
		<Card.Header>
			<Card.Title>User Profile</Card.Title>
			<Card.Description>Personalize your experience and help the AI give context-aware responses</Card.Description>
		</Card.Header>
		<Card.Content class="space-y-3">
			<div class="space-y-1">
				<label class="text-sm font-medium" for="user-name">Your Name</label>
				<Input id="user-name" bind:value={userName} placeholder="e.g., John Doe" />
			</div>
			<div class="space-y-1">
				<label class="text-sm font-medium" for="user-location">Location</label>
				<Input id="user-location" bind:value={userLocation} placeholder="e.g., Toronto, Canada" />
			</div>
			<div class="space-y-1">
				<label class="text-sm font-medium" for="user-timezone">Timezone</label>
				<Input id="user-timezone" bind:value={userTimezone} placeholder="e.g., America/Toronto" />
			</div>
			<div class="flex items-center gap-2">
				<Button onclick={saveProfile} disabled={profileSaving} size="sm">
					{profileSaving ? 'Saving...' : 'Save Profile'}
				</Button>
				{#if profileSaved}
					<span class="text-sm text-green-600">Saved</span>
				{/if}
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Notifications</Card.Title>
			<Card.Description>Choose how you receive notifications for each event type</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="space-y-4">
				{#each EVENT_TYPES as eventType}
					<div class="flex items-center justify-between gap-4">
						<p class="text-sm font-medium min-w-[160px]">{eventType.label}</p>
						<div class="flex items-center gap-4">
							{#each TARGETS as target}
								{#if !target.requiresTauri || isTauri}
									<label class="flex items-center gap-1.5 text-xs cursor-pointer">
										<input
											type="checkbox"
											checked={routingHasTarget(eventType.key, target.id)}
											onchange={(e) => toggleRoutingTarget(eventType.key, target.id, e.currentTarget.checked)}
											class="accent-primary h-3.5 w-3.5"
										/>
										{target.label}
									</label>
								{/if}
							{/each}
						</div>
					</div>
				{/each}
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root>
		<Card.Header>
			<Card.Title>Agent Features</Card.Title>
			<Card.Description>Toggle context injection and self-evolution at runtime</Card.Description>
		</Card.Header>
		<Card.Content class="space-y-4">
			<div class="flex items-center justify-between gap-4">
				<div>
					<p class="text-sm font-medium">Context Injection</p>
					<p class="text-xs text-muted-foreground">Prepends identity files, soul, persona, environment details, and user profile into every agent prompt. Provides richer, more personalized responses at the cost of additional input tokens per message.</p>
				</div>
				<Switch
					checked={configStore.config.context_injection_enabled === true}
					onCheckedChange={(v) => toggleConfig('context_injection_enabled', v)}
				/>
			</div>
			<div class="flex items-center justify-between gap-4">
				<div>
					<p class="text-sm font-medium">Self-Evolution</p>
					<p class="text-xs text-muted-foreground">Agent observes your preferences and usage patterns to store learnings, refine its behavior over time, and propose skill updates. Uses additional tokens for observation analysis and memory writes.</p>
				</div>
				<Switch
					checked={configStore.config.self_evolution_enabled === true}
					onCheckedChange={(v) => toggleConfig('self_evolution_enabled', v)}
				/>
			</div>
			<div class="flex items-center justify-between">
				<div>
					<p class="text-sm font-medium">Context Strategy</p>
					<p class="text-xs text-muted-foreground">Controls how much conversation history and memory is injected</p>
				</div>
				<Select.Root
					type="single"
					value={String(configStore.config.context_strategy ?? 'balanced')}
					onValueChange={(v) => { if (v) updateStrategy(v); }}
				>
					<Select.Trigger class="w-[140px]">
						{String(configStore.config.context_strategy ?? 'balanced')}
					</Select.Trigger>
					<Select.Content>
						<Select.Item value="minimal">Minimal</Select.Item>
						<Select.Item value="balanced">Balanced</Select.Item>
						<Select.Item value="full">Full</Select.Item>
					</Select.Content>
				</Select.Root>
			</div>
			<div class="flex items-center justify-between gap-4">
				<div>
					<p class="text-sm font-medium">Compact Prompts</p>
					<p class="text-xs text-muted-foreground">Uses compact axiom-based preamble instead of verbose prose. Reduces token usage by ~60-80% while maintaining response quality.</p>
				</div>
				<Switch
					checked={configStore.config.prompt_compact_identity === true}
					onCheckedChange={(v) => toggleConfig('prompt_compact_identity', v)}
				/>
			</div>
			<div class="flex items-center justify-between gap-4">
				<div class="flex-1">
					<p class="text-sm font-medium">Max Preamble Tokens</p>
					<p class="text-xs text-muted-foreground">Token budget for system preamble. Overflow trims lowest-priority context.</p>
				</div>
				<Input
					type="number"
					class="w-[100px]"
					value={String(configStore.config.prompt_max_preamble_tokens ?? 1500)}
					onchange={async (e) => {
						const val = parseInt(e.currentTarget.value, 10);
						if (!isNaN(val) && val > 0) {
							try {
								await configStore.update({ prompt_max_preamble_tokens: val });
								await configStore.load();
							} catch (err) {
								console.error('[Settings] Failed to update prompt_max_preamble_tokens:', err);
								await configStore.load();
							}
						}
					}}
				/>
			</div>
		</Card.Content>
	</Card.Root>
{/if}
