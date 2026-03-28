<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { apiGet, apiPost, apiDelete } from '$lib/api/client';
	import { onMount } from 'svelte';
	import * as m from '$lib/paraglide/messages';

	interface ServiceDef {
		id: string;
		name: string;
		type: string;
	}

	const BUILTIN_SERVICES: ServiceDef[] = [
		{ id: 'tavily', name: m.settings_services_service_tavily(), type: m.settings_services_type_web_search() },
		{ id: 'brave', name: m.settings_services_service_brave(), type: m.settings_services_type_web_search() },
	];

	let loading = $state(true);
	let configuredKeys = $state<Set<string>>(new Set());
	let expandedId = $state<string | null>(null);
	let apiKeyInputs = $state<Record<string, string>>({});
	let showKey = $state<Record<string, boolean>>({});
	let saving = $state<Record<string, boolean>>({});

	function credentialKey(id: string): string {
		return `api_key:${id}`;
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
</script>

<div class="flex items-center justify-between mb-4">
	<h2 class="text-lg font-semibold">{m.settings_services_title()}</h2>
	<span class="text-xs text-muted-foreground">{m.settings_services_coming_soon()}</span>
</div>

{#if loading}
	<div class="space-y-2">
		<Skeleton class="h-16 w-full" />
		<Skeleton class="h-16 w-full" />
		<Skeleton class="h-16 w-full" />
	</div>
{:else}
	<div class="space-y-2">
		{#each BUILTIN_SERVICES as service (service.id)}
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
									{configured ? m.settings_services_badge_configured() : m.settings_services_badge_not_configured()}
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
						<div class="space-y-2">
							<label class="text-sm font-medium" for="key-{service.id}">{m.settings_services_api_key_label()}</label>
							<div class="flex gap-2">
								<Input
									id="key-{service.id}"
									type={showKey[service.id] ? 'text' : 'password'}
									placeholder={configured ? m.settings_services_api_key_placeholder_set() : m.settings_services_api_key_placeholder_empty()}
									bind:value={apiKeyInputs[service.id]}
								/>
								<Button
									variant="ghost"
									size="sm"
									onclick={() => (showKey[service.id] = !showKey[service.id])}
								>
									{showKey[service.id] ? m.settings_services_hide_button() : m.settings_services_show_button()}
								</Button>
							</div>
							<div class="flex gap-2">
								<Button
									size="sm"
									disabled={!apiKeyInputs[service.id]?.trim() || saving[service.id]}
									onclick={() => saveKey(service)}
								>
									{saving[service.id] ? m.settings_services_saving_button() : m.settings_services_save_key_button()}
								</Button>
								{#if configured}
									<Button
										variant="destructive"
										size="sm"
										disabled={saving[service.id]}
										onclick={() => removeKey(service)}
									>
										{m.settings_services_remove_key_button()}
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
