<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Textarea } from '$lib/components/ui/textarea';
	import * as Card from '$lib/components/ui/card';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Badge } from '$lib/components/ui/badge';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import Search from '@lucide/svelte/icons/search';
	import Plus from '@lucide/svelte/icons/plus';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { memoryStore } from '$lib/stores/memory.svelte';
	import { toast } from 'svelte-sonner';
	import { onMount } from 'svelte';
	import * as m from '$lib/paraglide/messages';

	let query = $state('');
	let addOpen = $state(false);
	let editEntry = $state<{ key: string; content: string; category: string } | null>(null);
	let confirmOpen = $state(false);
	let deleteTarget = $state<string | null>(null);
	let newKey = $state('');
	let newContent = $state('');
	let newCategory = $state('core');
	let searchTimeout: ReturnType<typeof setTimeout>;

	onMount(() => {
		memoryStore.loadAll();
	});

	function handleSearch() {
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			if (query.trim()) {
				memoryStore.search(query);
			} else {
				memoryStore.loadAll();
			}
		}, 300);
	}

	async function handleAdd() {
		if (!newKey.trim() || !newContent.trim()) return;
		try {
			await memoryStore.create(newKey.trim(), newContent.trim(), newCategory);
			newKey = '';
			newContent = '';
			newCategory = 'core';
			addOpen = false;
		} catch (e) {
			toast.error(m.memory_add_error());
			console.error('handleAdd failed:', e);
		}
	}

	async function handleEdit() {
		if (!editEntry || !editEntry.content.trim()) return;
		try {
			await memoryStore.update(editEntry.key, editEntry.content, editEntry.category);
			editEntry = null;
		} catch (e) {
			toast.error(m.memory_update_error());
			console.error('handleEdit failed:', e);
		}
	}

	function handleDelete(key: string) {
		deleteTarget = key;
		confirmOpen = true;
	}

	async function confirmDelete() {
		if (!deleteTarget) return;
		try {
			await memoryStore.remove(deleteTarget);
		} catch (e) {
			toast.error(m.memory_delete_error());
			console.error('confirmDelete failed:', e);
		}
	}
</script>

<div class="max-w-3xl mx-auto space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-bold">{m.memory_page_title()}</h1>
		<Button size="sm" onclick={() => (addOpen = true)} class="gap-1">
			<Plus class="h-4 w-4" />
			{m.memory_add_button()}
		</Button>
	</div>

	<div class="relative">
		<Search class="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
		<Input
			placeholder={m.memory_search_placeholder()}
			class="pl-9"
			bind:value={query}
			oninput={handleSearch}
		/>
	</div>

	{#if memoryStore.loading}
		<div class="space-y-2">
			{#each Array(3) as _}
				<Skeleton class="h-20 w-full" />
			{/each}
		</div>
	{:else}
		{#if memoryStore.observations.length > 0}
			<div class="space-y-2">
				<h2 class="text-lg font-semibold">{m.memory_learned_observations_title()}</h2>
				<p class="text-sm text-muted-foreground">{m.memory_learned_observations_description()}</p>
				<div class="space-y-2">
					{#each memoryStore.observations as obs (obs.id)}
						<Card.Root>
							<Card.Content class="p-3">
								<div class="flex items-start justify-between gap-2">
									<div class="flex-1 min-w-0">
										<div class="flex items-center gap-2 mb-1">
											<span class="font-medium text-sm">{obs.key}</span>
											<Badge variant="outline" class="text-xs">{obs.category}</Badge>
											<span class="text-xs text-muted-foreground">
												{m.memory_confidence_label({ value: Math.round(obs.confidence * 100).toString() })}
											</span>
										</div>
										<p class="text-sm text-muted-foreground">{obs.value}</p>
									</div>
								</div>
							</Card.Content>
						</Card.Root>
					{/each}
				</div>
			</div>
		{/if}

		{#if memoryStore.entries.length > 0}
			<div class="space-y-2">
				<h2 class="text-lg font-semibold">{m.memory_stored_memories_title()}</h2>
				{#each memoryStore.entries as entry (entry.key)}
					<Card.Root>
						<Card.Content class="p-3">
							<div class="flex items-start justify-between gap-2">
								<div class="flex-1 min-w-0">
									<div class="flex items-center gap-2 mb-1">
										<span class="font-medium text-sm">{entry.key}</span>
										<Badge variant="secondary" class="text-xs">{entry.category}</Badge>
									</div>
									<p class="text-sm text-muted-foreground line-clamp-2">{entry.content}</p>
								</div>
								<div class="flex gap-1 shrink-0">
									<Button
										variant="ghost"
										size="icon"
										class="h-7 w-7"
										onclick={() => (editEntry = { key: entry.key, content: entry.content, category: entry.category })}
									>
										<Pencil class="h-3.5 w-3.5" />
									</Button>
									<Button
										variant="ghost"
										size="icon"
										class="h-7 w-7 text-destructive"
										onclick={() => handleDelete(entry.key)}
									>
										<Trash2 class="h-3.5 w-3.5" />
									</Button>
								</div>
							</div>
						</Card.Content>
					</Card.Root>
				{/each}
			</div>
		{/if}

		{#if memoryStore.entries.length === 0 && memoryStore.observations.length === 0}
			{#if query}
				<p class="text-center text-muted-foreground py-6">{m.memory_empty_no_results({ query })}</p>
			{:else}
				<p class="text-center text-muted-foreground py-6">{m.memory_empty_no_results_default()}</p>
			{/if}
		{/if}
	{/if}
</div>

<Dialog.Root bind:open={addOpen}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>{m.memory_add_dialog_title()}</Dialog.Title>
		</Dialog.Header>
		<div class="space-y-3">
			<Input placeholder={m.memory_key_placeholder()} bind:value={newKey} />
			<Textarea placeholder={m.memory_content_placeholder()} bind:value={newContent} rows={4} />
			<Input placeholder={m.memory_category_placeholder()} bind:value={newCategory} />
			<Button class="w-full" onclick={handleAdd}>{m.memory_save_button()}</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>

<Dialog.Root open={!!editEntry} onOpenChange={(open) => { if (!open) editEntry = null; }}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>{m.memory_edit_dialog_title({ key: editEntry?.key ?? '' })}</Dialog.Title>
		</Dialog.Header>
		{#if editEntry}
			<div class="space-y-3">
				<Textarea bind:value={editEntry.content} rows={4} />
				<Input placeholder={m.memory_category_placeholder()} bind:value={editEntry.category} />
				<Button class="w-full" onclick={handleEdit}>{m.memory_update_button()}</Button>
			</div>
		{/if}
	</Dialog.Content>
</Dialog.Root>

<ConfirmDialog
	bind:open={confirmOpen}
	title={m.memory_delete_confirm_title()}
	description={m.memory_delete_confirm_description()}
	onConfirm={confirmDelete}
/>
