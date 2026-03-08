<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { apiGet, apiPost, apiDelete } from '$lib/api/client';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	interface ServiceDef {
		id: string;
		name: string;
		type: string;
		isCustom?: boolean;
	}

	const BUILTIN_SERVICES: ServiceDef[] = [
		{ id: 'tavily', name: 'Tavily', type: 'Web Search API' },
		{ id: 'brave', name: 'Brave Search', type: 'Web Search API' },
		{ id: 'perplexity', name: 'Perplexity', type: 'AI-Powered Search API' },
		{ id: 'serpapi', name: 'SerpAPI', type: 'Search Engine Results API' },
		{ id: 'github', name: 'GitHub', type: 'Developer Platform Token' },
		{ id: 'jina', name: 'Jina AI', type: 'Reader & Embeddings API' },
		{ id: 'firecrawl', name: 'Firecrawl', type: 'Web Scraping API' },
	];

	const CUSTOM_STORAGE_KEY = 'mesoclaw_custom_services';

	let loading = $state(true);
	let configuredKeys = $state<Set<string>>(new Set());
	let expandedId = $state<string | null>(null);
	let apiKeyInputs = $state<Record<string, string>>({});
	let showKey = $state<Record<string, boolean>>({});
	let saving = $state<Record<string, boolean>>({});
	let showAddService = $state(false);
	let newService = $state({ id: '', name: '', type: '' });
	let customServices = $state<ServiceDef[]>([]);

	function credentialKey(id: string): string {
		return `api_key:${id}`;
	}

	function loadCustomServices(): ServiceDef[] {
		try {
			const raw = localStorage.getItem(CUSTOM_STORAGE_KEY);
			if (!raw) return [];
			return JSON.parse(raw);
		} catch {
			return [];
		}
	}

	function saveCustomServices(services: ServiceDef[]) {
		localStorage.setItem(CUSTOM_STORAGE_KEY, JSON.stringify(services));
	}

	function allServices(): ServiceDef[] {
		return [...BUILTIN_SERVICES, ...customServices];
	}

	async function refreshKeys() {
		try {
			const keys = await apiGet<string[]>('/credentials');
			configuredKeys = new Set(keys.filter((k) => k.startsWith('api_key:')));
		} catch {
			configuredKeys = new Set();
		}
	}

	onMount(async () => {
		customServices = loadCustomServices();
		await refreshKeys();
		loading = false;
	});

	function toggle(id: string) {
		expandedId = expandedId === id ? null : id;
	}

	function isConfigured(id: string): boolean {
		return configuredKeys.has(credentialKey(id));
	}

	async function saveKey(service: ServiceDef) {
		const value = apiKeyInputs[service.id];
		if (!value?.trim()) return;
		saving[service.id] = true;
		try {
			await apiPost('/credentials', { key: credentialKey(service.id), value: value.trim() });
			apiKeyInputs[service.id] = '';
			await refreshKeys();
		} finally {
			saving[service.id] = false;
		}
	}

	async function removeKey(service: ServiceDef) {
		saving[service.id] = true;
		try {
			await apiDelete(`/credentials/${encodeURIComponent(credentialKey(service.id))}`);
			await refreshKeys();
		} finally {
			saving[service.id] = false;
		}
	}

	function addService() {
		if (!newService.id.trim() || !newService.name.trim() || !newService.type.trim()) return;
		const existing = allServices().find((s) => s.id === newService.id.trim());
		if (existing) return;
		const entry: ServiceDef = {
			id: newService.id.trim(),
			name: newService.name.trim(),
			type: newService.type.trim(),
			isCustom: true,
		};
		customServices = [...customServices, entry];
		saveCustomServices(customServices);
		newService = { id: '', name: '', type: '' };
		showAddService = false;
	}

	function removeService(id: string) {
		customServices = customServices.filter((s) => s.id !== id);
		saveCustomServices(customServices);
		if (expandedId === id) expandedId = null;
	}
</script>

<div class="max-w-2xl mx-auto space-y-6">
	<div class="flex items-center justify-between">
		<div class="flex items-center gap-3">
			<Button variant="ghost" size="sm" onclick={() => goto('/settings')}>
				&larr; Back
			</Button>
			<h1 class="text-2xl font-bold">Services</h1>
		</div>
		<Button size="sm" variant="outline" onclick={() => (showAddService = !showAddService)}>
			{showAddService ? 'Cancel' : '+ Add Service'}
		</Button>
	</div>

	{#if showAddService}
		<Card.Root>
			<Card.Header>
				<Card.Title>Add Custom Service</Card.Title>
			</Card.Header>
			<Card.Content class="space-y-3">
				<div class="grid grid-cols-2 gap-3">
					<div class="space-y-1">
						<label class="text-sm font-medium" for="new-svc-id">Service ID</label>
						<Input id="new-svc-id" placeholder="my-service" bind:value={newService.id} />
					</div>
					<div class="space-y-1">
						<label class="text-sm font-medium" for="new-svc-name">Display Name</label>
						<Input id="new-svc-name" placeholder="My Service" bind:value={newService.name} />
					</div>
				</div>
				<div class="space-y-1">
					<label class="text-sm font-medium" for="new-svc-type">Type / Description</label>
					<Input id="new-svc-type" placeholder="Web Search API" bind:value={newService.type} />
				</div>
				<Button
					size="sm"
					disabled={!newService.id.trim() || !newService.name.trim() || !newService.type.trim()}
					onclick={addService}
				>
					Add Service
				</Button>
			</Card.Content>
		</Card.Root>
	{/if}

	{#if loading}
		<div class="space-y-2">
			<Skeleton class="h-16 w-full" />
			<Skeleton class="h-16 w-full" />
			<Skeleton class="h-16 w-full" />
		</div>
	{:else}
		<div class="space-y-2">
			{#each allServices() as service (service.id)}
				{@const configured = isConfigured(service.id)}
				<Card.Root>
					<button
						class="w-full text-left"
						onclick={() => toggle(service.id)}
					>
						<Card.Header class="py-3">
							<div class="flex items-center justify-between">
								<div class="flex items-center gap-2">
									<Card.Title class="text-base">{service.name}</Card.Title>
									<Badge variant="outline">{service.type}</Badge>
									<Badge variant={configured ? 'default' : 'secondary'}>
										{configured ? 'Configured' : 'Not configured'}
									</Badge>
								</div>
								<span class="text-xs text-muted-foreground">
									{expandedId === service.id ? '▲' : '▼'}
								</span>
							</div>
						</Card.Header>
					</button>

					{#if expandedId === service.id}
						<Card.Content class="pt-0 space-y-4">
							{#if service.isCustom}
								<div class="flex justify-end">
									<Button
										variant="destructive"
										size="sm"
										onclick={() => removeService(service.id)}
									>
										Remove Service
									</Button>
								</div>
							{/if}

							<div class="space-y-2">
								<label class="text-sm font-medium" for="key-{service.id}">API Key</label>
								<div class="flex gap-2">
									<Input
										id="key-{service.id}"
										type={showKey[service.id] ? 'text' : 'password'}
										placeholder={configured ? '••••••••  (key is set)' : 'Enter API key...'}
										bind:value={apiKeyInputs[service.id]}
									/>
									<Button
										variant="ghost"
										size="sm"
										onclick={() => (showKey[service.id] = !showKey[service.id])}
									>
										{showKey[service.id] ? 'Hide' : 'Show'}
									</Button>
								</div>
								<div class="flex gap-2">
									<Button
										size="sm"
										disabled={!apiKeyInputs[service.id]?.trim() || saving[service.id]}
										onclick={() => saveKey(service)}
									>
										{saving[service.id] ? 'Saving...' : 'Save Key'}
									</Button>
									{#if configured}
										<Button
											variant="destructive"
											size="sm"
											disabled={saving[service.id]}
											onclick={() => removeKey(service)}
										>
											Remove Key
										</Button>
									{/if}
								</div>
							</div>
						</Card.Content>
					{/if}
				</Card.Root>
			{/each}
		</div>
	{/if}
</div>
