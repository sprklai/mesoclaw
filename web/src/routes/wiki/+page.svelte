<script lang="ts">
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Textarea } from '$lib/components/ui/textarea';
	import { Badge } from '$lib/components/ui/badge';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Streamdown } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import { SvelteFlow, Controls, Background } from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';
	import { wikiStore, type WikiPage } from '$lib/stores/wiki.svelte';
	import { themeStore } from '$lib/stores/theme.svelte';
	import { shikiThemes } from '$lib/components/ai-elements/code/shiki';
	import * as m from '$lib/paraglide/messages';
	import Search from '@lucide/svelte/icons/search';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import GitFork from '@lucide/svelte/icons/git-fork';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Upload from '@lucide/svelte/icons/upload';
	import FileUp from '@lucide/svelte/icons/file-up';
	import X from '@lucide/svelte/icons/x';
	import Tag from '@lucide/svelte/icons/tag';
	import Link from '@lucide/svelte/icons/link';
	import Loader2 from '@lucide/svelte/icons/loader-2';

	const CATEGORIES = ['all', 'concepts', 'entities', 'topics', 'comparisons', 'queries'] as const;
	type Category = (typeof CATEGORIES)[number];

	let query = $state('');
	let searchTimeout: ReturnType<typeof setTimeout>;
	let activeCategory = $state<Category>('all');
	let selectedPage = $state<WikiPage | null>(null);
	let pageLoading = $state(false);
	let showGraph = $state(false);
	let ingestOpen = $state(false);
	let ingestFilename = $state('');
	let ingestContent = $state('');
	let ingesting = $state(false);
	let fileInput: HTMLInputElement | undefined = $state();

	// SvelteFlow nodes/edges derived from wiki graph
	let flowNodes = $derived.by(() => {
		const g = wikiStore.graph;
		if (!g) return [];
		return g.nodes.map((n, i) => ({
			id: n.id,
			type: 'default',
			position: { x: (i % 5) * 200, y: Math.floor(i / 5) * 120 },
			data: { label: n.label },
			style: `font-size: 12px; border-radius: 6px; padding: 6px 10px;`
		}));
	});

	let flowEdges = $derived.by(() => {
		const g = wikiStore.graph;
		if (!g) return [];
		return g.edges.map((e) => ({
			id: `${e.from}-${e.to}`,
			source: e.from,
			target: e.to,
			type: 'smoothstep'
		}));
	});

	let filteredPages = $derived.by(() => {
		if (activeCategory === 'all') return wikiStore.pages;
		return wikiStore.pages.filter((p) => p.page_type === activeCategory.slice(0, -1)); // remove trailing 's' for type matching
	});

	let currentTheme = $derived(themeStore.isDark ? 'github-dark-default' : 'github-light-default');

	onMount(() => {
		wikiStore.load();
	});

	function handleSearch() {
		clearTimeout(searchTimeout);
		searchTimeout = setTimeout(() => {
			if (query.trim()) {
				wikiStore.search(query.trim());
			} else {
				wikiStore.load();
			}
			selectedPage = null;
		}, 300);
	}

	async function handleSelectPage(slug: string) {
		if (selectedPage?.slug === slug) return;
		pageLoading = true;
		try {
			selectedPage = await wikiStore.getPage(slug);
		} finally {
			pageLoading = false;
		}
	}

	async function handleSync() {
		try {
			const count = await wikiStore.sync();
			toast.success(m.wiki_sync_success({ count: count.toString() }));
		} catch {
			toast.error(m.wiki_sync_error());
		}
	}

	async function handleToggleGraph() {
		showGraph = !showGraph;
		if (showGraph && !wikiStore.graph) {
			await wikiStore.loadGraph();
		}
	}

	async function loadFile(file: File) {
		ingestFilename = file.name;
		ingestContent = await file.text();
	}

	async function handleFileSelect(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		await loadFile(file);
	}

	function handleDropZoneDragOver(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
	}

	async function handleDropZoneDrop(e: DragEvent) {
		e.preventDefault();
		const file = e.dataTransfer?.files?.[0];
		if (!file) return;
		await loadFile(file);
	}

	// Whole-page drag-and-drop: open ingest dialog and pre-fill file
	let pageDragOver = $state(false);

	function handlePageDragOver(e: DragEvent) {
		if (!e.dataTransfer?.types.includes('Files')) return;
		e.preventDefault();
		pageDragOver = true;
	}

	function handlePageDragLeave(e: DragEvent) {
		// Only clear if leaving the outermost element
		if ((e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) return;
		pageDragOver = false;
	}

	async function handlePageDrop(e: DragEvent) {
		e.preventDefault();
		pageDragOver = false;
		const file = e.dataTransfer?.files?.[0];
		if (!file) return;
		await loadFile(file);
		ingestOpen = true;
	}

	async function handleIngest() {
		if (!ingestFilename.trim()) {
			toast.error('Filename is required');
			return;
		}
		if (!ingestContent.trim()) {
			toast.error('Content is required');
			return;
		}
		ingesting = true;
		try {
			const res = await wikiStore.ingest(ingestFilename.trim(), ingestContent.trim());
			toast.success(m.wiki_ingest_success({ slug: res.slug }));
			ingestOpen = false;
			ingestFilename = '';
			ingestContent = '';
			await wikiStore.load();
		} catch (e) {
			const msg = e instanceof Error ? e.message : m.wiki_ingest_error();
			toast.error(msg);
			console.error('[wiki] ingest failed:', e);
		} finally {
			ingesting = false;
		}
	}

	function handleCategoryClick(cat: Category) {
		activeCategory = cat;
		selectedPage = null;
	}

	function handleWikilinkClick(slug: string) {
		handleSelectPage(slug);
	}

	function typeColor(type: string): string {
		switch (type) {
			case 'concept': return 'bg-blue-500/10 text-blue-600 dark:text-blue-400';
			case 'entity': return 'bg-green-500/10 text-green-600 dark:text-green-400';
			case 'topic': return 'bg-orange-500/10 text-orange-600 dark:text-orange-400';
			case 'comparison': return 'bg-purple-500/10 text-purple-600 dark:text-purple-400';
			case 'query': return 'bg-pink-500/10 text-pink-600 dark:text-pink-400';
			default: return 'bg-muted text-muted-foreground';
		}
	}
</script>

<div
	class="flex h-full flex-col gap-0 overflow-hidden {pageDragOver ? 'outline outline-2 outline-primary/50' : ''}"
	ondragover={handlePageDragOver}
	ondragleave={handlePageDragLeave}
	ondrop={handlePageDrop}
	role="region"
	aria-label="Wiki"
>
	<!-- Header toolbar -->
	<div class="flex shrink-0 items-center justify-between border-b px-4 py-3">
		<div class="flex items-center gap-2">
			<BookOpen class="h-5 w-5 text-muted-foreground" />
			<h1 class="text-xl font-semibold">{m.wiki_page_title()}</h1>
			{#if !wikiStore.loading}
				<span class="text-sm text-muted-foreground">
					({wikiStore.pages.length})
				</span>
			{/if}
		</div>
		<div class="flex items-center gap-2">
			<Button
				variant={showGraph ? 'default' : 'outline'}
				size="sm"
				class="gap-1.5"
				onclick={handleToggleGraph}
			>
				<GitFork class="h-3.5 w-3.5" />
				{m.wiki_graph_toggle()}
			</Button>
			<Button
				variant="outline"
				size="sm"
				class="gap-1.5"
				onclick={() => (ingestOpen = true)}
			>
				<FileUp class="h-3.5 w-3.5" />
				{m.wiki_ingest_button()}
			</Button>
			<Button
				variant="outline"
				size="sm"
				class="gap-1.5"
				onclick={handleSync}
				disabled={wikiStore.syncing}
			>
				{#if wikiStore.syncing}
					<Loader2 class="h-3.5 w-3.5 animate-spin" />
					{m.wiki_syncing()}
				{:else}
					<RefreshCw class="h-3.5 w-3.5" />
					{m.wiki_sync_button()}
				{/if}
			</Button>
		</div>
	</div>

	<!-- Main content -->
	<div class="flex min-h-0 flex-1">
		<!-- Left panel: search + category tabs + page list -->
		<div class="flex w-64 shrink-0 flex-col border-r">
			<!-- Search -->
			<div class="border-b p-3">
				<div class="relative">
					<Search class="absolute left-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-muted-foreground" />
					<Input
						placeholder={m.wiki_search_placeholder()}
						class="pl-8 text-sm"
						bind:value={query}
						oninput={handleSearch}
					/>
				</div>
			</div>

			<!-- Category tabs -->
			<div class="flex flex-wrap gap-1 border-b p-2">
				{#each CATEGORIES as cat}
					<button
						class="rounded px-2 py-0.5 text-xs font-medium transition-colors {activeCategory === cat
							? 'bg-primary text-primary-foreground'
							: 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
						onclick={() => handleCategoryClick(cat)}
					>
						{m[`wiki_tab_${cat}`]()}
					</button>
				{/each}
			</div>

			<!-- Page list -->
			<div class="flex-1 overflow-y-auto">
				{#if wikiStore.loading}
					<div class="space-y-1 p-2">
						{#each Array(6) as _}
							<Skeleton class="h-10 w-full" />
						{/each}
					</div>
				{:else if filteredPages.length === 0}
					<p class="p-4 text-center text-sm text-muted-foreground">
						{query ? m.wiki_empty_no_results({ query }) : m.wiki_empty_no_pages()}
					</p>
				{:else}
					<div class="space-y-0.5 p-2">
						{#each filteredPages as page (page.slug)}
							<button
								class="w-full rounded-md px-2.5 py-2 text-left transition-colors {selectedPage?.slug === page.slug
									? 'bg-accent text-accent-foreground'
									: 'hover:bg-muted'}"
								onclick={() => handleSelectPage(page.slug)}
							>
								<div class="truncate text-sm font-medium">{page.title}</div>
								<div class="mt-0.5 flex items-center gap-1.5">
									<span class="rounded px-1 py-0 text-[10px] font-medium {typeColor(page.page_type)}">
										{page.page_type}
									</span>
									{#if page.updated}
										<span class="text-[10px] text-muted-foreground">{page.updated}</span>
									{/if}
								</div>
							</button>
						{/each}
					</div>
				{/if}
			</div>
		</div>

		<!-- Right panel: page reader or graph -->
		<div class="flex min-w-0 flex-1 flex-col overflow-hidden">
			{#if showGraph}
				<!-- Graph view -->
				<div class="flex-1">
					{#if wikiStore.graphLoading}
						<div class="flex h-full items-center justify-center">
							<Loader2 class="h-8 w-8 animate-spin text-muted-foreground" />
						</div>
					{:else if flowNodes.length === 0}
						<div class="flex h-full items-center justify-center">
							<p class="text-sm text-muted-foreground">{m.wiki_graph_empty()}</p>
						</div>
					{:else}
						<SvelteFlow
							nodes={flowNodes}
							edges={flowEdges}
							fitView
							colorMode={themeStore.isDark ? 'dark' : 'light'}
							class="h-full"
						>
							<Controls />
							<Background />
						</SvelteFlow>
					{/if}
				</div>
			{:else if pageLoading}
				<div class="flex-1 space-y-3 overflow-y-auto p-6">
					<Skeleton class="h-8 w-48" />
					<Skeleton class="h-4 w-32" />
					<Skeleton class="h-20 w-full" />
					<Skeleton class="h-4 w-full" />
					<Skeleton class="h-4 w-3/4" />
				</div>
			{:else if selectedPage}
				<!-- Page reader -->
				<div class="flex-1 overflow-y-auto p-6">
					<!-- Title + meta -->
					<div class="mb-4">
						<h2 class="text-2xl font-bold">{selectedPage.title}</h2>
						<div class="mt-2 flex flex-wrap items-center gap-2">
							<span class="rounded px-1.5 py-0.5 text-xs font-medium {typeColor(selectedPage.page_type)}">
								{selectedPage.page_type}
							</span>
							{#if selectedPage.updated}
								<span class="text-xs text-muted-foreground">
									{m.wiki_updated_label()}: {selectedPage.updated}
								</span>
							{/if}
							{#each selectedPage.tags as tag}
								<span class="inline-flex items-center gap-0.5 text-xs text-muted-foreground">
									<Tag class="h-3 w-3" />{tag}
								</span>
							{/each}
						</div>
					</div>

					<!-- TLDR box -->
					{#if selectedPage.tldr}
						<div class="mb-5 rounded-lg border bg-muted/40 p-4">
							<p class="mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								{m.wiki_tldr_heading()}
							</p>
							<p class="text-sm leading-relaxed">{selectedPage.tldr}</p>
						</div>
					{/if}

					<!-- Body -->
					{#if selectedPage.body}
						<div class="prose prose-sm dark:prose-invert max-w-none [&>*:first-child]:mt-0 [&>*:last-child]:mb-0">
							<Streamdown
								content={selectedPage.body}
								shikiTheme={currentTheme}
								baseTheme="shadcn"
								components={{ code: Code }}
								{shikiThemes}
							/>
						</div>
					{/if}

					<!-- Wikilinks -->
					{#if selectedPage.wikilinks.length > 0}
						<div class="mt-6 border-t pt-4">
							<p class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								<Link class="mr-1 inline h-3 w-3" />{m.wiki_wikilinks_label()}
							</p>
							<div class="flex flex-wrap gap-1.5">
								{#each selectedPage.wikilinks as link}
									<button
										class="rounded-md border px-2 py-0.5 text-xs text-foreground transition-colors hover:bg-accent"
										onclick={() => handleWikilinkClick(link)}
									>
										{link}
									</button>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			{:else}
				<!-- Empty state -->
				<div class="flex flex-1 flex-col items-center justify-center gap-3 text-center">
					<BookOpen class="h-12 w-12 text-muted-foreground/40" />
					<div>
						<p class="font-medium text-muted-foreground">{m.wiki_select_page_prompt()}</p>
						<p class="mt-1 text-sm text-muted-foreground/60">{m.wiki_select_page_hint()}</p>
					</div>
				</div>
			{/if}
		</div>
	</div>
</div>

<!-- Ingest dialog -->
<Dialog.Root bind:open={ingestOpen}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>{m.wiki_ingest_dialog_title()}</Dialog.Title>
		</Dialog.Header>
		<div class="space-y-3">
			<!-- File upload drop zone -->
			<div
				class="flex cursor-pointer flex-col items-center justify-center rounded-lg border-2 border-dashed border-muted-foreground/30 p-6 transition-colors hover:border-muted-foreground/50"
				onclick={() => fileInput?.click()}
				onkeydown={(e) => e.key === 'Enter' && fileInput?.click()}
				ondragover={handleDropZoneDragOver}
				ondrop={handleDropZoneDrop}
				role="button"
				tabindex="0"
			>
				<Upload class="mb-2 h-8 w-8 text-muted-foreground/50" />
				<p class="text-sm text-muted-foreground">
					{m.wiki_ingest_drop_hint()}
				</p>
				<input
					bind:this={fileInput}
					type="file"
					accept=".md,.txt,.html"
					class="hidden"
					onchange={handleFileSelect}
				/>
			</div>

			<div class="space-y-1">
				<Input
					placeholder={m.wiki_ingest_filename_placeholder()}
					bind:value={ingestFilename}
				/>
				<p class="text-[11px] text-muted-foreground px-0.5">
					{m.wiki_ingest_filename_hint()}
				</p>
			</div>
			<Textarea
				placeholder={m.wiki_ingest_content_placeholder()}
				bind:value={ingestContent}
				rows={6}
				class="font-mono text-xs"
			/>
			<Button class="w-full gap-1.5" onclick={handleIngest} disabled={ingesting}>
				{#if ingesting}
					<Loader2 class="h-4 w-4 animate-spin" />
				{:else}
					<FileUp class="h-4 w-4" />
				{/if}
				{m.wiki_ingest_submit()}
			</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>
