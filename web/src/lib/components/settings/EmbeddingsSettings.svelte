<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { configStore } from '$lib/stores/config.svelte';
	import { embeddingsStore } from '$lib/stores/embeddings.svelte';
	import { providersStore } from '$lib/stores/providers.svelte';
	import { onMount, onDestroy } from 'svelte';

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
			<Card.Title>Provider Selection <span class="text-xs font-normal text-muted-foreground">(Experimental)</span></Card.Title>
			<Card.Description>Choose how semantic embeddings are generated for memory search</Card.Description>
		</Card.Header>
		<Card.Content class="space-y-3">
			<div class="flex gap-2">
				<Button
					variant={embeddingsStore.status.provider === 'none' ? 'default' : 'outline'}
					disabled={switching}
					onclick={() => setProvider('none')}
				>
					None (FTS5 only)
				</Button>
				<Button
					variant="outline"
					disabled={true}
					class="opacity-50 cursor-not-allowed"
				>
					Local (fastembed) <span class="text-xs ml-1">(Experimental)</span>
				</Button>
				<Button
					variant={embeddingsStore.status.provider === 'openai' ? 'default' : 'outline'}
					disabled={switching}
					onclick={() => setProvider('openai')}
				>
					OpenAI API
				</Button>
			</div>

			{#if embeddingsStore.status.provider === 'none'}
				<p class="text-sm text-muted-foreground">
					Semantic search is disabled. Select a provider to enable.
				</p>
			{/if}
		</Card.Content>
	</Card.Root>

	{#if embeddingsStore.status.provider === 'local'}
		<Card.Root>
			<Card.Header>
				<Card.Title>Local Model</Card.Title>
				<Card.Description>Manage the local embedding model</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-3">
				<div class="space-y-1">
					<p class="text-sm font-medium">Model: {embeddingsStore.status.model || 'bge-small-en-v1.5'}</p>
					<p class="text-xs text-muted-foreground">Dimensions: {embeddingsStore.status.dimensions}</p>
				</div>
				{#if embeddingsStore.status.model_available}
					<p class="text-sm text-green-600 dark:text-green-400 font-medium">Available</p>
				{:else}
					<div class="flex items-center gap-2">
						<svg class="h-4 w-4 animate-spin text-muted-foreground" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
							<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
							<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
						</svg>
						<p class="text-sm text-muted-foreground">Downloading model...</p>
					</div>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}

	{#if embeddingsStore.status.provider === 'openai'}
		<Card.Root>
			<Card.Header>
				<Card.Title>OpenAI Configuration</Card.Title>
				<Card.Description>Uses your OpenAI API key from Settings &gt; Providers</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-3">
				{#if hasOpenAiKey}
					<p class="text-sm text-muted-foreground">
						Model: text-embedding-3-small ({embeddingsStore.status.dimensions} dimensions)
					</p>
				{:else}
					<p class="text-sm text-red-600 dark:text-red-400">
						OpenAI API key is required. Add it in Settings &gt; Providers.
					</p>
				{/if}
				<Button variant="outline" onclick={() => { window.location.hash = 'providers'; }}>
					Manage Providers
				</Button>
			</Card.Content>
		</Card.Root>
	{/if}

	{#if embeddingsStore.status.provider !== 'none'}
		<Card.Root>
			<Card.Header>
				<Card.Title>Status</Card.Title>
			</Card.Header>
			<Card.Content class="space-y-3">
				<div class="grid grid-cols-2 gap-2 text-sm">
					<span class="text-muted-foreground">Provider:</span>
					<span>{embeddingsStore.status.provider}</span>
					<span class="text-muted-foreground">Model:</span>
					<span>{embeddingsStore.status.model || '-'}</span>
					<span class="text-muted-foreground">Dimensions:</span>
					<span>{embeddingsStore.status.dimensions}</span>
				</div>

				<div class="flex gap-2 pt-2">
					<Button variant="outline" size="sm" onclick={runTest} disabled={testing || !embeddingsStore.status.model_available}>
						{testing ? 'Testing...' : 'Test Connection'}
					</Button>
					<Button
						variant="outline"
						size="sm"
						onclick={triggerReindex}
						disabled={reindexing || !embeddingsStore.status.model_available}
					>
						{reindexing ? 'Re-indexing...' : 'Re-index All Memories'}
					</Button>
				</div>

				{#if testResult}
					<div class="text-sm mt-2 p-2 rounded bg-muted">
						{#if testResult.success}
							<p class="text-green-600 dark:text-green-400">Test passed ({testResult.dimensions} dims, {testResult.latency_ms}ms)</p>
						{:else}
							<p class="text-red-600 dark:text-red-400">Test failed: {testResult.error}</p>
						{/if}
					</div>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
{/if}
