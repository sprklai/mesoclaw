<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte';
	import { nodeRegistry } from '../node-registry';
	import { t } from '../i18n-utils';
	import { getCategoryStyle } from './node-colors';
	import NodeIcon from './NodeIcon.svelte';
	import Loader2 from '@lucide/svelte/icons/loader-2';
	import TriangleAlert from '@lucide/svelte/icons/triangle-alert';

	let { data }: { data: Record<string, unknown>; id: string } = $props();

	const definition = $derived(nodeRegistry.get(data.definitionType as string));
	const label = $derived(definition ? t(definition.label) : (data.definitionType as string));
	const isRunning = $derived(data.isRunning === true);
	const category = $derived(definition?.category ?? 'system');
	const iconName = $derived(definition?.icon ?? 'Zap');
	const style = $derived(getCategoryStyle(category));
	// Show warning badge if definition is missing or marked hidden (unsupported in backend)
	const isUnsupported = $derived(!definition || definition.hidden === true);
</script>

<!-- Input on left, output on right — n8n style square card -->
<div
	class="w-[110px] rounded-2xl border-2 bg-card text-card-foreground shadow-md flex flex-col items-center py-4 px-3 gap-2 transition-shadow
		{isRunning ? 'ring-2 ring-yellow-400 shadow-yellow-400/20' : 'hover:shadow-xl'}
		{isUnsupported ? 'border-orange-400/60' : style.handleBorder.replace('!border-', 'border-')}"
>
	<Handle
		type="target"
		position={Position.Left}
		class="!bg-card !border-2 !w-3.5 !h-3.5 !rounded-full {style.handleBorder}"
	/>

	<!-- Large centred icon -->
	<div class="w-12 h-12 rounded-xl flex items-center justify-center {style.iconBg}">
		{#if isRunning}
			<Loader2 class="h-6 w-6 animate-spin text-yellow-400" />
		{:else}
			<NodeIcon name={iconName} class="h-6 w-6 {style.iconText}" />
		{/if}
	</div>

	<!-- Label + step name -->
	<div class="w-full text-center">
		<div class="text-[11px] font-semibold leading-tight truncate">{label}</div>
		{#if data.stepName}
			<div class="text-[9px] text-muted-foreground truncate mt-0.5">
				{data.stepName as string}
			</div>
		{/if}
	</div>

	<!-- Unsupported warning badge -->
	{#if isUnsupported}
		<div class="flex items-center gap-1 px-1.5 py-0.5 rounded bg-orange-400/15 text-orange-400">
			<TriangleAlert class="h-3 w-3 shrink-0" />
			<span class="text-[9px] font-medium leading-none">Not supported</span>
		</div>
	{/if}

	<Handle
		type="source"
		position={Position.Right}
		class="!bg-card !border-2 !w-3.5 !h-3.5 !rounded-full {style.handleBorder}"
	/>
</div>
