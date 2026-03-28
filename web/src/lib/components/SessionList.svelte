<script lang="ts">
	import * as m from '$lib/paraglide/messages';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Button } from '$lib/components/ui/button';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Pencil from '@lucide/svelte/icons/pencil';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { messagesStore } from '$lib/stores/messages.svelte';
	import { toast } from 'svelte-sonner';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';

	let editingId = $state<string | null>(null);
	let editTitle = $state('');
	let confirmOpen = $state(false);
	let deleteTarget = $state<string | null>(null);
	let editInputRef = $state<HTMLInputElement | null>(null);
	let creating = $state(false);

	$effect(() => {
		if (editInputRef) {
			editInputRef.focus();
		}
	});

	async function handleRefresh() {
		await sessionsStore.load();
		if (page.params.id) {
			await messagesStore.load(page.params.id);
		}
	}

	async function handleNew() {
		if (creating) return;
		creating = true;
		try {
			const session = await sessionsStore.create(m.chat_new());
			goto(`/chat/${session.id}`);
		} catch (e) {
			toast.error(m.chat_session_create_error());
			console.error('handleNew failed:', e);
		} finally {
			creating = false;
		}
	}

	function handleDelete(e: Event, id: string) {
		e.stopPropagation();
		deleteTarget = id;
		confirmOpen = true;
	}

	async function confirmDelete() {
		if (!deleteTarget) return;
		const id = deleteTarget;
		try {
			await sessionsStore.remove(id);
			if (page.params.id === id) {
				goto('/');
			}
		} catch (e) {
			toast.error(m.session_delete_error());
			console.error('confirmDelete failed:', e);
		}
	}

	function startEdit(e: Event, id: string, title: string) {
		e.stopPropagation();
		editingId = id;
		editTitle = title;
	}

	async function saveEdit() {
		if (editingId && editTitle.trim()) {
			try {
				await sessionsStore.update(editingId, editTitle.trim());
			} catch (e) {
				toast.error(m.session_rename_error());
				console.error('saveEdit failed:', e);
			}
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
		<span>{m.session_chats_label()}</span>
		<div class="flex items-center gap-0.5">
			<Button variant="ghost" size="icon" class="h-5 w-5" onclick={handleRefresh} disabled={sessionsStore.loading}>
				<RefreshCw class="h-3.5 w-3.5 {sessionsStore.loading ? 'animate-spin' : ''}" />
			</Button>
			<Button variant="ghost" size="icon" class="h-5 w-5" onclick={handleNew} disabled={creating}>
				<MessageSquarePlus class="h-3.5 w-3.5" />
			</Button>
		</div>
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
								bind:this={editInputRef}
								onkeydown={handleEditKeydown}
								onblur={saveEdit}
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

<ConfirmDialog
	bind:open={confirmOpen}
	title={m.session_delete_confirm_title()}
	description={m.session_delete_confirm_description()}
	onConfirm={confirmDelete}
/>
