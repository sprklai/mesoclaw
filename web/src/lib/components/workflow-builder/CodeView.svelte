<script lang="ts">
	import { t } from './i18n-utils';
	import Info from '@lucide/svelte/icons/info';

	let {
		value
	}: {
		value: string;
	} = $props();
</script>

<div class="flex flex-col h-full">
	<div class="flex items-center gap-1.5 px-3 py-1.5 text-xs text-muted-foreground bg-muted/50 border-b">
		<Info class="h-3 w-3 shrink-0" />
		{t('wb_code_readonly_notice')}
	</div>
	<textarea
		{value}
		readonly
		class="flex-1 w-full bg-background text-foreground font-mono text-sm p-4 resize-none focus:outline-none cursor-default"
		spellcheck="false"
	></textarea>
</div>
