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
	const iconName = $derived(definition?.icon ?? 'PlayCircle');
</script>

<!-- Trigger: no left input — only right output. Dashed violet border marks workflow start. -->
<div
	class="w-[110px] rounded-2xl border-2 border-dashed border-violet-500/70 bg-card text-card-foreground shadow-md flex flex-col items-center pt-5 pb-4 px-3 gap-2 transition-shadow
		{isRunning ? 'ring-2 ring-yellow-400 shadow-yellow-400/20' : 'hover:shadow-xl hover:border-violet-500'}"
>
	<!-- Floating "Trigger" pill at top -->
	<div class="absolute -top-3 left-1/2 -translate-x-1/2">
		<span class="bg-violet-500 text-white text-[9px] font-bold px-2 py-0.5 rounded-full tracking-wide uppercase whitespace-nowrap">
			Trigger
		</span>
	</div>

	<!-- Large centred icon -->
	<div class="w-12 h-12 rounded-xl flex items-center justify-center bg-violet-500/15">
		{#if isRunning}
			<Loader2 class="h-6 w-6 animate-spin text-yellow-400" />
		{:else}
			<NodeIcon name={iconName} class="h-6 w-6 text-violet-500" />
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

	<Handle
		type="source"
		position={Position.Right}
		class="!bg-card !border-2 !border-violet-500 !w-3.5 !h-3.5 !rounded-full"
	/>
</div>
