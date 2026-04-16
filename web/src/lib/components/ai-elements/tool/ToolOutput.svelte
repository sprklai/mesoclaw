<script lang="ts">
	import { cn } from "$lib/utils";
	import * as m from "$lib/paraglide/messages";
	import * as Code from "$lib/components/ai-elements/code/index.js";
	import type { Snippet } from "svelte";
	import type { SupportedLanguage } from "../code/shiki";

	interface ToolOutputProps {
		class?: string;
		output?: any;
		errorText?: string;
		children?: Snippet;
		[key: string]: any;
	}

	let {
		class: className = "",
		output,
		errorText,
		children,
		...restProps
	}: ToolOutputProps = $props();

	let shouldRender = $derived.by(() => {
		return !!(output || errorText);
	});
	type OutputComp = {
		type: "code" | "text";
		content: string;
		language: SupportedLanguage;
	};

	let outputComponent: OutputComp | null = $derived.by(() => {
		if (!output) return null;

		if (typeof output === "object") {
			return {
				type: "code",
				content: JSON.stringify(output, null, 2),
				language: "json",
			};
		} else if (typeof output === "string") {
			// Tool results arrive as serialized ToolResult: {"output":"...","success":bool}
			// Try to unwrap the outer envelope first.
			let inner: string = output;
			try {
				const parsed = JSON.parse(output);
				if (parsed && typeof parsed.output === "string") {
					inner = parsed.output;
				} else {
					// Outer object has no .output field — pretty-print it as JSON
					return { type: "code", content: JSON.stringify(parsed, null, 2), language: "json" };
				}
			} catch {
				// Not JSON at all — fall through with inner = output
			}

			// Try to parse inner content as JSON (e.g. wiki search results)
			try {
				const innerParsed = JSON.parse(inner);
				return {
					type: "code",
					content: JSON.stringify(innerParsed, null, 2),
					language: "json",
				};
			} catch {
				// Inner content is plain text (markdown, prose, etc.)
				return { type: "text", content: inner, language: "text" };
			}
		} else {
			return {
				type: "text",
				content: String(output),
				language: "text",
			};
		}
	});

	let id = $props.id();
</script>

{#if shouldRender}
	<div {id} class={cn("space-y-2 p-4", className)} {...restProps}>
		<h4 class="text-muted-foreground text-xs font-medium tracking-wide uppercase">
			{errorText ? m.tool_output_error_heading() : m.tool_output_result_heading()}
		</h4>
		<div
			class={cn(
				"rounded-md text-xs [&_table]:w-full",
				outputComponent?.type === "text" ? "overflow-hidden" : "overflow-x-auto",
				errorText ? "bg-destructive/10 text-destructive" : "bg-muted/50 text-foreground"
			)}
		>
			{#if errorText}
				<div class="p-3 whitespace-pre-wrap break-words">{errorText}</div>
			{:else if outputComponent}
				{#if outputComponent.type === "code"}
					<Code.Root
						code={outputComponent.content}
						lang={outputComponent.language}
						hideLines
					>
						<Code.CopyButton />
					</Code.Root>
				{:else}
					<div class="p-3 whitespace-pre-wrap break-words">{outputComponent.content}</div>
				{/if}
			{/if}
		</div>
	</div>
{/if}
