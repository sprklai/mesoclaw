<script lang="ts">
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Button } from '$lib/components/ui/button';
	import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Pencil from '@lucide/svelte/icons/pencil';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	let editingId = $state<string | null>(null);
	let editTitle = $state('');

	async function handleNew() {
		const session = await sessionsStore.create('New Chat');
		goto(`/chat/${session.id}`);
	}

	async function handleDelete(e: Event, id: string) {
		e.stopPropagation();
		await sessionsStore.remove(id);
		if (page.params.id === id) {
			goto('/');
		}
	}

	function startEdit(e: Event, id: string, title: string) {
		e.stopPropagation();
		editingId = id;
		editTitle = title;
	}

	async function saveEdit() {
		if (editingId && editTitle.trim()) {
			await sessionsStore.update(editingId, editTitle.trim());
		}
		editingId = null;
	}

	function cancelEdit() {
		editingId = null;
	}

	function handleEditKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			e.preventDefault();
			saveEdit();
		} else if (e.key === 'Escape') {
			cancelEdit();
		}
	}
</script>

<Sidebar.Group>
	<Sidebar.GroupLabel class="flex items-center justify-between">
		<span>Chats</span>
		<Button variant="ghost" size="icon" class="h-5 w-5" onclick={handleNew}>
			<MessageSquarePlus class="h-3.5 w-3.5" />
		</Button>
	</Sidebar.GroupLabel>
	<Sidebar.GroupContent>
		<Sidebar.Menu>
			{#each sessionsStore.sessions as session (session.id)}
				<Sidebar.MenuItem>
					{#if editingId === session.id}
						<div class="flex w-full items-center gap-1 px-2 py-1">
							<input
								class="flex-1 rounded border bg-background text-foreground px-1 py-0.5 text-sm focus:outline-none focus:ring-1 focus:ring-ring"
								bind:value={editTitle}
								onkeydown={handleEditKeydown}
								onblur={saveEdit}
								autofocus
							/>
						</div>
					{:else}
						<Sidebar.MenuButton
							isActive={page.params.id === session.id}
							onclick={() => goto(`/chat/${session.id}`)}
						>
							<MessageSquare class="h-4 w-4" />
							<span
								class="truncate"
								role="button"
								tabindex="-1"
								ondblclick={(e: MouseEvent) => startEdit(e, session.id, session.title)}
							>
								{session.title}
							</span>
						</Sidebar.MenuButton>
						<div class="absolute end-1 top-1.5 flex items-center gap-0.5 opacity-0 group-focus-within/menu-item:opacity-100 group-hover/menu-item:opacity-100 transition-opacity">
							<button
								class="flex h-5 w-5 items-center justify-center rounded-md text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
								onclick={(e: Event) => startEdit(e, session.id, session.title)}
							>
								<Pencil class="h-3 w-3" />
							</button>
							<button
								class="flex h-5 w-5 items-center justify-center rounded-md text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground"
								onclick={(e: Event) => handleDelete(e, session.id)}
							>
								<Trash2 class="h-3 w-3" />
							</button>
						</div>
					{/if}
				</Sidebar.MenuItem>
			{/each}
		</Sidebar.Menu>
	</Sidebar.GroupContent>
</Sidebar.Group>
