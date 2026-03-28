<script lang="ts">
	import type { DelegationRecord } from '$lib/stores/messages.svelte';
	import * as m from '$lib/paraglide/messages';

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
			case 'Completed': return '\u2713';
			case 'Failed': return '\u2717';
			case 'TimedOut': return '\u23F1';
			default: return '\u25CF';
		}
	}

	function statusColor(status: string): string {
		switch (status) {
			case 'Completed': return 'text-green-500';
			case 'Failed': return 'text-red-500';
			case 'TimedOut': return 'text-red-500';
			default: return 'text-muted-foreground';
		}
	}
</script>

<div class="rounded-lg border border-border bg-muted/30 p-3 font-mono text-sm">
	<div class="font-semibold text-cyan-500">
		{m.delegation_summary_header({ count: String(delegation.agents.length), duration: formatDuration(delegation.total_duration_ms), tokens: formatTokens(delegation.total_tokens) })}
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
				&middot; {m.delegation_summary_agent_stats({ toolUses: String(agent.tool_uses), tokens: formatTokens(agent.tokens_used), duration: formatDuration(agent.duration_ms) })}
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
