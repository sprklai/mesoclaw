<script lang="ts">
	import { nodeRegistry, type NodeDefinition, type NodeCategory } from './node-registry';
	import { capabilitiesStore } from '$lib/stores/capabilities.svelte';
	import { builderStore } from '$lib/stores/workflow-builder.svelte';
	import { generateStepName } from './graph-utils';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import { t } from './i18n-utils';

	const CATEGORY_LABELS: Record<NodeCategory, string> = {
		triggers: 'wb_cat_triggers',
		ai: 'wb_cat_ai',
		search: 'wb_cat_search',
		system: 'wb_cat_system',
		files: 'wb_cat_files',
		memory: 'wb_cat_memory',
		channels: 'wb_cat_channels',
		config: 'wb_cat_config',
		schedule: 'wb_cat_schedule',
		flow: 'wb_cat_flow'
	};

	const CATEGORY_ORDER: NodeCategory[] = [
		'triggers', 'ai', 'search', 'system', 'files', 'memory', 'channels', 'config', 'schedule', 'flow'
	];

	let searchQuery = $state('');
	let expandedCategories = $state<Set<NodeCategory>>(new Set(CATEGORY_ORDER));

	const capabilities = $derived(
		Object.entries(capabilitiesStore.capabilities)
			.filter(([, v]) => v)
			.map(([k]) => k)
	);

	const visibleDefs = $derived(
		nodeRegistry.getVisible(capabilities).filter(d => {
			if (!searchQuery.trim()) return true;
			const q = searchQuery.toLowerCase();
			return t(d.label).toLowerCase().includes(q) || t(d.description).toLowerCase().includes(q);
		})
	);

	const grouped = $derived.by(() => {
		const map = new Map<NodeCategory, NodeDefinition[]>();
		for (const d of visibleDefs) {
			const arr = map.get(d.category) || [];
			arr.push(d);
			map.set(d.category, arr);
		}
		return map;
	});

	function toggleCategory(cat: NodeCategory) {
		const next = new Set(expandedCategories);
		if (next.has(cat)) next.delete(cat);
		else next.add(cat);
		expandedCategories = next;
	}

	function handleDragStart(e: DragEvent, def: NodeDefinition) {
		if (!e.dataTransfer) return;
		e.dataTransfer.setData('application/workflow-node', def.type);
		e.dataTransfer.effectAllowed = 'move';
	}
</script>

<div class="flex flex-col h-full border-r bg-muted/30">
	<div class="p-2 border-b">
		<h3 class="text-xs font-semibold mb-2">{t('wb_palette_title')}</h3>
		<input
			type="text"
			bind:value={searchQuery}
			placeholder={t('wb_palette_search')}
			class="w-full rounded-md border bg-background text-foreground px-2 py-1 text-xs"
		/>
	</div>

	<div class="flex-1 overflow-y-auto">
		{#each CATEGORY_ORDER as cat}
			{@const defs = grouped.get(cat)}
			{#if defs && defs.length > 0}
				<div class="border-b">
					<button
						class="w-full flex items-center gap-1 px-2 py-1.5 text-xs font-medium text-muted-foreground hover:bg-muted/50 cursor-pointer"
						onclick={() => toggleCategory(cat)}
					>
						{#if expandedCategories.has(cat)}
							<ChevronDown class="h-3 w-3" />
						{:else}
							<ChevronRight class="h-3 w-3" />
						{/if}
						{t(CATEGORY_LABELS[cat])}
						<span class="ml-auto text-[10px]">{defs.length}</span>
					</button>

					{#if expandedCategories.has(cat)}
						<div class="px-1 pb-1 space-y-0.5">
							{#each defs as def (def.type)}
								<div
									draggable="true"
									ondragstart={(e) => handleDragStart(e, def)}
									class="flex items-center gap-2 px-2 py-1.5 rounded text-xs cursor-grab hover:bg-muted active:cursor-grabbing"
									title={t(def.description)}
								>
									<span class="truncate">{t(def.label)}</span>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			{/if}
		{/each}
	</div>
</div>
