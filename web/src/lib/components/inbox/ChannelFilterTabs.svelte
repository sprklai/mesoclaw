<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { inboxStore } from '$lib/stores/inbox.svelte';

	const channels = [
		{ label: 'All', value: null },
		{ label: 'Telegram', value: 'telegram' },
		{ label: 'Slack', value: 'slack' },
		{ label: 'Discord', value: 'discord' }
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
