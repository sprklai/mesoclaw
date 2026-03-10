<script lang="ts">
	import { cn } from "$lib/utils";
	import { codeVariants } from ".";
	import type { CodeRootProps } from "./types";
	import { useCode } from "./code.svelte.js";
	import { box } from "svelte-toolbelt";

	let {
		ref = $bindable(null),
		variant = "default",
		lang = "typescript",
		code,
		class: className,
		hideLines = false,
		highlight = [],
		children,
		...rest
	}: CodeRootProps = $props();

	const codeState = useCode({
		code: box.with(() => code),
		hideLines: box.with(() => hideLines),
		highlight: box.with(() => highlight),
		lang: box.with(() => lang),
	});
</script>

<div {...rest} bind:this={ref} class={cn(codeVariants({ variant }), className)}>
	<div class="ai-code-wrapper">
		{@html codeState.highlighted}
		{@render children?.()}
	</div>
</div>

<style>
	/* Scoped global styles - only affect elements within .ai-code-wrapper */
	/* Dark mode: check dark class on parent, then scope to wrapper */
	:global(.dark) .ai-code-wrapper :global(.shiki),
	:global(.dark) .ai-code-wrapper :global(.shiki span) {
		color: var(--shiki-dark) !important;
		font-style: var(--shiki-dark-font-style) !important;
		font-weight: var(--shiki-dark-font-weight) !important;
		text-decoration: var(--shiki-dark-text-decoration) !important;
	}

	/* Shiki see: https://shiki.matsu.io/guide/dual-themes#class-based-dark-mode */
	:global(html.dark) .ai-code-wrapper :global(.shiki),
	:global(html.dark) .ai-code-wrapper :global(.shiki span) {
		color: var(--shiki-dark) !important;
		font-style: var(--shiki-dark-font-style) !important;
		font-weight: var(--shiki-dark-font-weight) !important;
		text-decoration: var(--shiki-dark-text-decoration) !important;
	}

	.ai-code-wrapper :global(pre.shiki) {
		overflow-x: auto;
		border-radius: 0.5rem;
		background-color: inherit;
		padding-top: 1rem;
		padding-bottom: 1rem;
		font-size: 0.875rem;
		line-height: 1.25rem;
	}

	.ai-code-wrapper :global(pre.shiki:not([data-code-overflow] *):not([data-code-overflow])) {
		overflow-y: auto;
		max-height: min(100%, 650px);
	}

	.ai-code-wrapper :global(pre.shiki code) {
		display: grid;
		min-width: 100%;
		border-radius: 0;
		border-width: 0;
		background-color: transparent;
		padding: 0;
		overflow-wrap: break-word;
		counter-reset: line;
		box-decoration-break: clone;
	}

	.ai-code-wrapper :global(pre.line-numbers) {
		counter-reset: step;
		counter-increment: step 0;
	}

	.ai-code-wrapper :global(pre.line-numbers .line::before) {
		content: counter(step);
		counter-increment: step;
		display: inline-block;
		width: 1.8rem;
		margin-right: 1.4rem;
		text-align: right;
		color: var(--color-muted-foreground);
	}

	.ai-code-wrapper :global(pre .line.line--highlighted) {
		background-color: var(--color-secondary);
	}

	.ai-code-wrapper :global(pre .line.line--highlighted span) {
		position: relative;
	}

	.ai-code-wrapper :global(pre .line) {
		display: inline-block;
		min-height: 1rem;
		width: 100%;
		padding-left: 1rem;
		padding-right: 1rem;
		padding-top: 0.125rem;
		padding-bottom: 0.125rem;
	}

	.ai-code-wrapper :global(pre.line-numbers .line) {
		padding-left: 0.5rem;
		padding-right: 0.5rem;
	}
</style>
