<script lang="ts">
	import ChannelFilterTabs from '$lib/components/inbox/ChannelFilterTabs.svelte';
	import ConversationList from '$lib/components/inbox/ConversationList.svelte';
	import ConversationThread from '$lib/components/inbox/ConversationThread.svelte';
	import { inboxStore } from '$lib/stores/inbox.svelte';
	import { onMount } from 'svelte';

	onMount(() => {
		inboxStore.load();
	});
</script>

<div class="flex h-full -m-2 sm:-m-4 md:-m-6">
	<!-- Left panel: filter + conversation list -->
	<div class="w-80 shrink-0 border-r border-border flex flex-col bg-background">
		<div class="border-b border-border px-3 py-2">
			<h1 class="font-semibold text-lg">Channels</h1>
		</div>
		<ChannelFilterTabs />
		<ConversationList />
	</div>

	<!-- Right panel: message thread -->
	<div class="flex-1 min-w-0">
		<ConversationThread />
	</div>
</div>
