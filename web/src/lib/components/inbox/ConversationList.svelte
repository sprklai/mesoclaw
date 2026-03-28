<script lang="ts">
	import { inboxStore } from '$lib/stores/inbox.svelte';
	import * as m from '$lib/paraglide/messages';

	function channelIcon(source: string): string {
		switch (source) {
			case 'telegram':
				return 'TG';
			case 'slack':
				return 'SL';
			case 'discord':
				return 'DC';
			default:
				return 'CH';
		}
	}

	function isUnread(id: string, updatedAt: string, messageCount: number): boolean {
		if (messageCount === 0) return false;
		const lastRead = inboxStore.lastReadTimestamps[id] ?? 0;
		return new Date(updatedAt).getTime() > lastRead;
	}

	function timeAgo(dateStr: string): string {
		const diff = Date.now() - new Date(dateStr).getTime();
		const mins = Math.floor(diff / 60000);
		if (mins < 1) return m.inbox_time_now();
		if (mins < 60) return `${mins}m`;
		const hours = Math.floor(mins / 60);
		if (hours < 24) return `${hours}h`;
		return `${Math.floor(hours / 24)}d`;
	}
</script>

<div class="flex-1 overflow-y-auto">
	{#if inboxStore.loading}
		<div class="p-4 text-sm text-muted-foreground">{m.inbox_loading()}</div>
	{:else if inboxStore.conversations.length === 0}
		<div class="p-4 text-sm text-muted-foreground">{m.inbox_no_conversations()}</div>
	{:else}
		{#each inboxStore.conversations as conv (conv.id)}
			<button
				class="w-full text-left px-3 py-2.5 border-b border-border hover:bg-accent/50 transition-colors
					{inboxStore.selectedId === conv.id ? 'bg-accent' : ''}"
				onclick={() => inboxStore.selectConversation(conv.id)}
			>
				<div class="flex items-center gap-2">
					<span
						class="inline-flex items-center justify-center h-6 w-6 rounded text-xs font-bold
							{conv.source === 'telegram' ? 'bg-blue-500/20 text-blue-400' : ''}
							{conv.source === 'slack' ? 'bg-purple-500/20 text-purple-400' : ''}
							{conv.source === 'discord' ? 'bg-indigo-500/20 text-indigo-400' : ''}"
					>
						{channelIcon(conv.source)}
					</span>
					<span class="flex-1 truncate text-sm font-medium">{conv.title}</span>
					{#if isUnread(conv.id, conv.updated_at, conv.message_count)}
						<span class="h-2 w-2 rounded-full bg-primary shrink-0"></span>
					{/if}
				</div>
				<div class="flex items-center gap-2 mt-0.5 text-xs text-muted-foreground">
					<span>{m.inbox_message_count({ count: conv.message_count.toString() })}</span>
					<span class="ml-auto">{timeAgo(conv.updated_at)}</span>
				</div>
			</button>
		{/each}
	{/if}
</div>
