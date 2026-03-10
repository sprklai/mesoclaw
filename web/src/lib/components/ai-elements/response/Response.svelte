<script lang="ts">
	import { Streamdown, type StreamdownProps } from "svelte-streamdown";
	import Code from "svelte-streamdown/code"; // Shiki syntax highlighting
	import { cn } from "$lib/utils";
	import { themeStore } from "$lib/stores/theme.svelte";
	import { shikiThemes } from "$lib/components/ai-elements/code/shiki";

	type Props = StreamdownProps & {
		class?: string;
	};

	let { class: className, ...restProps }: Props = $props();
	let currentTheme = $derived(
		themeStore.isDark ? "github-dark-default" : "github-light-default"
	);
</script>

<Streamdown
	class={cn("size-full [&>*:first-child]:mt-0 [&>*:last-child]:mb-0", className)}
	shikiTheme={currentTheme}
	baseTheme="shadcn"
	components={{ code: Code }}
	shikiThemes={shikiThemes}
	{...restProps}
/>
