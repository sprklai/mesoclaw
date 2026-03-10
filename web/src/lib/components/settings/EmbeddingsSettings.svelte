<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { configStore } from '$lib/stores/config.svelte';
	import { embeddingsStore } from '$lib/stores/embeddings.svelte';
	import { onMount } from 'svelte';

	let testResult = $state<{ success: boolean; dimensions?: number; latency_ms: number; error?: string } | null>(null);
	let testing = $state(false);
	let downloading = $state(false);
	let reindexing = $state(false);

	onMount(() => {
		embeddingsStore.loadStatus();
		configStore.load();
	});

	async function setProvider(provider: string) {
		try {
			await configStore.update({ embedding_provider: provider });
			await configStore.load();
			await embeddingsStore.loadStatus();
		} catch (e) {
			console.error('[Embeddings] Failed to set provider:', e);
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

	async function triggerDownload() {
		downloading = true;
		try {
			await embeddingsStore.download();
			await embeddingsStore.loadStatus();
		} catch (e) {
			console.error('[Embeddings] Download failed:', e);
		} finally {
			downloading = false;
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
					onclick={() => setProvider('none')}
				>
					None (FTS5 only)
				</Button>
				<Button
					variant={embeddingsStore.status.provider === 'local' ? 'default' : 'outline'}
					onclick={() => setProvider('local')}
				>
					Local (fastembed)
				</Button>
				<Button
					variant={embeddingsStore.status.provider === 'openai' ? 'default' : 'outline'}
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
				<Button onclick={triggerDownload} disabled={downloading}>
					{downloading ? 'Downloading...' : 'Download Model'}
				</Button>
			</Card.Content>
		</Card.Root>
	{/if}

	{#if embeddingsStore.status.provider === 'openai'}
		<Card.Root>
			<Card.Header>
				<Card.Title>OpenAI Configuration</Card.Title>
				<Card.Description>Uses your existing OpenAI API key from Settings &gt; Services</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-3">
				<p class="text-sm text-muted-foreground">
					Model: text-embedding-3-small ({embeddingsStore.status.dimensions} dimensions)
				</p>
				<Button variant="outline" onclick={() => { window.location.hash = 'services'; }}>
					Manage API Keys
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
					<Button variant="outline" size="sm" onclick={runTest} disabled={testing}>
						{testing ? 'Testing...' : 'Test Connection'}
					</Button>
					<Button
						variant="outline"
						size="sm"
						onclick={triggerReindex}
						disabled={reindexing}
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
