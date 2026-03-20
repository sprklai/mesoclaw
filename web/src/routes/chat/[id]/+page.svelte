<script lang="ts">
	import ChatView from '$lib/components/ChatView.svelte';
	import { messagesStore } from '$lib/stores/messages.svelte';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { page } from '$app/state';
	import { Skeleton } from '$lib/components/ui/skeleton';

	let sessionId = $derived(page.params.id!);

	$effect(() => {
		if (sessionId) {
			// P0.1: Skip clearing if we're actively streaming this same session
			// (e.g., after first-message goto navigates here)
			const isStreamingThisSession =
				messagesStore.streaming && messagesStore.activeStreamSessionId === sessionId;

			if (!isStreamingThisSession) {
				messagesStore.clear();
				messagesStore.load(sessionId);
			}
			sessionsStore.get(sessionId);
		}
	});
</script>

{#if messagesStore.loading}
	<div class="space-y-4 p-4">
		<Skeleton class="h-12 w-3/4" />
		<Skeleton class="h-12 w-1/2 ml-auto" />
		<Skeleton class="h-12 w-2/3" />
	</div>
{:else}
	<ChatView {sessionId} />
{/if}
