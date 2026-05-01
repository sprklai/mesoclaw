<script lang="ts">
	import { Handle, Position } from '@xyflow/svelte';
	import { nodeRegistry } from '../node-registry';
	import { t } from '../i18n-utils';
	import NodeIcon from './NodeIcon.svelte';
	import Loader2 from '@lucide/svelte/icons/loader-2';

	let { data }: { data: Record<string, unknown>; id: string } = $props();

	const definition = $derived(nodeRegistry.get(data.definitionType as string));
	const label = $derived(definition ? t(definition.label) : (data.definitionType as string));
	const isRunning = $derived(data.isRunning === true);
	const iconName = $derived(definition?.icon ?? 'GitFork');
</script>

<!--
  Condition: left input, two right outputs (true = upper-right, false = lower-right).
  Branch labels are pinned to the right edge near their handles.
-->
<div
	class="w-[110px] rounded-2xl border-2 border-amber-500/70 bg-card text-card-foreground shadow-md flex flex-col items-center py-4 px-3 gap-2 transition-shadow
		{isRunning ? 'ring-2 ring-yellow-400 shadow-yellow-400/20' : 'hover:shadow-xl hover:border-amber-500'}"
>
	<Handle
		type="target"
		position={Position.Left}
		class="!bg-card !border-2 !border-amber-500 !w-3.5 !h-3.5 !rounded-full"
	/>

	<!-- Large centred icon -->
	<div class="w-12 h-12 rounded-xl flex items-center justify-center bg-amber-500/15">
		{#if isRunning}
			<Loader2 class="h-6 w-6 animate-spin text-yellow-400" />
		{:else}
			<NodeIcon name={iconName} class="h-6 w-6 text-amber-500" />
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

	<div class="flex justify-between px-3 py-1 text-[9px] text-muted-foreground">
		<span class="text-green-500">{t('wb_handle_true')}</span>
		<span class="text-red-400">{t('wb_handle_false')}</span>
	</div>

	<!-- Branch indicators aligned with handles -->
	<div class="w-full flex flex-col items-end gap-3 pr-0.5 text-[9px] font-semibold">
		<span class="text-emerald-500">true ●</span>
		<span class="text-red-400">false ●</span>
	</div>

	<!-- true → upper-right, false → lower-right -->
	<Handle
		type="source"
		position={Position.Right}
		id="true"
		class="!bg-card !border-2 !border-emerald-500 !w-3.5 !h-3.5 !rounded-full"
		style="top: 35%;"
	/>
	<Handle
		type="source"
		position={Position.Right}
		id="false"
		class="!bg-card !border-2 !border-red-400 !w-3.5 !h-3.5 !rounded-full"
		style="top: 70%;"
	/>
</div>
