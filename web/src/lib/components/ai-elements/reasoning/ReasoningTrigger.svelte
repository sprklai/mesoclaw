<script lang="ts">
	import { cn } from "$lib/utils";
	import { CollapsibleTrigger } from "$lib/components/ui/collapsible/index.js";
	import { getReasoningContext } from "./reasoning-context.svelte.js";
	import BrainIcon from "@lucide/svelte/icons/brain";
	import ChevronDownIcon from "@lucide/svelte/icons/chevron-down";
	import * as m from '$lib/paraglide/messages';

	interface Props {
		class?: string;
		children?: import("svelte").Snippet;
	}

	let { class: className = "", children, ...props }: Props = $props();

	let reasoningContext = getReasoningContext();

	let getThinkingMessage = $derived.by(() => {
		let { isStreaming, duration } = reasoningContext;

		if (isStreaming || duration === 0) {
			return m.reasoning_thinking();
		}
		if (duration === undefined) {
			return m.reasoning_thought_few_seconds();
		}
		return m.reasoning_thought_duration({ duration: String(duration) });
	});
</script>

<CollapsibleTrigger
	class={cn(
		"text-muted-foreground hover:text-foreground flex w-full items-center gap-2 text-sm transition-colors",
		className
	)}
	{...props}
>
	{#if children}
		{@render children()}
	{:else}
		<BrainIcon class="size-4" />
		<p>{getThinkingMessage}</p>
		<ChevronDownIcon
			class={cn(
				"size-4 transition-transform",
				reasoningContext.isOpen ? "rotate-180" : "rotate-0"
			)}
		/>
	{/if}
</CollapsibleTrigger>
