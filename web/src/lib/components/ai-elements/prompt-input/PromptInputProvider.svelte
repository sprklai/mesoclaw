<script lang="ts">
	import { untrack } from "svelte";
	import { PromptInputController, setPromptInputProvider } from "./attachments-context.svelte.js";

	interface Props {
		initialInput?: string;
		accept?: string;
		multiple?: boolean;
		children?: import("svelte").Snippet;
	}

	let { initialInput = "", accept, multiple = true, children }: Props = $props();

	// Intentionally capture initial values for one-time controller setup
	let controller = untrack(() => new PromptInputController(initialInput, accept, multiple));

	setPromptInputProvider(controller);
</script>

{#if children}
	{@render children()}
{/if}
