<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { inboxStore } from '$lib/stores/inbox.svelte';
	import * as m from '$lib/paraglide/messages';

	const channels = [
		{ label: m.channels_filter_all(), value: null },
		{ label: m.channels_filter_telegram(), value: 'telegram' },
		{ label: m.channels_filter_slack(), value: 'slack' },
		{ label: m.channels_filter_discord(), value: 'discord' }
	];
</script>

<div class="flex gap-1 p-2">
	{#each channels as ch (ch.label)}
		<Button
			variant={inboxStore.channelFilter === ch.value ? 'default' : 'ghost'}
			size="sm"
			onclick={() => inboxStore.setFilter(ch.value)}
		>
			{ch.label}
		</Button>
	{/each}
</div>
