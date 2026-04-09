<script lang="ts">
	import * as m from '$lib/paraglide/messages';
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
	import { wikiStore } from '$lib/stores/wiki.svelte';
	import { toast } from 'svelte-sonner';
	import { goto } from '$app/navigation';
	import { notificationStore } from '$lib/stores/notifications.svelte';
	import MessageSquarePlus from '@lucide/svelte/icons/message-square-plus';
	import Radio from '@lucide/svelte/icons/radio';
	import Brain from '@lucide/svelte/icons/brain';
	import Clock from '@lucide/svelte/icons/clock';
	import GitBranch from '@lucide/svelte/icons/git-branch';
	import BookOpen from '@lucide/svelte/icons/book-open';

	let loading = $state(true);
	let creating = $state(false);
	let refreshTimer: ReturnType<typeof setTimeout> | undefined;
	let lastRefreshedActivity = 0;

	$effect(() => {
		loadDashboardData();
	});

	// Debounce-refresh dashboard when push events arrive (session/message/channel activity)
	$effect(() => {
		const ts = notificationStore.lastActivityAt;
		if (ts > lastRefreshedActivity) {
			clearTimeout(refreshTimer);
			refreshTimer = setTimeout(() => {
				lastRefreshedActivity = ts;
				loadDashboardData();
			}, 2000);
		}
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
				wikiStore.load(),
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
			const session = await sessionsStore.create(m.chat_new());
			goto(`/chat/${session.id}`);
		} catch {
			toast.error(m.chat_session_create_error());
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
		<h1 class="text-3xl font-bold">{m.dashboard_title()}</h1>
		<p class="text-muted-foreground">{m.app_tagline()}</p>
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
					<Card.Title class="text-base font-semibold">{m.dashboard_card_chat_title()}</Card.Title>
				</div>
				<Button
					size="sm"
					onclick={handleNewChat}
					disabled={creating}
					class="h-7 gap-1 px-2 text-xs"
				>
					<MessageSquarePlus class="h-3.5 w-3.5" />
					{m.dashboard_new_chat_button()}
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
						{m.dashboard_sessions_count({ count: sessionsStore.sessions.length, suffix: sessionsStore.sessions.length !== 1 ? 's' : '' })}
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
						<p class="text-sm text-muted-foreground">{m.dashboard_no_chats()}</p>
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
					<Card.Title class="text-base font-semibold">{m.dashboard_card_channels_title()}</Card.Title>
				</div>
				{#if !loading && channelStats().active > 0}
					<span class="flex items-center gap-1.5 text-xs text-muted-foreground">
						<span class="h-2 w-2 rounded-full bg-green-500"></span>
						{m.dashboard_channels_live()}
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
						{m.dashboard_channels_active_total({ active: stats.active, total: stats.total })}
					</p>
					<div class="mb-2 flex flex-wrap gap-1.5">
						{#each channelsStore.channels as channel (channel.id)}
							<Badge variant={channel.connected ? 'default' : 'secondary'} class="text-xs">
								{channel.name}
							</Badge>
						{/each}
						{#if channelsStore.channels.length === 0}
							<span class="text-sm text-muted-foreground">{m.dashboard_channels_none()}</span>
						{/if}
					</div>
					{#if inboxStore.totalUnread > 0}
						<p class="text-xs text-muted-foreground">
							{m.dashboard_channels_unread({ count: inboxStore.totalUnread, suffix: inboxStore.totalUnread !== 1 ? 's' : '' })}
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
					<Card.Title class="text-base font-semibold">{m.dashboard_card_memory_title()}</Card.Title>
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
						{m.dashboard_memory_total({ count: memoryStore.entries.length + memoryStore.observations.length })}
					</p>
					<div class="mt-3 flex gap-6">
						<div class="text-center">
							<div class="text-2xl font-bold text-blue-500">
								{memoryStore.observations.length}
							</div>
							<div class="text-xs text-muted-foreground">{m.dashboard_memory_learned()}</div>
						</div>
						<div class="text-center">
							<div class="text-2xl font-bold text-orange-500">
								{memoryStore.entries.length}
							</div>
							<div class="text-xs text-muted-foreground">{m.dashboard_memory_saved()}</div>
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
					<Card.Title class="text-base font-semibold">{m.dashboard_card_schedule_title()}</Card.Title>
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
						{m.dashboard_schedule_enabled({ total: stats.total, suffix: stats.total !== 1 ? 's' : '', enabled: stats.enabled })}
					</p>
					<div class="mt-3 flex gap-6">
						<div class="text-center">
							<div class="text-2xl font-bold text-green-500">{stats.cron}</div>
							<div class="text-xs text-muted-foreground">{m.dashboard_schedule_cron()}</div>
						</div>
						<div class="text-center">
							<div class="text-2xl font-bold text-blue-500">{stats.interval}</div>
							<div class="text-xs text-muted-foreground">{m.dashboard_schedule_interval()}</div>
						</div>
						<div class="text-center">
							<div class="text-2xl font-bold text-zinc-400">{stats.oneTime}</div>
							<div class="text-xs text-muted-foreground">{m.dashboard_schedule_one_time()}</div>
						</div>
					</div>
				{/if}
			</Card.Content>
		</Card.Root>

		<!-- Wiki Card -->
		<Card.Root
			class="cursor-pointer transition-colors hover:bg-accent/50"
			onclick={() => goto('/wiki')}
		>
			<Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
				<div class="flex items-center gap-2">
					<BookOpen class="h-5 w-5 text-muted-foreground" />
					<Card.Title class="text-base font-semibold">{m.dashboard_card_wiki_title()}</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				{#if loading}
					<Skeleton class="h-4 w-24" />
					<div class="mt-3 flex gap-6">
						<Skeleton class="h-10 w-16" />
					</div>
				{:else}
					<p class="text-xs text-muted-foreground">
						{m.dashboard_wiki_pages_count({ count: wikiStore.pages.length, suffix: wikiStore.pages.length !== 1 ? 's' : '' })}
					</p>
					{#if wikiStore.pages.length > 0}
						<div class="mt-2 flex flex-wrap gap-1">
							{#each [...new Set(wikiStore.pages.map(p => p.page_type))].slice(0, 5) as type}
								<span class="rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">{type}</span>
							{/each}
						</div>
					{:else}
						<p class="mt-1 text-sm text-muted-foreground">{m.dashboard_wiki_no_pages()}</p>
					{/if}
				{/if}
			</Card.Content>
		</Card.Root>

		<!-- Workflows Card -->
		<Card.Root
			class="cursor-pointer transition-colors hover:bg-accent/50"
			onclick={() => goto('/workflows')}
		>
			<Card.Header class="flex flex-row items-center justify-between space-y-0 pb-2">
				<div class="flex items-center gap-2">
					<GitBranch class="h-5 w-5 text-muted-foreground" />
					<Card.Title class="text-base font-semibold">{m.dashboard_card_workflows_title()}</Card.Title>
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
						{m.dashboard_workflows_count({ count: stats.total, suffix: stats.total !== 1 ? 's' : '' })}
					</p>
					<div class="mt-3 flex gap-6">
						{#if stats.running > 0}
							<div class="text-center">
								<div class="text-2xl font-bold text-green-500">{stats.running}</div>
								<div class="text-xs text-muted-foreground">{m.dashboard_workflows_running()}</div>
							</div>
						{/if}
						<div class="text-center">
							<div class="text-2xl font-bold text-zinc-400">
								{stats.total - stats.running}
							</div>
							<div class="text-xs text-muted-foreground">{m.dashboard_workflows_idle()}</div>
						</div>
					</div>
					{#if stats.total === 0}
						<p class="text-sm text-muted-foreground">{m.dashboard_no_workflows()}</p>
					{/if}
				{/if}
			</Card.Content>
		</Card.Root>
	</div>
</div>
