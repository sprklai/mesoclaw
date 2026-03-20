<script lang="ts">
	import { onDestroy } from 'svelte';
	import type { DelegationState } from '$lib/stores/delegation.svelte';

	let {
		delegation
	}: {
		delegation: DelegationState;
	} = $props();

	let elapsed = $state(0);

	const interval = setInterval(() => {
		elapsed = Math.floor((Date.now() - delegation.startedAt) / 1000);
	}, 1000);
	onDestroy(() => clearInterval(interval));

	function formatTokens(tokens: number): string {
		if (tokens >= 1_000_000) return `${(tokens / 1_000_000).toFixed(1)}M`;
		if (tokens >= 1_000) return `${(tokens / 1_000).toFixed(1)}k`;
		return `${tokens}`;
	}
</script>

<div class="rounded-lg border border-border bg-muted/30 p-3 font-mono text-sm">
	<div class="font-semibold text-cyan-500">
		Running {delegation.agents.length} agents... ({elapsed}s)
	</div>
	{#each delegation.agents as agent, i}
		{@const isLast = i === delegation.agents.length - 1}
		{@const connector = isLast ? '\u2514\u2500' : '\u251C\u2500'}
		{@const subConnector = isLast ? '   ' : '\u2502  '}
		<div class="mt-1">
			<span class="text-muted-foreground">{connector} </span>
			<span
				class={agent.status === 'completed'
					? 'text-green-500'
					: agent.status === 'failed'
						? 'text-red-500'
						: ''}
			>
				{#if agent.status === 'completed'}&#10003; {/if}
				{#if agent.status === 'failed'}&#10007; {/if}
				{agent.description}
			</span>
			<span class="text-muted-foreground">
				{#if agent.status === 'completed' && agent.durationMs}
					&middot; completed ({(agent.durationMs / 1000).toFixed(1)}s) &middot;
				{/if}
				{#if agent.status === 'failed'} &middot; failed &middot; {/if}
				{agent.toolUses} tool uses &middot; {formatTokens(agent.tokensUsed)} tokens
			</span>
		</div>
		{#if agent.currentActivity || agent.status === 'pending'}
			<div class="text-muted-foreground">
				<span>{subConnector}\u2514 </span>
				{agent.currentActivity || 'Pending...'}
			</div>
		{/if}
	{/each}
</div>
