<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import * as Select from '$lib/components/ui/select';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { Switch } from '$lib/components/ui/switch';
	import { configStore } from '$lib/stores/config.svelte';
	import { getBaseUrl, setBaseUrl, getToken, setToken } from '$lib/api/client';
	import { onMount } from 'svelte';

	let baseUrl = $state(getBaseUrl());
	let token = $state(getToken() ?? '');

	onMount(() => {
		configStore.load();
	});

	function handleSaveConnection() {
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

	async function updateStrategy(value: string) {
		try {
			await configStore.update({ context_strategy: value });
			await configStore.load();
		} catch (e) {
			console.error('[Settings] Failed to update context_strategy:', e);
			await configStore.load();
		}
	}
</script>

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
		<Button onclick={handleSaveConnection}>Save Connection</Button>
	</Card.Content>
</Card.Root>

{#if configStore.loading}
	<Skeleton class="h-40 w-full" />
{:else if Object.keys(configStore.config).length > 0}
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
		</Card.Content>
	</Card.Root>
{/if}
