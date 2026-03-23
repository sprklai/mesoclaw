<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { memoryStore } from '$lib/stores/memory.svelte';
	import { schedulerStore } from '$lib/stores/scheduler.svelte';
	import { workflowsStore } from '$lib/stores/workflows.svelte';
	import { channelsStore } from '$lib/stores/channels.svelte';
	import { inboxStore } from '$lib/stores/inbox.svelte';
	import { toast } from 'svelte-sonner';
	import { goto } from '$app/navigation';
	import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
	import Radio from '@lucide/svelte/icons/radio';
	import Brain from '@lucide/svelte/icons/brain';
	import Clock from '@lucide/svelte/icons/clock';
	import GitBranch from '@lucide/svelte/icons/git-branch';

	let loading = $state(true);
	let creating = $state(false);

	$effect(() => {
		loadDashboardData();
	});

	async function loadDashboardData() {
		loading = true;
		try {
			await Promise.allSettled([
				sessionsStore.load(),
				memoryStore.loadAll(),
				schedulerStore.load(),
				workflowsStore.load(),
				channelsStore.load(),
				inboxStore.load(),
			]);
		} finally {
			loading = false;
		}
	}

	async function handleNewChat(e: Event) {
		e.stopPropagation();
		if (creating) return;
		creating = true;
		try {
			const session = await sessionsStore.create('New Chat');
			goto(`/chat/${session.id}`);
		} catch {
			toast.error('Failed to create chat session');
		} finally {
			creating = false;
		}
	}

	function channelStats() {
		const channels = channelsStore.channels;
		const active = channels.filter((c) => c.connected).length;
		return { active, total: channels.length };
	}

	function scheduleStats() {
		const jobs = schedulerStore.jobs;
		let cron = 0;
		let interval = 0;
		let oneTime = 0;
		let enabled = 0;
		for (const job of jobs) {
			if (job.enabled) enabled++;
			if (job.schedule.type === 'cron') cron++;
			else if (job.schedule.type === 'interval') interval++;
			else oneTime++;
		}
		return { total: jobs.length, cron, interval, oneTime, enabled };
	}

	function workflowStats() {
		const wfs = workflowsStore.workflows;
		let running = 0;
		for (const wf of wfs) {
			if (workflowsStore.isRunning(wf.id)) running++;
		}
		return { total: wfs.length, running };
	}
</script>

<div class="mx-auto max-w-4xl space-y-6 p-4">
	<div class="space-y-1 text-center">
		<h1 class="text-3xl font-bold">Zenii</h1>
		<p class="text-muted-foreground">Your private AI backend</p>
	</div>

	<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
		<!-- Chat Card -->
		<Card.Root
			class="cursor-pointer transition-colors hover:bg-accent/50"
			onclick={() => goto('/chat')}
		>
			<Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
				<div class="flex items-center gap-2">
					<MessageSquarePlus class="h-5 w-5 text-muted-foreground" />
					<Card.Title class="text-base font-semibold">Chat</Card.Title>
				</div>
				<Button
					size="sm"
					onclick={handleNewChat}
					disabled={creating}
					class="h-7 gap-1 px-2 text-xs"
				>
					<MessageSquarePlus class="h-3.5 w-3.5" />
					New Chat
				</Button>
			</Card.Header>
			<Card.Content>
				{#if loading}
					<Skeleton class="mb-2 h-4 w-24" />
					<div class="space-y-2">
						<Skeleton class="h-8 w-full" />
						<Skeleton class="h-8 w-full" />
						<Skeleton class="h-8 w-full" />
					</div>
				{:else}
					<p class="mb-2 text-xs text-muted-foreground">
						{sessionsStore.sessions.length} session{sessionsStore.sessions.length !== 1 ? 's' : ''}
					</p>
					{#if sessionsStore.sessions.length > 0}
						<div class="space-y-1.5">
							{#each sessionsStore.sessions.slice(0, 3) as session (session.id)}
								<div
									class="flex items-center justify-between rounded-md bg-muted/50 px-3 py-1.5"
								>
									<span class="truncate text-sm">{session.title}</span>
									<span class="ml-2 shrink-0 text-xs text-muted-foreground">
										{new Date(session.created_at).toLocaleDateString()}
									</span>
								</div>
							{/each}
						</div>
					{:else}
						<p class="text-sm text-muted-foreground">No chats yet</p>
					{/if}
				{/if}
			</Card.Content>
		</Card.Root>

		<!-- Channels Card -->
		<Card.Root
			class="cursor-pointer transition-colors hover:bg-accent/50"
			onclick={() => goto('/channels')}
		>
			<Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
				<div class="flex items-center gap-2">
					<Radio class="h-5 w-5 text-muted-foreground" />
					<Card.Title class="text-base font-semibold">Channels</Card.Title>
				</div>
				{#if !loading && channelStats().active > 0}
					<span class="flex items-center gap-1.5 text-xs text-muted-foreground">
						<span class="h-2 w-2 rounded-full bg-green-500"></span>
						Live
					</span>
				{/if}
			</Card.Header>
			<Card.Content>
				{#if loading}
					<Skeleton class="mb-2 h-4 w-32" />
					<Skeleton class="h-6 w-40" />
				{:else}
					{@const stats = channelStats()}
					<p class="mb-2 text-xs text-muted-foreground">
						{stats.active} active / {stats.total} total
					</p>
					<div class="mb-2 flex flex-wrap gap-1.5">
						{#each channelsStore.channels as channel (channel.id)}
							<Badge variant={channel.connected ? 'default' : 'secondary'} class="text-xs">
								{channel.name}
							</Badge>
						{/each}
						{#if channelsStore.channels.length === 0}
							<span class="text-sm text-muted-foreground">No channels configured</span>
						{/if}
					</div>
					{#if inboxStore.totalUnread > 0}
						<p class="text-xs text-muted-foreground">
							{inboxStore.totalUnread} unread conversation{inboxStore.totalUnread !== 1 ? 's' : ''}
						</p>
					{/if}
				{/if}
			</Card.Content>
		</Card.Root>

		<!-- Memory Card -->
		<Card.Root
			class="cursor-pointer transition-colors hover:bg-accent/50"
			onclick={() => goto('/memory')}
		>
			<Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
				<div class="flex items-center gap-2">
					<Brain class="h-5 w-5 text-muted-foreground" />
					<Card.Title class="text-base font-semibold">Memory</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				{#if loading}
					<Skeleton class="h-4 w-24" />
					<div class="mt-3 flex gap-6">
						<Skeleton class="h-10 w-16" />
						<Skeleton class="h-10 w-16" />
					</div>
				{:else}
					<p class="text-xs text-muted-foreground">
						{memoryStore.entries.length + memoryStore.observations.length} total entries
					</p>
					<div class="mt-3 flex gap-6">
						<div class="text-center">
							<div class="text-2xl font-bold text-blue-500">
								{memoryStore.observations.length}
							</div>
							<div class="text-xs text-muted-foreground">Learned</div>
						</div>
						<div class="text-center">
							<div class="text-2xl font-bold text-orange-500">
								{memoryStore.entries.length}
							</div>
							<div class="text-xs text-muted-foreground">Saved</div>
						</div>
					</div>
				{/if}
			</Card.Content>
		</Card.Root>

		<!-- Schedule Card -->
		<Card.Root
			class="cursor-pointer transition-colors hover:bg-accent/50"
			onclick={() => goto('/schedule')}
		>
			<Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
				<div class="flex items-center gap-2">
					<Clock class="h-5 w-5 text-muted-foreground" />
					<Card.Title class="text-base font-semibold">Schedule</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				{#if loading}
					<Skeleton class="h-4 w-20" />
					<div class="mt-3 flex gap-6">
						<Skeleton class="h-10 w-14" />
						<Skeleton class="h-10 w-14" />
						<Skeleton class="h-10 w-14" />
					</div>
				{:else}
					{@const stats = scheduleStats()}
					<p class="text-xs text-muted-foreground">
						{stats.total} job{stats.total !== 1 ? 's' : ''} &middot; {stats.enabled} enabled
					</p>
					<div class="mt-3 flex gap-6">
						<div class="text-center">
							<div class="text-2xl font-bold text-green-500">{stats.cron}</div>
							<div class="text-xs text-muted-foreground">Cron</div>
						</div>
						<div class="text-center">
							<div class="text-2xl font-bold text-blue-500">{stats.interval}</div>
							<div class="text-xs text-muted-foreground">Interval</div>
						</div>
						<div class="text-center">
							<div class="text-2xl font-bold text-zinc-400">{stats.oneTime}</div>
							<div class="text-xs text-muted-foreground">One-time</div>
						</div>
					</div>
				{/if}
			</Card.Content>
		</Card.Root>

		<!-- Workflows Card (full width) -->
		<Card.Root
			class="cursor-pointer transition-colors hover:bg-accent/50 md:col-span-2"
			onclick={() => goto('/workflows')}
		>
			<Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
				<div class="flex items-center gap-2">
					<GitBranch class="h-5 w-5 text-muted-foreground" />
					<Card.Title class="text-base font-semibold">Workflows</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				{#if loading}
					<Skeleton class="h-4 w-28" />
					<div class="mt-3 flex gap-6">
						<Skeleton class="h-10 w-14" />
						<Skeleton class="h-10 w-14" />
					</div>
				{:else}
					{@const stats = workflowStats()}
					<p class="text-xs text-muted-foreground">
						{stats.total} workflow{stats.total !== 1 ? 's' : ''}
					</p>
					<div class="mt-3 flex gap-6">
						{#if stats.running > 0}
							<div class="text-center">
								<div class="text-2xl font-bold text-green-500">{stats.running}</div>
								<div class="text-xs text-muted-foreground">Running</div>
							</div>
						{/if}
						<div class="text-center">
							<div class="text-2xl font-bold text-zinc-400">
								{stats.total - stats.running}
							</div>
							<div class="text-xs text-muted-foreground">Idle</div>
						</div>
					</div>
					{#if stats.total === 0}
						<p class="text-sm text-muted-foreground">No workflows yet</p>
					{/if}
				{/if}
			</Card.Content>
		</Card.Root>
	</div>
</div>
