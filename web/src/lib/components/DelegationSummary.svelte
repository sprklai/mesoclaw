<script lang="ts">
	import type { DelegationRecord } from '$lib/stores/messages.svelte';

	let {
		delegation
	}: {
		delegation: DelegationRecord;
	} = $props();

	function formatTokens(tokens: number): string {
		if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
		if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}k`;
		return `${tokens}`;
	}

	function formatDuration(ms: number): string {
		return `${(ms / 1000).toFixed(1)}s`;
	}

	function statusIcon(status: string): string {
		switch (status) {
			case 'completed': return '\u2713';
			case 'failed': return '\u2717';
			case 'timed_out': return '\u23F1';
			default: return '\u25CF';
		}
	}

	function statusColor(status: string): string {
		switch (status) {
			case 'completed': return 'text-green-500';
			case 'failed': return 'text-red-500';
			case 'timed_out': return 'text-red-500';
			default: return 'text-muted-foreground';
		}
	}
</script>

<div class="rounded-lg border border-border bg-muted/30 p-3 font-mono text-sm">
	<div class="font-semibold text-cyan-500">
		Delegated to {delegation.agents.length} agent{delegation.agents.length !== 1 ? 's' : ''} ({formatDuration(delegation.total_duration_ms)}) &middot; {formatTokens(delegation.total_tokens)} tokens
	</div>
	{#each delegation.agents as agent, i}
		{@const isLast = i === delegation.agents.length - 1}
		{@const connector = isLast ? '\u2514\u2500\u2500' : '\u251C\u2500\u2500'}
		{@const subConnector = isLast ? '    ' : '\u2502   '}
		<div class="mt-1">
			<span class="text-muted-foreground">{connector} </span>
			<span class={statusColor(agent.status)}>
				{statusIcon(agent.status)} {agent.description}
			</span>
			<span class="text-muted-foreground">
				&middot; {agent.tool_uses} tool{agent.tool_uses !== 1 ? 's' : ''} &middot; {formatTokens(agent.tokens_used)} tokens &middot; {formatDuration(agent.duration_ms)}
			</span>
		</div>
		{#if agent.error}
			<div class="text-red-400">
				<span class="text-muted-foreground">{subConnector}\u2514\u2500\u2500 </span>
				{agent.error}
			</div>
		{/if}
	{/each}
</div>
