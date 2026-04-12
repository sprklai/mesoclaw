<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { toast } from 'svelte-sonner';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Badge } from '$lib/components/ui/badge';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Streamdown } from 'svelte-streamdown';
	import Code from 'svelte-streamdown/code';
	import WikiGraph from '$lib/components/wiki/WikiGraph.svelte';
	import { wikiStore, type WikiPage, type LintIssue, type FixedIssue, type QueryResult } from '$lib/stores/wiki.svelte';
	import { themeStore } from '$lib/stores/theme.svelte';
	import { shikiThemes } from '$lib/components/ai-elements/code/shiki';
	import { isTauri, openPath, openConfigFile, openWikiDir, openInBrowser } from '$lib/tauri';
	import { configStore } from '$lib/stores/config.svelte';
	import * as m from '$lib/paraglide/messages';
	import Search from '@lucide/svelte/icons/search';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import GitFork from '@lucide/svelte/icons/git-fork';
	import RefreshCw from '@lucide/svelte/icons/refresh-cw';
	import Upload from '@lucide/svelte/icons/upload';
	import FileUp from '@lucide/svelte/icons/file-up';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import X from '@lucide/svelte/icons/x';
	import Tag from '@lucide/svelte/icons/tag';
	import Link from '@lucide/svelte/icons/link';
	import Loader2 from '@lucide/svelte/icons/loader-2';
	import MessageCircleQuestion from '@lucide/svelte/icons/message-circle-question';
	import ShieldCheck from '@lucide/svelte/icons/shield-check';
	import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
	import AlertTriangle from '@lucide/svelte/icons/alert-triangle';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import Settings from '@lucide/svelte/icons/settings';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import RotateCw from '@lucide/svelte/icons/rotate-cw';

	const CATEGORIES = ['all', 'concepts', 'entities', 'topics', 'comparisons', 'queries'] as const;
	// Add new accepted types here — drives both the file input and the drop zone hint.
	// Text types go to /wiki/ingest (JSON body); binary types go to /wiki/upload (multipart).
	const INGEST_ACCEPT =
		'.md,.txt,.html,.htm,.org,.rst,' +
		'application/pdf,.pdf,' +
		'application/vnd.openxmlformats-officedocument.wordprocessingml.document,.docx,' +
		'application/msword,.doc,' +
		'application/vnd.ms-powerpoint,.ppt,' +
		'application/vnd.openxmlformats-officedocument.presentationml.presentation,.pptx,' +
		'application/vnd.ms-excel,.xls,' +
		'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,.xlsx,' +
		'application/zip,.zip,' +
		'application/epub+zip,.epub,' +
		'image/jpeg,.jpg,.jpeg,image/png,.png,image/gif,.gif,image/webp,.webp,image/bmp,.bmp,image/tiff,.tiff';
	const BINARY_EXTENSIONS = new Set([
		'pdf', 'docx', 'doc', 'pptx', 'ppt', 'xlsx', 'xls',
		'jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'tiff',
		'zip', 'epub'
	]);
	const BINARY_MIME_PREFIXES = [
		'application/pdf',
		'application/vnd.openxmlformats',
		'application/vnd.ms-',
		'application/msword',
		'application/zip',
		'application/epub',
		'image/'
	];

	function isBinaryFile(file: File): boolean {
		const ext = file.name.split('.').pop()?.toLowerCase() ?? '';
		if (BINARY_EXTENSIONS.has(ext)) return true;
		return BINARY_MIME_PREFIXES.some((prefix) => file.type.startsWith(prefix));
	}

	type Category = (typeof CATEGORIES)[number];

	let query = $state('');
	let searchTimeout: ReturnType<typeof setTimeout>;
	let activeCategory = $state<Category>('all');
	let selectedPage = $state<WikiPage | null>(null);
	let pageLoading = $state(false);
	// Replace [[slug]] wikilink syntax with markdown links so the prose renderer shows them.
	const processedBody = $derived(
		selectedPage?.body.replace(/\[\[([^\]]+)\]\]/g, (_, slug) => `[${slug}](#wiki-${slug})`) ?? ''
	);
	let showGraph = $state(true);
	let ingestOpen = $state(false);
	let ingestFiles = $state<File[]>([]);
	let ingestProgress = $state<{ current: number; total: number } | null>(null);
	let ingesting = $state(false);
	let fileInput: HTMLInputElement | undefined = $state();

	const CATEGORY_TYPE: Record<string, string> = {
		concepts: 'concept',
		entities: 'entity',
		topics: 'topic',
		comparisons: 'comparison',
		queries: 'query'
	};

	let filteredPages = $derived.by(() => {
		if (activeCategory === 'all') return wikiStore.pages;
		const type = CATEGORY_TYPE[activeCategory] ?? activeCategory.slice(0, -1);
		return wikiStore.pages.filter((p) => p.page_type === type);
	});

	// L6: deferred — currentTheme is already a minimal $derived that only changes when
	// isDark flips. Svelte 5 re-renders only the Streamdown components that receive the
	// updated prop; debouncing would add complexity without meaningful gain here.
	let currentTheme = $derived(themeStore.isDark ? 'github-dark-default' : 'github-light-default');

	onMount(async () => {
		wikiStore.load();
		wikiStore.loadGraph();
		if (!configStore.config || Object.keys(configStore.config).length === 0) {
			await configStore.load();
		}
	});



	onDestroy(() => {
		clearTimeout(searchTimeout);
		clearTimeout(sourceSearchTimeout);
		clearTimeout(lintSearchTimeout);
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
		// M10: guard against multiple rapid clicks enqueueing concurrent loadGraph() calls
		if (wikiStore.graphLoading) return;
		showGraph = !showGraph;
		if (showGraph && !wikiStore.graph) {
			await wikiStore.loadGraph();
		}
	}

	function addFiles(files: FileList | File[]) {
		const existing = new Set(ingestFiles.map((f) => f.name));
		const incoming = Array.from(files).filter((f) => !existing.has(f.name));
		ingestFiles = [...ingestFiles, ...incoming];
	}

	function removeIngestFile(index: number) {
		ingestFiles = ingestFiles.filter((_, i) => i !== index);
	}

	function handleFileSelect(e: Event) {
		const input = e.currentTarget as HTMLInputElement;
		if (input.files?.length) addFiles(input.files);
		input.value = '';
	}

	function handleDropZoneDragOver(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
	}

	function handleDropZoneDrop(e: DragEvent) {
		e.preventDefault();
		if (e.dataTransfer?.files.length) addFiles(e.dataTransfer.files);
	}

	// Whole-page drag-and-drop: open ingest dialog and queue files
	let pageDragOver = $state(false);

	function handlePageDragOver(e: DragEvent) {
		if (!e.dataTransfer) return;
		const { types } = e.dataTransfer;
		// 'Files' = Chromium-based browsers; 'text/uri-list' = WebKitGTK (Tauri on Linux/macOS)
		if (!types.includes('Files') && !types.includes('text/uri-list')) return;
		e.preventDefault();
		pageDragOver = true;
	}

	function handlePageDragLeave(e: DragEvent) {
		// Only clear if leaving the outermost element
		if ((e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) return;
		pageDragOver = false;
	}

	function handlePageDrop(e: DragEvent) {
		e.preventDefault();
		pageDragOver = false;
		if (e.dataTransfer?.files.length) addFiles(e.dataTransfer.files);
		ingestOpen = true;
	}

	function handleIngestDialogClose() {
		if (!ingesting) {
			ingestFiles = [];
			ingestProgress = null;
		}
	}

	// H4: max file size before calling file.text() to prevent OOM/hang on large binaries
	const MAX_TEXT_SIZE = 10 * 1024 * 1024; // 10MB

	async function handleIngest() {
		if (ingestFiles.length === 0) return;
		ingesting = true;
		let succeeded = 0;
		for (let i = 0; i < ingestFiles.length; i++) {
			ingestProgress = { current: i + 1, total: ingestFiles.length };
			const file = ingestFiles[i];
			// H4: reject files over 10MB before reading to avoid OOM/hang
			if (file.size > MAX_TEXT_SIZE) {
				toast.error(`${file.name}: file too large (max 10MB)`);
				continue;
			}
			try {
				let res: { slug: string; page_count: number; message: string };
				if (isBinaryFile(file)) {
					// H6: binary files (PDF/DOCX/image) go to /wiki/upload (multipart)
					res = await wikiStore.uploadBinary(file);
				} else {
					const content = await file.text();
					res = await wikiStore.ingest(file.name, content);
				}
				// Show meaningful feedback: extraction status or fallback notice
				const isLlmFallback = res.page_count <= 1 && res.message.includes('LLM unavailable');
				if (isLlmFallback) {
					toast.warning(`${file.name}: ${res.message}`);
				} else {
					toast.success(res.message);
				}
				succeeded++;
			} catch (e) {
				const msg = e instanceof Error ? e.message : m.wiki_ingest_error();
				toast.error(`${file.name}: ${msg}`);
				console.error('[wiki] ingest failed:', file.name, e);
			}
		}
		ingesting = false;
		ingestProgress = null;
		if (succeeded > 0) {
			ingestOpen = false;
			ingestFiles = [];
			await wikiStore.load();
			wikiStore.loadGraph();
			wikiStore.fetchSources();
		}
	}

	function handleCategoryClick(cat: Category) {
		activeCategory = cat;
		selectedPage = null;
	}

	function handleWikilinkClick(slug: string) {
		handleSelectPage(slug);
	}

	// ── Query ────────────────────────────────────────────────────────────────────

	let queryOpen = $state(false);
	let queryQuestion = $state('');
	let querySave = $state(false);
	let queryResult = $state<QueryResult | null>(null);
	let queryError = $state<string | null>(null);

	async function handleQuery() {
		if (!queryQuestion.trim()) return;
		queryResult = null;
		queryError = null;
		try {
			queryResult = await wikiStore.query(queryQuestion.trim(), querySave);
			if (querySave && queryResult.saved_page) {
				await wikiStore.load();
			}
		} catch (e) {
			queryError = e instanceof Error ? e.message : m.wiki_query_error();
			toast.error(queryError);
		}
	}

	// ── Open Folder ─────────────────────────────────────────────────────────────

	async function handleOpenFolder() {
		// In Tauri: use the open_wiki_dir command which resolves wiki_dir from
		// config on the Rust side — this works for custom wiki_dir paths that
		// would be outside the default capability scope.
		if (isTauri) {
			try {
				await openWikiDir();
			} catch (e) {
				console.error('[wiki] openWikiDir failed:', e);
				toast.error('Could not open wiki folder');
			}
			return;
		}
		// Browser fallback: fetch path from daemon then use opener.
		let path: string;
		try {
			path = await wikiStore.fetchWikiDir();
		} catch (e) {
			console.error('[wiki] fetchWikiDir failed:', e);
			toast.error('Could not fetch wiki folder path from daemon');
			return;
		}
		try {
			await openPath(path);
		} catch (e) {
			console.error('[wiki] openPath failed:', e, 'path:', path);
			// Fall back: show path in toast so user can navigate manually
			toast.info(path);
		}
	}

	// ── Popovers / modals ────────────────────────────────────────────────────────

	// M7: single source of truth for which inline popover is open — only one can be open at a time.
	// Modal dialogs (promptOpen, deleteWikiOpen, etc.) are separate because they use Dialog.Root.
	let openPopover = $state<'lint' | 'sources' | 'gear' | null>(null);
	function closeAllPopovers() { openPopover = null; }
	let lintPopOpen = $derived(openPopover === 'lint');
	let sourcesPopOpen = $derived(openPopover === 'sources');
	let gearOpen = $derived(openPopover === 'gear');

	let promptOpen = $state(false);
	let promptContent = $state('');
	let promptLoading = $state(false);
	let promptSaving = $state(false);
	let deleteWikiOpen = $state(false);
	let deleteConfirmText = $state('');
	let deletingWiki = $state(false);
	let deleteAllSourcesOpen = $state(false);
	let deletingAllSources = $state(false);

	function handleDocumentClick(e: MouseEvent) {
		const target = e.target as HTMLElement;
		if (!target.closest('[data-popover-anchor]') && !target.closest('[role="dialog"]')) {
			closeAllPopovers();
		}
	}

	// ── Sources panel ────────────────────────────────────────────────────────────

	let confirmDeleteFilename = $state<string | null>(null);
	let confirmDeleteOpen = $state(false);
	let regenerateConfirmOpen = $state(false);

	// Sources search & filter
	let sourceSearch = $state('');
	let sourceSearchDebounced = $state('');
	let sourceSearchTimeout: ReturnType<typeof setTimeout>;
	let sourceFilter = $state<'all' | 'active' | 'inactive'>('all');
	let regeneratingSource = $state<string | null>(null);

	// Lint search & filter
	let lintSearch = $state('');
	let lintSearchDebounced = $state('');
	let lintSearchTimeout: ReturnType<typeof setTimeout>;
	type LintKindFilter = 'all' | 'broken_wikilink' | 'orphan_page' | 'missing_index_entry' | 'missing_updated';
	let lintKindFilter = $state<LintKindFilter>('all');

	const visibleSources = $derived(
		wikiStore.sources
			.filter(s =>
				sourceFilter === 'all' ||
				(sourceFilter === 'active' ? s.active : !s.active)
			)
			.filter(s =>
				!sourceSearchDebounced ||
				s.filename.toLowerCase().includes(sourceSearchDebounced.toLowerCase())
			)
	);

	const visibleIssues = $derived(
		(wikiStore.lintIssues ?? [])
			.filter(i => lintKindFilter === 'all' || i.kind === lintKindFilter)
			.filter(i =>
				!lintSearchDebounced ||
				i.page_slug.toLowerCase().includes(lintSearchDebounced.toLowerCase()) ||
				i.detail.toLowerCase().includes(lintSearchDebounced.toLowerCase())
			)
	);

	async function handleToggleSources() {
		if (sourcesPopOpen) {
			closeAllPopovers();
			return;
		}
		openPopover = 'sources';
		if (wikiStore.sources.length === 0) {
			await wikiStore.fetchSources();
		}
	}

	function handleDeleteSourceClick(filename: string) {
		confirmDeleteFilename = filename;
		confirmDeleteOpen = true;
	}

	async function handleDeleteSourceConfirm() {
		if (!confirmDeleteFilename) return;
		const filename = confirmDeleteFilename;
		confirmDeleteOpen = false;
		confirmDeleteFilename = null;
		try {
			const result = await wikiStore.deleteSource(filename);
			toast.success(m.wiki_sources_delete_success({ filename: result.filename }));
		} catch (e) {
			toast.error(m.wiki_sources_delete_error());
			console.error('[wiki] delete source failed:', e);
		}
	}

	async function handleOpenConfig() {
		closeAllPopovers();
		try {
			await openConfigFile();
		} catch (e) {
			toast.error('Failed to open config', { description: e instanceof Error ? e.message : String(e) });
		}
	}

	async function handleOpenPrompt() {
		if (promptLoading) return;
		closeAllPopovers();
		promptLoading = true;
		try {
			promptContent = await wikiStore.fetchPrompt();
		} finally {
			promptLoading = false;
		}
		promptOpen = true;
	}

	async function handleSavePrompt() {
		promptSaving = true;
		try {
			await wikiStore.savePrompt(promptContent);
			toast.success(m.wiki_prompt_saved());
			promptOpen = false;
		} catch (e) {
			toast.error(e instanceof Error ? e.message : String(e));
		} finally {
			promptSaving = false;
		}
	}

	async function handleDeleteWiki() {
		if (deleteConfirmText !== 'DELETE') return;
		deletingWiki = true;
		try {
			const count = await wikiStore.deleteAllPages();
			toast.success(m.wiki_delete_success({ count: count.toString() }));
			selectedPage = null;
			deleteWikiOpen = false;
			deleteConfirmText = '';
			await wikiStore.load();
			await wikiStore.loadGraph();
		} catch (e) {
			toast.error(e instanceof Error ? e.message : String(e));
		} finally {
			deletingWiki = false;
		}
	}

	async function handleDeleteAllSources() {
		deletingAllSources = true;
		try {
			const count = await wikiStore.deleteAllSources();
			toast.success(m.wiki_sources_delete_all_success({ count: count.toString() }));
			deleteAllSourcesOpen = false;
		} catch (e) {
			toast.error(e instanceof Error ? e.message : String(e));
		} finally {
			deletingAllSources = false;
		}
	}

	async function handleRegenerate() {
		regenerateConfirmOpen = false;
		try {
			const result = await wikiStore.regenerate();
			toast.success(m.wiki_regenerate_success({
				pages: result.pages_generated.toString(),
				sources: result.sources_processed.toString()
			}));
		} catch (e) {
			toast.error(m.wiki_regenerate_error());
			console.error('[wiki] regenerate failed:', e);
		}
	}

	// ── Lint ────────────────────────────────────────────────────────────────────

	async function handleLint() {
		if (lintPopOpen) {
			closeAllPopovers();
			return;
		}
		openPopover = 'lint';
		if (!wikiStore.lintIssues) {
			try {
				await wikiStore.lint();
			} catch {
				toast.error(m.wiki_lint_error());
			}
		}
	}

	async function handleRelint() {
		try {
			await wikiStore.lint();
		} catch {
			toast.error(m.wiki_lint_error());
		}
	}

	function lintKindIcon(kind: string): string {
		switch (kind) {
			case 'broken_wikilink': return '🔗';
			case 'orphan_page': return '🏝';
			case 'missing_index_entry': return '📋';
			case 'missing_updated': return '📅';
			default: return '⚠';
		}
	}

	function handleSourceSearch(e: Event) {
		clearTimeout(sourceSearchTimeout);
		const val = (e.currentTarget as HTMLInputElement).value;
		sourceSearch = val;
		sourceSearchTimeout = setTimeout(() => { sourceSearchDebounced = val; }, 300);
	}

	function handleLintSearch(e: Event) {
		clearTimeout(lintSearchTimeout);
		const val = (e.currentTarget as HTMLInputElement).value;
		lintSearch = val;
		lintSearchTimeout = setTimeout(() => { lintSearchDebounced = val; }, 300);
	}

	function sourceForSlug(slug: string): string | null {
		return wikiStore.sources.find(s => s.pages.includes(slug))?.filename ?? null;
	}

	async function handleRegenerateSource(filename: string) {
		regeneratingSource = filename;
		try {
			await wikiStore.regenerateSource(filename);
			toast.success(`Regenerated pages from "${filename}"`);
		} catch (e) {
			toast.error(e instanceof Error ? e.message : 'Regeneration failed');
			console.error('[wiki] regenerateSource failed:', filename, e);
		} finally {
			regeneratingSource = null;
		}
	}

	async function handleRegenerateFromIssue(issue: LintIssue) {
		const src = sourceForSlug(issue.page_slug);
		if (!src) return;
		try {
			await wikiStore.regenerateSource(src);
			toast.success(`Regenerated pages from "${src}"`);
			// Re-lint to update issue list
			await wikiStore.lint();
		} catch (e) {
			toast.error(e instanceof Error ? e.message : 'Regeneration failed');
			console.error('[wiki] regenerateFromIssue failed:', issue, e);
		}
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

<svelte:document onclick={handleDocumentClick} />

<div
	class="flex h-full flex-col gap-0 overflow-hidden {pageDragOver ? 'outline outline-2 outline-primary/50' : ''}"
	ondragover={handlePageDragOver}
	ondragleave={handlePageDragLeave}
	ondrop={handlePageDrop}
	role="region"
	aria-label="Wiki knowledge base viewer"
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
			<!-- Graph toggle -->
			<Button
				variant={showGraph ? 'default' : 'outline'}
				size="sm"
				class="gap-1.5"
				onclick={handleToggleGraph}
			>
				<GitFork class="h-3.5 w-3.5" />
				{m.wiki_graph_toggle()}
			</Button>

			<!-- Ask -->
			<Button
				variant="outline"
				size="sm"
				class="gap-1.5"
				onclick={() => { queryOpen = true; queryResult = null; queryError = null; }}
			>
				<MessageCircleQuestion class="h-3.5 w-3.5" />
				{m.wiki_query_button()}
			</Button>

			<!-- Lint popover -->
			<div class="relative" data-popover-anchor>
				<Button
					variant={lintPopOpen ? 'default' : 'outline'}
					size="sm"
					class="gap-1.5"
					onclick={(e) => { e.stopPropagation(); handleLint(); }}
					disabled={wikiStore.linting}
				>
					{#if wikiStore.linting}
						<Loader2 class="h-3.5 w-3.5 animate-spin" />
						{m.wiki_lint_running()}
					{:else}
						<ShieldCheck class="h-3.5 w-3.5" />
						{m.wiki_lint_button()}
						{#if wikiStore.lintIssues !== null && wikiStore.lintIssues.length > 0}
							<span class="rounded-full bg-yellow-500/20 px-1.5 py-0 text-[10px] font-bold text-yellow-600 dark:text-yellow-400">
								{wikiStore.lintIssues.length}
							</span>
						{:else if wikiStore.lintIssues !== null && wikiStore.lintIssues.length === 0}
							<CheckCircle2 class="h-3 w-3 text-green-500" />
						{/if}
						<ChevronDown class="h-3 w-3" />
					{/if}
				</Button>
				{#if lintPopOpen}
					<div class="absolute right-0 top-full z-50 mt-1.5 w-[22rem] rounded-lg border bg-popover shadow-lg" role="none" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
						<div class="flex items-center justify-between border-b px-3 py-2">
							<span class="text-sm font-semibold">{m.wiki_lint_button()}</span>
							<div class="flex items-center gap-2">
								{#if wikiStore.lintIssues !== null}
									<span class="text-xs text-muted-foreground">
										{wikiStore.lintIssues.length === 0 ? m.wiki_lint_no_issues() : m.wiki_lint_issue_count({ count: wikiStore.lintIssues.length.toString(), suffix: wikiStore.lintIssues.length === 1 ? '' : 's' })}
									</span>
								{/if}
								<button class="rounded p-0.5 text-muted-foreground hover:bg-muted" onclick={() => closeAllPopovers()}><X class="h-3.5 w-3.5" /></button>
							</div>
						</div>
						<!-- Auto-fix summary -->
						{#if wikiStore.lintFixed.length > 0}
							<div class="border-b bg-green-500/5 px-3 py-1.5 text-xs text-green-700 dark:text-green-400">
								Auto-fixed {wikiStore.lintFixed.length} issue{wikiStore.lintFixed.length === 1 ? '' : 's'}
							</div>
						{/if}
						<!-- Search + filter bar (only when there are issues) -->
						{#if wikiStore.lintIssues !== null && wikiStore.lintIssues.length > 0}
							<div class="border-b px-2 py-2 space-y-1.5">
								<input
									class="w-full rounded-md border bg-background px-2.5 py-1.5 text-xs placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
									placeholder="Filter issues..."
									value={lintSearch}
									oninput={handleLintSearch}
								/>
								<div class="flex flex-wrap gap-1">
									{#each [
										{ v: 'all', label: 'All' },
										{ v: 'broken_wikilink', label: '🔗 Broken' },
										{ v: 'orphan_page', label: '🏝 Orphan' },
										{ v: 'missing_index_entry', label: '📋 Index' },
										{ v: 'missing_updated', label: '📅 Date' }
									] as f}
										<button
											class="rounded px-2 py-0.5 text-[11px] font-medium transition-colors {lintKindFilter === f.v ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-muted/80'}"
											onclick={() => { lintKindFilter = f.v as LintKindFilter; }}
										>{f.label}</button>
									{/each}
								</div>
							</div>
						{/if}
						<!-- Issue list -->
						{#if wikiStore.lintIssues !== null && visibleIssues.length > 0}
							<div class="max-h-52 overflow-y-auto p-2 space-y-1.5">
								{#each visibleIssues as issue}
									<div class="rounded-md border bg-background p-2 text-xs">
										<div class="flex items-center justify-between gap-1.5">
											<div class="flex min-w-0 flex-1 items-center gap-1.5">
												<AlertTriangle class="h-3.5 w-3.5 shrink-0 text-yellow-500" />
												<span class="font-mono font-medium text-yellow-600 dark:text-yellow-400">{issue.kind}</span>
												<button
													class="truncate font-medium text-primary hover:underline"
													onclick={() => { handleSelectPage(issue.page_slug); lintPopOpen = false; }}
												>{issue.page_slug}</button>
											</div>
											{#if sourceForSlug(issue.page_slug)}
												<button
													class="shrink-0 rounded p-0.5 text-muted-foreground hover:bg-primary/10 hover:text-primary disabled:opacity-40"
													onclick={() => handleRegenerateFromIssue(issue)}
													disabled={wikiStore.regenerating}
													title="Regenerate pages from source"
												>
													<RotateCw class="h-3 w-3" />
												</button>
											{/if}
										</div>
										<p class="mt-1 text-muted-foreground">{issue.detail}</p>
										{#if issue.fix}
											<p class="mt-0.5 text-muted-foreground/70"><span class="font-medium">{m.wiki_lint_fix_label()}:</span> {issue.fix}</p>
										{/if}
									</div>
								{/each}
							</div>
						{:else if wikiStore.lintIssues !== null && wikiStore.lintIssues.length > 0 && visibleIssues.length === 0}
							<p class="p-3 text-center text-sm text-muted-foreground">No issues match filter</p>
						{:else if wikiStore.lintIssues !== null && wikiStore.lintIssues.length === 0}
							<p class="p-3 text-center text-sm text-muted-foreground">{m.wiki_lint_no_issues()}</p>
						{:else}
							<p class="p-3 text-center text-sm text-muted-foreground">Run lint to check for issues</p>
						{/if}
						<div class="border-t p-2">
							<Button size="sm" class="w-full gap-1.5" onclick={handleRelint} disabled={wikiStore.linting}>
								{#if wikiStore.linting}<Loader2 class="h-3.5 w-3.5 animate-spin" />{:else}<RefreshCw class="h-3.5 w-3.5" />{/if}
								{m.wiki_lint_running()}
							</Button>
						</div>
					</div>
				{/if}
			</div>

			<!-- Sources popover -->
			<div class="relative" data-popover-anchor>
				<Button
					variant={sourcesPopOpen ? 'default' : 'outline'}
					size="sm"
					class="gap-1.5"
					onclick={(e) => { e.stopPropagation(); handleToggleSources(); }}
				>
					<ChevronDown class="h-3.5 w-3.5 transition-transform {sourcesPopOpen ? 'rotate-180' : ''}" />
					{m.wiki_sources_button()}
					{#if wikiStore.sources.length > 0}
						<span class="rounded-full bg-primary/20 px-1.5 py-0 text-[10px] font-medium text-primary">
							{wikiStore.sources.length}
						</span>
					{/if}
				</Button>
				{#if sourcesPopOpen}
					<div class="absolute right-0 top-full z-50 mt-1.5 w-[22rem] rounded-lg border bg-popover shadow-lg" role="none" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
						<div class="flex items-center justify-between border-b px-3 py-2">
							<span class="text-sm font-semibold">{m.wiki_sources_button()}</span>
							<button class="rounded p-0.5 text-muted-foreground hover:bg-muted" onclick={() => closeAllPopovers()}><X class="h-3.5 w-3.5" /></button>
						</div>
						<!-- Search + filter bar -->
						<div class="border-b px-2 py-2 space-y-1.5">
							<input
								class="w-full rounded-md border bg-background px-2.5 py-1.5 text-xs placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
								placeholder="Search sources..."
								value={sourceSearch}
								oninput={handleSourceSearch}
							/>
							<div class="flex gap-1">
								{#each ['all', 'active', 'inactive'] as f}
									<button
										class="rounded px-2 py-0.5 text-[11px] font-medium transition-colors {sourceFilter === f ? 'bg-primary text-primary-foreground' : 'bg-muted text-muted-foreground hover:bg-muted/80'}"
										onclick={() => { sourceFilter = f as typeof sourceFilter; }}
									>{f === 'all' ? 'All' : f === 'active' ? 'Active' : 'Inactive'}</button>
								{/each}
							</div>
						</div>
						<div class="max-h-48 overflow-y-auto p-2">
							{#if wikiStore.sourcesLoading}
								<div class="space-y-1.5">
									{#each Array(3) as _}<Skeleton class="h-8 w-full" />{/each}
								</div>
							{:else if visibleSources.length === 0}
								<p class="py-2 text-center text-sm text-muted-foreground">
									{wikiStore.sources.length === 0 ? m.wiki_sources_empty() : 'No sources match filter'}
								</p>
							{:else}
								<div class="space-y-1">
									{#each visibleSources as source (source.filename)}
										<div class="flex items-center justify-between rounded-md border bg-background px-2.5 py-1.5 text-xs">
											<div class="flex min-w-0 flex-1 items-center gap-2">
												<span class="truncate font-medium">{source.filename}</span>
												<span class="shrink-0 font-mono text-[10px] text-muted-foreground">{source.hash.slice(0, 8)}</span>
												<span class="shrink-0 rounded px-1 py-0 text-[10px] {source.active ? 'bg-green-500/10 text-green-600 dark:text-green-400' : 'bg-muted text-muted-foreground'}">
													{source.active ? m.wiki_source_status_active() : m.wiki_source_status_inactive()}
												</span>
											</div>
											<div class="ml-2 flex shrink-0 items-center gap-0.5">
												<button
													class="rounded p-1 text-muted-foreground hover:bg-primary/10 hover:text-primary disabled:opacity-40"
													onclick={() => handleRegenerateSource(source.filename)}
													disabled={regeneratingSource === source.filename || wikiStore.regenerating}
													title="Regenerate pages from this source"
												>
													{#if regeneratingSource === source.filename}
														<Loader2 class="h-3.5 w-3.5 animate-spin" />
													{:else}
														<RotateCw class="h-3.5 w-3.5" />
													{/if}
												</button>
												<button
													class="rounded p-1 text-muted-foreground hover:bg-destructive/10 hover:text-destructive"
													onclick={() => handleDeleteSourceClick(source.filename)}
												><Trash2 class="h-3.5 w-3.5" /></button>
											</div>
										</div>
									{/each}
								</div>
							{/if}
						</div>
						<div class="space-y-1.5 border-t p-2">
							<div class="flex gap-1.5">
								<Button variant="outline" size="sm" class="h-7 flex-1 gap-1 text-xs" onclick={handleOpenFolder}><FolderOpen class="h-3 w-3" /> {m.wiki_open_folder()}</Button>
								<Button variant="outline" size="sm" class="h-7 flex-1 gap-1 text-xs" onclick={() => (regenerateConfirmOpen = true)} disabled={wikiStore.regenerating || wikiStore.sources.length === 0}>
									{#if wikiStore.regenerating}<Loader2 class="h-3 w-3 animate-spin" />{:else}<RefreshCw class="h-3 w-3" />{/if}
									{m.wiki_regenerate_button()}
								</Button>
							</div>
							<Button
								variant="outline"
								size="sm"
								class="h-7 w-full gap-1 text-xs text-destructive hover:bg-destructive/10 hover:text-destructive border-destructive/30"
								onclick={() => { deleteAllSourcesOpen = true; closeAllPopovers(); }}
								disabled={wikiStore.sources.length === 0}
							>
								<Trash2 class="h-3 w-3" />
								{m.wiki_sources_delete_all_button()}
							</Button>
						</div>
					</div>
				{/if}
			</div>

			<!-- Ingest -->
			<Button variant="outline" size="sm" class="gap-1.5" onclick={() => (ingestOpen = true)}>
				<FileUp class="h-3.5 w-3.5" />
				{m.wiki_ingest_button()}
			</Button>

			<!-- Gear / Settings -->
			<div class="relative" data-popover-anchor>
				<Button
					variant={gearOpen ? 'default' : 'outline'}
					size="sm"
					class="px-2"
					onclick={(e) => { e.stopPropagation(); openPopover = gearOpen ? null : 'gear'; }}
					aria-label="Wiki settings"
				>
					<Settings class="h-3.5 w-3.5" />
				</Button>
				{#if gearOpen}
					<div class="absolute right-0 top-full z-50 mt-1.5 w-52 rounded-lg border bg-popover py-1 shadow-lg" role="none" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
						<button
							class="flex w-full items-center gap-2.5 px-3 py-2 text-sm text-foreground hover:bg-muted"
							onclick={() => { closeAllPopovers(); handleSync(); }}
							disabled={wikiStore.syncing}
						>
							<RefreshCw class="h-3.5 w-3.5 text-muted-foreground {wikiStore.syncing ? 'animate-spin' : ''}" />
							{m.wiki_gear_sync()}
						</button>
						<div class="my-1 h-px bg-border"></div>
						<p class="px-3 py-1 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">Configuration</p>
						<button
							class="flex w-full items-center gap-2.5 px-3 py-2 text-sm text-foreground hover:bg-muted"
							onclick={handleOpenPrompt}
						>
							<Tag class="h-3.5 w-3.5 text-muted-foreground" />
							{m.wiki_gear_change_prompt()}
						</button>
						{#if isTauri}
							<button
								class="flex w-full items-center gap-2.5 px-3 py-2 text-sm text-foreground hover:bg-muted"
								onclick={handleOpenConfig}
							>
								<ExternalLink class="h-3.5 w-3.5 text-muted-foreground" />
								Edit graph settings
							</button>
						{/if}
						<div class="my-1 h-px bg-border"></div>
						<p class="px-3 py-1 text-[10px] font-semibold uppercase tracking-wide text-muted-foreground">Danger zone</p>
						<button
							class="flex w-full items-center gap-2.5 px-3 py-2 text-sm text-destructive hover:bg-destructive/10"
							onclick={() => { closeAllPopovers(); deleteWikiOpen = true; deleteConfirmText = ''; }}
						>
							<Trash2 class="h-3.5 w-3.5" />
							{m.wiki_gear_delete_all()}
						</button>
					</div>
				{/if}
			</div>
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
			<div class="min-h-0 flex-1 overflow-y-auto">
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
								onclick={() => { showGraph = false; handleSelectPage(page.slug); }}
							>
								<div class="truncate text-xs font-medium">{page.title}</div>
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
				<div class="flex-1 overflow-hidden">
					{#if wikiStore.graphLoading}
						<div class="flex h-full items-center justify-center">
							<Loader2 class="h-8 w-8 animate-spin text-muted-foreground" />
						</div>
					{:else if !wikiStore.graph || wikiStore.graph.nodes.length === 0}
						<div class="flex h-full items-center justify-center">
							<p class="text-sm text-muted-foreground">{m.wiki_graph_empty()}</p>
						</div>
					{:else}
						<WikiGraph
							nodes={wikiStore.graph!.nodes}
							edges={wikiStore.graph!.edges}
							pages={wikiStore.pages}
							onnodeclick={(e) => { handleSelectPage(e.detail.slug); showGraph = false; }}
						/>
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
						<div
							role="none"
							class="prose prose-sm dark:prose-invert max-w-none [&>*:first-child]:mt-0 [&>*:last-child]:mb-0"
							onclick={(e) => {
								const a = (e.target as HTMLElement).closest('a');
								if (a) {
									const href = a.getAttribute('href') ?? '';
									if (href.startsWith('#wiki-')) {
										e.preventDefault();
										handleWikilinkClick(href.slice(6));
									} else if (href.startsWith('http://') || href.startsWith('https://')) {
										e.preventDefault();
										openInBrowser(href);
									}
								}
							}}
						>
							<Streamdown
								content={processedBody}
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

<!-- Query dialog -->
<Dialog.Root bind:open={queryOpen} onOpenChange={(open) => { if (!open) { queryResult = null; queryError = null; } }}>
	<Dialog.Content class="sm:max-w-2xl">
		<Dialog.Header>
			<Dialog.Title>{m.wiki_query_dialog_title()}</Dialog.Title>
		</Dialog.Header>
		<div class="space-y-3">
			<div class="flex gap-2">
				<textarea
					class="min-h-[72px] flex-1 resize-none rounded-md border bg-background px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
					placeholder={m.wiki_query_placeholder()}
					bind:value={queryQuestion}
					onkeydown={(e) => { if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) handleQuery(); }}
				></textarea>
				<Button
					class="shrink-0 self-end gap-1.5"
					onclick={handleQuery}
					disabled={wikiStore.querying || !queryQuestion.trim()}
				>
					{#if wikiStore.querying}
						<Loader2 class="h-4 w-4 animate-spin" />
						{m.wiki_query_asking()}
					{:else}
						<MessageCircleQuestion class="h-4 w-4" />
						{m.wiki_query_ask()}
					{/if}
				</Button>
			</div>
			<label class="flex cursor-pointer items-center gap-2 text-sm">
				<input type="checkbox" class="h-4 w-4" bind:checked={querySave} />
				{m.wiki_query_save_label()}
			</label>
			{#if queryError}
				<p class="text-sm text-destructive">{queryError}</p>
			{/if}
			{#if queryResult}
				<div class="max-h-96 overflow-y-auto space-y-3 rounded-md border p-3">
					<div>
						<p class="mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
							{m.wiki_query_answer_heading()}
						</p>
						<div class="prose prose-sm dark:prose-invert max-w-none [&>*:first-child]:mt-0">
							<Streamdown
								content={queryResult.answer}
								shikiTheme={currentTheme}
								baseTheme="shadcn"
								components={{ code: Code }}
								{shikiThemes}
							/>
						</div>
					</div>
					{#if queryResult.citations.length > 0}
						<div class="border-t pt-2">
							<p class="mb-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								{m.wiki_query_citations_heading()}
							</p>
							<div class="flex flex-wrap gap-1.5">
								{#each queryResult.citations as slug}
									<button
										class="rounded-md border px-2 py-0.5 text-xs text-foreground transition-colors hover:bg-accent"
										onclick={() => { queryOpen = false; handleSelectPage(slug); }}
									>
										{slug}
									</button>
								{/each}
							</div>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</Dialog.Content>
</Dialog.Root>

<!-- Delete source confirmation dialog -->
<Dialog.Root bind:open={confirmDeleteOpen} onOpenChange={(open) => { if (!open) confirmDeleteFilename = null; }}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>{m.wiki_sources_delete_confirm_title()}</Dialog.Title>
		</Dialog.Header>
		<p class="text-sm text-muted-foreground">
			{m.wiki_sources_delete_confirm_body({ filename: confirmDeleteFilename ?? '' })}
		</p>
		<Dialog.Footer>
			<Button variant="outline" onclick={() => (confirmDeleteOpen = false)}>{m.common_cancel()}</Button>
			<Button variant="destructive" onclick={handleDeleteSourceConfirm}>
				{m.wiki_sources_delete_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- Regenerate confirmation dialog -->
<Dialog.Root bind:open={regenerateConfirmOpen}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>{m.wiki_regenerate_confirm_title()}</Dialog.Title>
		</Dialog.Header>
		<p class="text-sm text-muted-foreground">{m.wiki_regenerate_confirm_body()}</p>
		<Dialog.Footer>
			<Button variant="outline" onclick={() => (regenerateConfirmOpen = false)}>{m.common_cancel()}</Button>
			<Button onclick={handleRegenerate}>
				<RefreshCw class="mr-1.5 h-4 w-4" />
				{m.wiki_regenerate_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- Ingest dialog -->
<Dialog.Root bind:open={ingestOpen} onOpenChange={(open) => { if (!open) handleIngestDialogClose(); }}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>{m.wiki_ingest_dialog_title()}</Dialog.Title>
		</Dialog.Header>
		<div class="space-y-3">
			<!-- File upload drop zone -->
			<div
				class="flex cursor-pointer flex-col items-center justify-center rounded-lg border-2 border-dashed border-muted-foreground/30 p-6 transition-colors hover:border-muted-foreground/50"
				onclick={() => fileInput?.click()}
				onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); fileInput?.click(); } }}
				ondragover={handleDropZoneDragOver}
				ondrop={handleDropZoneDrop}
				role="button"
				tabindex="0"
			>
				<Upload class="mb-2 h-8 w-8 text-muted-foreground/50" />
				<p class="text-sm text-muted-foreground">
					{m.wiki_ingest_drop_hint()}
				</p>
				<p class="mt-0.5 text-xs text-muted-foreground/60">
					{INGEST_ACCEPT.split(',').join(', ')}
				</p>
				<input
					bind:this={fileInput}
					type="file"
					accept={INGEST_ACCEPT}
					multiple
					class="hidden"
					onchange={handleFileSelect}
				/>
			</div>

			<!-- Selected file list -->
			{#if ingestFiles.length > 0}
				<div class="max-h-40 overflow-y-auto rounded-md border">
					{#each ingestFiles as file, i (file.name)}
						<div class="flex items-center justify-between px-3 py-1.5 text-sm {i > 0 ? 'border-t' : ''}">
							<span class="truncate text-xs">{file.name}</span>
							<button
								class="ml-2 shrink-0 rounded p-0.5 text-muted-foreground hover:bg-muted hover:text-foreground"
								onclick={() => removeIngestFile(i)}
								disabled={ingesting}
								aria-label="Remove {file.name}"
							>
								<X class="h-3.5 w-3.5" />
							</button>
						</div>
					{/each}
				</div>
			{/if}

			<Button
				class="w-full gap-1.5"
				onclick={handleIngest}
				disabled={ingesting || ingestFiles.length === 0}
			>
				{#if ingesting && ingestProgress}
					<Loader2 class="h-4 w-4 animate-spin" />
					{m.wiki_ingest_progress({ current: ingestProgress.current.toString(), total: ingestProgress.total.toString() })}
				{:else if ingestFiles.length === 0}
					{m.wiki_ingest_no_files()}
				{:else}
					<FileUp class="h-4 w-4" />
					{m.wiki_ingest_file_count({ count: ingestFiles.length.toString(), suffix: ingestFiles.length > 1 ? 's' : '' })}
				{/if}
			</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>

<!-- Change Prompt modal -->
<Dialog.Root bind:open={promptOpen}>
	<Dialog.Content class="sm:max-w-lg">
		<Dialog.Header>
			<Dialog.Title>{m.wiki_prompt_modal_title()}</Dialog.Title>
			<Dialog.Description>{m.wiki_prompt_modal_desc()}</Dialog.Description>
		</Dialog.Header>
		{#if promptLoading}
			<Skeleton class="h-32 w-full" />
		{:else}
			<textarea
				class="min-h-[130px] w-full resize-none rounded-md border bg-background px-3 py-2 text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
				bind:value={promptContent}
			></textarea>
		{/if}
		<Dialog.Footer>
			<Button variant="outline" onclick={() => (promptOpen = false)}>Cancel</Button>
			<Button onclick={handleSavePrompt} disabled={promptSaving || promptLoading || !promptContent.trim()}>
				{#if promptSaving}<Loader2 class="mr-1.5 h-3.5 w-3.5 animate-spin" />{/if}
				{m.wiki_prompt_save()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- Delete All Wiki modal -->
<Dialog.Root bind:open={deleteWikiOpen} onOpenChange={(open) => { if (!open) deleteConfirmText = ''; }}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title class="text-destructive">{m.wiki_delete_modal_title()}</Dialog.Title>
		</Dialog.Header>
		<div class="rounded-lg border border-destructive/30 bg-destructive/5 p-3 text-sm text-destructive">
			<AlertTriangle class="mb-1 h-4 w-4" />
			{m.wiki_delete_modal_warning({ count: wikiStore.pages.length.toString() })}
		</div>
		<div class="space-y-2">
			<p class="text-xs text-muted-foreground">{m.wiki_delete_confirm_placeholder()}</p>
			<input
				type="text"
				class="w-full rounded-md border px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-destructive/50"
				placeholder="DELETE"
				bind:value={deleteConfirmText}
				onkeydown={(e) => { if (e.key === 'Enter' && deleteConfirmText === 'DELETE') handleDeleteWiki(); }}
			/>
		</div>
		<Dialog.Footer>
			<Button variant="outline" onclick={() => (deleteWikiOpen = false)}>Cancel</Button>
			<Button
				variant="destructive"
				onclick={handleDeleteWiki}
				disabled={deleteConfirmText !== 'DELETE' || deletingWiki}
			>
				{#if deletingWiki}<Loader2 class="mr-1.5 h-3.5 w-3.5 animate-spin" />{/if}
				{m.wiki_delete_confirm_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<!-- Delete All Sources dialog -->
<Dialog.Root bind:open={deleteAllSourcesOpen}>
	<Dialog.Content class="sm:max-w-sm">
		<Dialog.Header>
			<Dialog.Title>{m.wiki_sources_delete_all_button()}</Dialog.Title>
		</Dialog.Header>
		<p class="text-sm text-muted-foreground">
			{m.wiki_sources_delete_all_confirm({ count: wikiStore.sources.length.toString() })}
		</p>
		<Dialog.Footer>
			<Button variant="outline" onclick={() => (deleteAllSourcesOpen = false)}>Cancel</Button>
			<Button variant="destructive" onclick={handleDeleteAllSources} disabled={deletingAllSources}>
				{#if deletingAllSources}<Loader2 class="mr-1.5 h-3.5 w-3.5 animate-spin" />{/if}
				{m.wiki_sources_delete_all_button()}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
