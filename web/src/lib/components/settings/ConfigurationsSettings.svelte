<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { apiGet } from '$lib/api/client';
	import { isTauri, openConfigFile } from '$lib/tauri';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import * as m from '$lib/paraglide/messages';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import ExternalLink from '@lucide/svelte/icons/external-link';

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

	async function handleOpenInEditor() {
		try {
			const backupPath = await openConfigFile();
			if (backupPath) {
				toast.success(m.settings_config_toast_opened(), {
					description: m.settings_config_toast_opened_description({ backupPath })
				});
			}
		} catch (e) {
			toast.error(m.settings_config_toast_failed(), {
				description: e instanceof Error ? e.message : String(e)
			});
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
				<Card.Title>{m.settings_config_title()}</Card.Title>
				<Card.Description>{m.settings_config_description()}</Card.Description>
			</div>
			<div class="flex items-center gap-1">
				{#if isTauri}
					<Button variant="ghost" size="icon" onclick={handleOpenInEditor} title={m.settings_config_open_in_editor_tooltip()}>
						<ExternalLink class="h-4 w-4" />
					</Button>
				{/if}
				<Button variant="ghost" size="icon" onclick={loadConfigFile} disabled={loading}>
					<RefreshCw class="h-4 w-4 {loading ? 'animate-spin' : ''}" />
				</Button>
			</div>
		</div>
	</Card.Header>
	<Card.Content class="space-y-3">
		{#if loading && !configFile}
			<Skeleton class="h-40 w-full" />
		{:else if error}
			<p class="text-sm text-destructive">{error}</p>
		{:else if configFile}
			<div class="space-y-1">
				<p class="text-sm font-medium">{m.settings_config_file_path_label()}</p>
				<code class="block rounded bg-muted px-3 py-2 text-xs break-all">{configFile.path}</code>
			</div>
			<div class="space-y-1">
				<p class="text-sm font-medium">{m.settings_config_content_label()}</p>
				<pre class="rounded bg-muted px-3 py-2 text-xs overflow-x-auto max-h-96 whitespace-pre-wrap">{configFile.content}</pre>
			</div>
		{/if}
	</Card.Content>
</Card.Root>
