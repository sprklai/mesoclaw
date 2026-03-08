<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { configStore } from '$lib/stores/config.svelte';
	import { getBaseUrl, setBaseUrl, getToken, setToken } from '$lib/api/client';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	let baseUrl = $state(getBaseUrl());
	let token = $state(getToken() ?? '');

	onMount(() => {
		configStore.load();
	});

	function handleSaveConnection() {
		setBaseUrl(baseUrl);
		setToken(token);
	}
</script>

<div class="max-w-2xl mx-auto space-y-6">
	<h1 class="text-2xl font-bold">Settings</h1>

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

	<div class="flex gap-2">
		<Button variant="outline" onclick={() => goto('/settings/providers')}>
			Providers
		</Button>
		<Button variant="outline" onclick={() => goto('/settings/services')}>
			Services
		</Button>
		<Button variant="outline" onclick={() => goto('/settings/persona')}>
			Persona & Skills
		</Button>
		<Button variant="outline" onclick={() => goto('/settings/channels')}>
			Channels
		</Button>
	</div>

	{#if configStore.loading}
		<Skeleton class="h-40 w-full" />
	{:else if Object.keys(configStore.config).length > 0}
		<Card.Root>
			<Card.Header>
				<Card.Title>Current Configuration</Card.Title>
			</Card.Header>
			<Card.Content>
				<pre class="text-xs bg-muted rounded-lg p-3 overflow-auto max-h-80">{JSON.stringify(configStore.config, null, 2)}</pre>
			</Card.Content>
		</Card.Root>
	{/if}
</div>
