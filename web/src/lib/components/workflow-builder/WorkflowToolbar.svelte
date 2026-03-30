<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { builderStore } from '$lib/stores/workflow-builder.svelte';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
	import Save from '@lucide/svelte/icons/save';
	import Play from '@lucide/svelte/icons/play';
	import Download from '@lucide/svelte/icons/download';
	import Code from '@lucide/svelte/icons/code';
	import LayoutGrid from '@lucide/svelte/icons/layout-grid';
	import Loader2 from '@lucide/svelte/icons/loader-2';
	import Circle from '@lucide/svelte/icons/circle';
	import { t } from './i18n-utils';

	let {
		onSave,
		onRun,
		onExport,
		onBack
	}: {
		onSave: () => void;
		onRun: () => void;
		onExport: () => void;
		onBack: () => void;
	} = $props();


</script>

<div class="flex items-center gap-2 px-3 py-2 border-b bg-muted/30">
	<Button variant="ghost" size="icon" onclick={onBack} title={t('wb_toolbar_back')}>
		<ArrowLeft class="h-4 w-4" />
	</Button>

	<div class="flex-1 min-w-0">
		<input
			type="text"
			value={builderStore.workflowName}
			oninput={(e) => builderStore.updateMeta({ name: (e.target as HTMLInputElement).value })}
			placeholder={t('wb_field_name')}
			class="bg-transparent border-none text-sm font-medium focus:outline-none focus:ring-0 w-full"
		/>
	</div>

	{#if builderStore.isDirty}
		<span class="flex items-center gap-1 text-[10px] text-yellow-500">
			<Circle class="h-2 w-2 fill-current" />
			{t('wb_toolbar_unsaved')}
		</span>
	{/if}

	{#if builderStore.isRunning}
		<span class="flex items-center gap-1 text-xs text-yellow-500">
			<Loader2 class="h-3 w-3 animate-spin" />
			{t('wb_running_indicator')}
		</span>
	{/if}

	<div class="flex items-center gap-1 border rounded-md">
		<Button
			variant={builderStore.viewMode === 'visual' ? 'secondary' : 'ghost'}
			size="sm"
			onclick={() => builderStore.setViewMode('visual')}
			class="rounded-r-none h-7 text-xs"
		>
			<LayoutGrid class="h-3 w-3 mr-1" />
			{t('wb_toolbar_view_visual')}
		</Button>
		<Button
			variant={builderStore.viewMode === 'code' ? 'secondary' : 'ghost'}
			size="sm"
			onclick={() => builderStore.setViewMode('code')}
			class="rounded-l-none h-7 text-xs"
		>
			<Code class="h-3 w-3 mr-1" />
			{t('wb_toolbar_view_code')}
		</Button>
	</div>

	<Button variant="ghost" size="sm" onclick={onExport} title={t('wb_toolbar_export')}>
		<Download class="h-4 w-4" />
	</Button>

	<Button variant="ghost" size="sm" onclick={onRun} disabled={builderStore.isRunning}>
		{#if builderStore.isRunning}
			<Loader2 class="h-4 w-4 animate-spin" />
		{:else}
			<Play class="h-4 w-4" />
		{/if}
	</Button>

	<Button variant="default" size="sm" onclick={onSave} disabled={!builderStore.isDirty}>
		<Save class="h-4 w-4 mr-1" />
		{t('wb_toolbar_save')}
	</Button>
</div>
