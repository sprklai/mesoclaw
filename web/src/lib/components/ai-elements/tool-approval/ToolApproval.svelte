<script lang="ts">
	import type { PendingApproval } from '$lib/stores/approvals.svelte';
	import { ShieldAlert, Check, ShieldCheck, X } from '@lucide/svelte';
	import * as m from '$lib/paraglide/messages';

	let {
		approval,
		onRespond
	}: {
		approval: PendingApproval;
		onRespond: (approvalId: string, decision: 'approve' | 'approve_always' | 'deny') => void;
	} = $props();

	let remainingSecs = $state(0);
	let expired = $state(false);

	$effect(() => {
		remainingSecs = approval.timeoutSecs;
		const startedAt = approval.requestedAt;
		const timeout = approval.timeoutSecs;
		const interval = setInterval(() => {
			const elapsed = Math.floor((Date.now() - startedAt) / 1000);
			remainingSecs = Math.max(0, timeout - elapsed);
			if (remainingSecs <= 0) {
				expired = true;
				clearInterval(interval);
			}
		}, 1000);

		return () => clearInterval(interval);
	});

	const riskColor = $derived(
		approval.riskLevel === 'high'
			? 'text-red-500'
			: approval.riskLevel === 'medium'
				? 'text-amber-500'
				: 'text-blue-500'
	);

	const riskBg = $derived(
		approval.riskLevel === 'high'
			? 'bg-red-500/10 border-red-500/30'
			: approval.riskLevel === 'medium'
				? 'bg-amber-500/10 border-amber-500/30'
				: 'bg-blue-500/10 border-blue-500/30'
	);

	const progressPct = $derived(
		approval.timeoutSecs > 0 ? (remainingSecs / approval.timeoutSecs) * 100 : 0
	);
</script>

<div class="rounded-lg border {riskBg} p-3 space-y-2">
	<div class="flex items-center gap-2 text-sm font-medium {riskColor}">
		<ShieldAlert class="size-4" />
		<span>{m.tool_approval_needs_approval({ toolName: approval.toolName })}</span>
		<span class="ml-auto text-xs text-muted-foreground">{remainingSecs}s</span>
	</div>

	<div class="rounded bg-muted/50 px-2 py-1 font-mono text-xs text-foreground">
		{approval.argsSummary}
	</div>

	<div class="text-xs text-muted-foreground">{approval.reason}</div>

	<!-- Countdown bar -->
	<div class="h-1 w-full rounded-full bg-muted overflow-hidden">
		<div
			class="h-full rounded-full transition-all duration-1000 ease-linear {approval.riskLevel === 'high' ? 'bg-red-500' : 'bg-amber-500'}"
			style="width: {progressPct}%"
		></div>
	</div>

	{#if !expired}
		<div class="flex gap-2">
			<button
				class="inline-flex items-center gap-1 rounded-md bg-green-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-green-700"
				onclick={() => onRespond(approval.approvalId, 'approve')}
			>
				<Check class="size-3" />
				{m.tool_approval_approve()}
			</button>
			<button
				class="inline-flex items-center gap-1 rounded-md bg-blue-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-700"
				onclick={() => onRespond(approval.approvalId, 'approve_always')}
			>
				<ShieldCheck class="size-3" />
				{m.tool_approval_always_allow()}
			</button>
			<button
				class="inline-flex items-center gap-1 rounded-md bg-red-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-red-700"
				onclick={() => onRespond(approval.approvalId, 'deny')}
			>
				<X class="size-3" />
				{m.tool_approval_deny()}
			</button>
		</div>
	{:else}
		<div class="text-xs text-muted-foreground">{m.tool_approval_timed_out()}</div>
	{/if}
</div>
