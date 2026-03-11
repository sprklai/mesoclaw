<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { apiGet } from '$lib/api/client';
	import { onMount } from 'svelte';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';

	let configFile = $state<{ path: string; content: string } | null>(null);
	let loading = $state(false);
	let error = $state('');

	async function loadConfigFile() {
		loading = true;
		error = '';
		try {
			configFile = await apiGet<{ path: string; content: string }>('/config/file');
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		loadConfigFile();
	});
</script>

<Card.Root>
	<Card.Header>
		<div class="flex items-center justify-between">
			<div>
				<Card.Title>Configuration File</Card.Title>
				<Card.Description>Raw TOML configuration — edit directly on disk or use settings above</Card.Description>
			</div>
			<Button variant="ghost" size="icon" onclick={loadConfigFile} disabled={loading}>
				<RefreshCw class="h-4 w-4 {loading ? 'animate-spin' : ''}" />
			</Button>
		</div>
	</Card.Header>
	<Card.Content class="space-y-3">
		{#if loading && !configFile}
			<Skeleton class="h-40 w-full" />
		{:else if error}
			<p class="text-sm text-destructive">{error}</p>
		{:else if configFile}
			<div class="space-y-1">
				<p class="text-sm font-medium">File Path</p>
				<code class="block rounded bg-muted px-3 py-2 text-xs break-all">{configFile.path}</code>
			</div>
			<div class="space-y-1">
				<p class="text-sm font-medium">Content</p>
				<pre class="rounded bg-muted px-3 py-2 text-xs overflow-x-auto max-h-96 whitespace-pre-wrap">{configFile.content}</pre>
			</div>
		{/if}
	</Card.Content>
</Card.Root>
