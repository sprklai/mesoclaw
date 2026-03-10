<script lang="ts">
	import { inboxStore } from '$lib/stores/inbox.svelte';
	import { Button } from '$lib/components/ui/button';
	import { onMount } from 'svelte';

	let threadContainer: HTMLDivElement | undefined = $state();
	let selectedConv = $derived(
		inboxStore.conversations.find((c) => c.id === inboxStore.selectedId)
	);

	onMount(() => {
		scrollToBottom();
	});

	$effect(() => {
		if (inboxStore.messages.length > 0) {
			scrollToBottom();
		}
	});

	function scrollToBottom() {
		const el = threadContainer;
		if (el) {
			requestAnimationFrame(() => {
				el.scrollTop = el.scrollHeight;
			});
		}
	}

	function formatTime(dateStr: string): string {
		try {
			return new Date(dateStr).toLocaleTimeString([], {
				hour: '2-digit',
				minute: '2-digit'
			});
		} catch {
			return '';
		}
	}
</script>

{#if !inboxStore.selectedId}
	<div class="flex h-full items-center justify-center text-muted-foreground">
		<p>Select a conversation to view messages</p>
	</div>
{:else if inboxStore.loadingMessages}
	<div class="flex h-full items-center justify-center text-muted-foreground">
		<p>Loading messages...</p>
	</div>
{:else}
	<div class="flex h-full flex-col">
		<!-- Header -->
		<div class="border-b border-border px-4 py-2">
			<h2 class="font-semibold text-sm">{selectedConv?.title ?? 'Conversation'}</h2>
			<p class="text-xs text-muted-foreground">{selectedConv?.source ?? ''} — {selectedConv?.message_count ?? 0} messages</p>
		</div>

		<!-- Load more button -->
		<div class="text-center py-1">
			<Button variant="ghost" size="sm" onclick={() => inboxStore.loadMoreMessages()}>
				Load older messages
			</Button>
		</div>

		<!-- Messages -->
		<div bind:this={threadContainer} class="flex-1 overflow-y-auto px-4 py-2 space-y-3">
			{#each inboxStore.messages as msg (msg.id)}
				<div
					class="flex flex-col {msg.role === 'assistant' ? 'items-start' : 'items-end'}"
				>
					<div
						class="max-w-[80%] rounded-lg px-3 py-2 text-sm
							{msg.role === 'assistant'
								? 'bg-muted text-foreground'
								: 'bg-primary text-primary-foreground'}"
					>
						<p class="whitespace-pre-wrap break-words">{msg.content}</p>
					</div>
					<span class="text-xs text-muted-foreground mt-0.5">
						{msg.role === 'assistant' ? 'Bot' : 'User'} — {formatTime(msg.created_at)}
					</span>
				</div>
			{/each}

			{#if inboxStore.messages.length === 0}
				<div class="text-center text-sm text-muted-foreground py-8">
					No messages in this conversation
				</div>
			{/if}
		</div>
	</div>
{/if}
