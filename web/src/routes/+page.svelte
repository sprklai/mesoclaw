<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { goto } from '$app/navigation';
	import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';

	async function handleNewChat() {
		const session = await sessionsStore.create('New Chat');
		goto(`/chat/${session.id}`);
	}
</script>

<div class="max-w-2xl mx-auto space-y-4">
	<div class="text-center space-y-2">
		<h1 class="text-3xl font-bold">Zenii</h1>
		<p class="text-muted-foreground">Your AI assistant</p>
	</div>

	<div class="flex justify-center">
		<Button size="lg" onclick={handleNewChat} class="gap-2">
			<MessageSquarePlus class="h-5 w-5" />
			New Chat
		</Button>
	</div>

	{#if sessionsStore.sessions.length > 0}
		<div class="space-y-2">
			<h2 class="text-sm font-medium text-muted-foreground">Recent Sessions</h2>
			{#each sessionsStore.sessions.slice(0, 5) as session (session.id)}
				<Card.Root
					class="cursor-pointer hover:bg-accent transition-colors"
					onclick={() => goto(`/chat/${session.id}`)}
				>
					<Card.Content class="p-3">
						<div class="flex items-center justify-between">
							<span class="font-medium truncate">{session.title}</span>
							<span class="text-xs text-muted-foreground">
								{new Date(session.created_at).toLocaleDateString()}
							</span>
						</div>
					</Card.Content>
				</Card.Root>
			{/each}
		</div>
	{/if}
</div>
