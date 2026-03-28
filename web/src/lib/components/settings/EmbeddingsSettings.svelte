<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { configStore } from '$lib/stores/config.svelte';
	import { embeddingsStore } from '$lib/stores/embeddings.svelte';
	import { providersStore } from '$lib/stores/providers.svelte';
	import { onMount, onDestroy } from 'svelte';
	import * as m from '$lib/paraglide/messages';

	let testResult = $state<{ success: boolean; dimensions?: number; latency_ms: number; error?: string } | null>(null);
	let testing = $state(false);
	let reindexing = $state(false);
	let switching = $state(false);
	let pollInterval: ReturnType<typeof setInterval> | undefined;

	let hasOpenAiKey = $derived(
		providersStore.providers.some((p) => p.id === 'openai' && p.has_api_key)
	);

	function startPolling() {
		stopPolling();
		if (embeddingsStore.status.provider === 'local' && !embeddingsStore.status.model_available) {
			pollInterval = setInterval(async () => {
				await embeddingsStore.refreshStatus();
				if (embeddingsStore.status.model_available) {
					stopPolling();
				}
			}, 2000);
		}
	}

	function stopPolling() {
		if (pollInterval !== undefined) {
			clearInterval(pollInterval);
			pollInterval = undefined;
		}
	}

	onMount(async () => {
		await embeddingsStore.loadStatus();
		await configStore.load();
		await providersStore.load();

		// Auto-migrate: local embeddings is disabled (experimental), switch to none
		if (embeddingsStore.status.provider === 'local') {
			await setProvider('none');
			return;
		}

		startPolling();
	});

	onDestroy(() => {
		stopPolling();
	});

	async function setProvider(provider: string) {
		if (provider === embeddingsStore.status.provider || switching) return;
		switching = true;
		try {
			await configStore.update({ embedding_provider: provider });
			await configStore.load();
			await embeddingsStore.refreshStatus();
			startPolling();
		} catch (e) {
			console.error('[Embeddings] Failed to set provider:', e);
		} finally {
			switching = false;
		}
	}

	async function runTest() {
		testing = true;
		try {
			testResult = await embeddingsStore.test();
		} catch (e) {
			testResult = { success: false, latency_ms: 0, error: String(e) };
		} finally {
			testing = false;
		}
	}

	async function triggerReindex() {
		reindexing = true;
		try {
			await embeddingsStore.reindex();
		} catch (e) {
			console.error('[Embeddings] Reindex failed:', e);
		} finally {
			reindexing = false;
		}
	}
</script>

{#if embeddingsStore.loading}
	<Skeleton class="h-40 w-full" />
{:else}
	<Card.Root>
		<Card.Header>
			<Card.Title>{m.settings_embeddings_provider_title()} <span class="text-xs font-normal text-muted-foreground">{m.settings_embeddings_provider_experimental()}</span></Card.Title>
			<Card.Description>{m.settings_embeddings_provider_description()}</Card.Description>
		</Card.Header>
		<Card.Content class="space-y-3">
			<div class="flex gap-2">
				<Button
					variant={embeddingsStore.status.provider === 'none' ? 'default' : 'outline'}
					disabled={switching}
					onclick={() => setProvider('none')}
				>
					{m.settings_embeddings_provider_none()}
				</Button>
				<Button
					variant="outline"
					disabled={true}
					class="opacity-50 cursor-not-allowed"
				>
					{m.settings_embeddings_provider_local()} <span class="text-xs ml-1">{m.settings_embeddings_provider_local_experimental()}</span>
				</Button>
				<Button
					variant={embeddingsStore.status.provider === 'openai' ? 'default' : 'outline'}
					disabled={switching}
					onclick={() => setProvider('openai')}
				>
					{m.settings_embeddings_provider_openai()}
				</Button>
			</div>

			{#if embeddingsStore.status.provider === 'none'}
				<p class="text-sm text-muted-foreground">
					{m.settings_embeddings_disabled_message()}
				</p>
			{/if}
		</Card.Content>
	</Card.Root>

	{#if embeddingsStore.status.provider === 'local'}
		<Card.Root>
			<Card.Header>
				<Card.Title>{m.settings_embeddings_local_model_title()}</Card.Title>
				<Card.Description>{m.settings_embeddings_local_model_description()}</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-3">
				<div class="space-y-1">
					<p class="text-sm font-medium">{m.settings_embeddings_local_model_prefix()} {embeddingsStore.status.model || 'bge-small-en-v1.5'}</p>
					<p class="text-xs text-muted-foreground">{m.settings_embeddings_local_dimensions_prefix()} {embeddingsStore.status.dimensions}</p>
				</div>
				{#if embeddingsStore.status.model_available}
					<p class="text-sm text-green-600 dark:text-green-400 font-medium">{m.settings_embeddings_local_available()}</p>
				{:else}
					<div class="flex items-center gap-2">
						<svg class="h-4 w-4 animate-spin text-muted-foreground" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
							<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
							<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
						</svg>
						<p class="text-sm text-muted-foreground">{m.settings_embeddings_local_downloading()}</p>
					</div>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}

	{#if embeddingsStore.status.provider === 'openai'}
		<Card.Root>
			<Card.Header>
				<Card.Title>{m.settings_embeddings_openai_title()}</Card.Title>
				<Card.Description>{m.settings_embeddings_openai_description()}</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-3">
				{#if hasOpenAiKey}
					<p class="text-sm text-muted-foreground">
						{m.settings_embeddings_openai_model_info({ dimensions: String(embeddingsStore.status.dimensions) })}
					</p>
				{:else}
					<p class="text-sm text-red-600 dark:text-red-400">
						{m.settings_embeddings_openai_key_required()}
					</p>
				{/if}
				<Button variant="outline" onclick={() => { window.location.hash = 'providers'; }}>
					{m.settings_embeddings_manage_providers_button()}
				</Button>
			</Card.Content>
		</Card.Root>
	{/if}

	{#if embeddingsStore.status.provider !== 'none'}
		<Card.Root>
			<Card.Header>
				<Card.Title>{m.settings_embeddings_status_title()}</Card.Title>
			</Card.Header>
			<Card.Content class="space-y-3">
				<div class="grid grid-cols-2 gap-2 text-sm">
					<span class="text-muted-foreground">{m.settings_embeddings_status_provider()}</span>
					<span>{embeddingsStore.status.provider}</span>
					<span class="text-muted-foreground">{m.settings_embeddings_status_model()}</span>
					<span>{embeddingsStore.status.model || '-'}</span>
					<span class="text-muted-foreground">{m.settings_embeddings_status_dimensions()}</span>
					<span>{embeddingsStore.status.dimensions}</span>
				</div>

				<div class="flex gap-2 pt-2">
					<Button variant="outline" size="sm" onclick={runTest} disabled={testing || !embeddingsStore.status.model_available}>
						{testing ? m.settings_embeddings_testing_button() : m.settings_embeddings_test_connection_button()}
					</Button>
					<Button
						variant="outline"
						size="sm"
						onclick={triggerReindex}
						disabled={reindexing || !embeddingsStore.status.model_available}
					>
						{reindexing ? m.settings_embeddings_reindexing_button() : m.settings_embeddings_reindex_button()}
					</Button>
				</div>

				{#if testResult}
					<div class="text-sm mt-2 p-2 rounded bg-muted">
						{#if testResult.success}
							<p class="text-green-600 dark:text-green-400">{m.settings_embeddings_test_passed({ dimensions: String(testResult.dimensions ?? 0), latency_ms: String(testResult.latency_ms) })}</p>
						{:else}
							<p class="text-red-600 dark:text-red-400">{m.settings_embeddings_test_failed({ error: testResult.error ?? '' })}</p>
						{/if}
					</div>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
{/if}
