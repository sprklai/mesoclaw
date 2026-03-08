<script lang="ts">
	import { onMount } from 'svelte';
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import Calendar from '@lucide/svelte/icons/calendar';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Play from '@lucide/svelte/icons/play';
	import Pause from '@lucide/svelte/icons/pause';
	import History from '@lucide/svelte/icons/history';
	import X from '@lucide/svelte/icons/x';
	import {
		schedulerStore,
		type ScheduledJob,
		type JobExecution
	} from '$lib/stores/scheduler.svelte';

	let showForm = $state(false);
	let showHistory = $state<string | null>(null);
	let historyEntries = $state<JobExecution[]>([]);

	// Form state
	let jobName = $state('');
	let scheduleType = $state<'interval' | 'cron'>('interval');
	let intervalSecs = $state(60);
	let cronExpr = $state('');
	let payloadType = $state<'notify' | 'heartbeat' | 'agent_turn'>('notify');
	let payloadMessage = $state('');
	let payloadPrompt = $state('');
	let sessionTarget = $state<'main' | 'isolated'>('main');
	let deleteAfterRun = $state(false);
	let activeHoursEnabled = $state(false);
	let activeStartHour = $state(9);
	let activeEndHour = $state(17);
	let formError = $state('');

	onMount(() => {
		schedulerStore.load();
	});

	function resetForm() {
		jobName = '';
		scheduleType = 'interval';
		intervalSecs = 60;
		cronExpr = '';
		payloadType = 'notify';
		payloadMessage = '';
		payloadPrompt = '';
		sessionTarget = 'main';
		deleteAfterRun = false;
		activeHoursEnabled = false;
		activeStartHour = 9;
		activeEndHour = 17;
		formError = '';
	}

	async function handleCreate() {
		formError = '';
		if (!jobName.trim()) {
			formError = 'Job name is required';
			return;
		}

		const schedule =
			scheduleType === 'interval'
				? { type: 'interval' as const, secs: intervalSecs }
				: { type: 'cron' as const, expr: cronExpr };

		let payload: ScheduledJob['payload'];
		if (payloadType === 'heartbeat') {
			payload = { type: 'heartbeat' };
		} else if (payloadType === 'agent_turn') {
			if (!payloadPrompt.trim()) {
				formError = 'Prompt is required for agent turn';
				return;
			}
			payload = { type: 'agent_turn', prompt: payloadPrompt };
		} else {
			if (!payloadMessage.trim()) {
				formError = 'Message is required for notify';
				return;
			}
			payload = { type: 'notify', message: payloadMessage };
		}

		try {
			await schedulerStore.createJob({
				name: jobName.trim(),
				schedule,
				payload,
				session_target: sessionTarget,
				delete_after_run: deleteAfterRun,
				active_hours: activeHoursEnabled
					? { start_hour: activeStartHour, end_hour: activeEndHour }
					: null
			});
			resetForm();
			showForm = false;
		} catch (e) {
			formError = e instanceof Error ? e.message : 'Failed to create job';
		}
	}

	async function handleToggle(id: string) {
		await schedulerStore.toggleJob(id);
	}

	async function handleDelete(id: string) {
		await schedulerStore.deleteJob(id);
	}

	async function handleShowHistory(id: string) {
		showHistory = id;
		historyEntries = await schedulerStore.getHistory(id);
	}

	function formatSchedule(job: ScheduledJob): string {
		if (job.schedule.type === 'interval') {
			const secs = job.schedule.secs;
			if (secs >= 3600) return `Every ${Math.round(secs / 3600)}h`;
			if (secs >= 60) return `Every ${Math.round(secs / 60)}m`;
			return `Every ${secs}s`;
		}
		return `Cron: ${job.schedule.expr}`;
	}

	function formatPayload(job: ScheduledJob): string {
		switch (job.payload.type) {
			case 'heartbeat':
				return 'Heartbeat';
			case 'agent_turn':
				return `Agent: ${job.payload.prompt.slice(0, 40)}...`;
			case 'notify':
				return `Notify: ${job.payload.message.slice(0, 40)}`;
			case 'send_via_channel':
				return `Channel: ${job.payload.channel}`;
			default:
				return 'Unknown';
		}
	}

	function formatTime(iso: string | null): string {
		if (!iso) return '—';
		return new Date(iso).toLocaleString();
	}
</script>

<div class="max-w-3xl mx-auto space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-bold">Schedule</h1>
		<div class="flex items-center gap-3">
			{#if schedulerStore.status.running}
				<span class="text-xs text-green-500 font-medium">Running</span>
			{:else}
				<span class="text-xs text-muted-foreground">Stopped</span>
			{/if}
			<Button size="sm" onclick={() => { showForm = !showForm; if (showForm) resetForm(); }}>
				{#if showForm}
					<X class="h-4 w-4 mr-1" /> Cancel
				{:else}
					<Plus class="h-4 w-4 mr-1" /> New Job
				{/if}
			</Button>
		</div>
	</div>

	<!-- Create Job Form -->
	{#if showForm}
		<Card.Root>
			<Card.Header>
				<Card.Title>Create Scheduled Job</Card.Title>
			</Card.Header>
			<Card.Content class="space-y-4">
				{#if formError}
					<p class="text-sm text-red-500">{formError}</p>
				{/if}

				<div class="space-y-2">
					<Label for="job-name">Name</Label>
					<Input id="job-name" bind:value={jobName} placeholder="e.g. daily-health-check" />
				</div>

				<div class="grid grid-cols-2 gap-4">
					<div class="space-y-2">
						<Label>Schedule Type</Label>
						<select
							bind:value={scheduleType}
							class="w-full rounded-md border bg-background text-foreground px-3 py-2 text-sm"
						>
							<option value="interval">Interval</option>
							<option value="cron">Cron</option>
						</select>
					</div>

					{#if scheduleType === 'interval'}
						<div class="space-y-2">
							<Label for="interval-secs">Interval (seconds)</Label>
							<Input
								id="interval-secs"
								type="number"
								min="1"
								bind:value={intervalSecs}
							/>
						</div>
					{:else}
						<div class="space-y-2">
							<Label for="cron-expr">Cron Expression</Label>
							<Input
								id="cron-expr"
								bind:value={cronExpr}
								placeholder="0 */5 * * * *"
							/>
						</div>
					{/if}
				</div>

				<div class="space-y-2">
					<Label>Payload</Label>
					<select
						bind:value={payloadType}
						class="w-full rounded-md border bg-background text-foreground px-3 py-2 text-sm"
					>
						<option value="notify">Notify</option>
						<option value="heartbeat">Heartbeat</option>
						<option value="agent_turn">Agent Turn</option>
					</select>
				</div>

				{#if payloadType === 'notify'}
					<div class="space-y-2">
						<Label for="payload-message">Message</Label>
						<Input
							id="payload-message"
							bind:value={payloadMessage}
							placeholder="Notification message"
						/>
					</div>
				{:else if payloadType === 'agent_turn'}
					<div class="space-y-2">
						<Label for="payload-prompt">Prompt</Label>
						<Input
							id="payload-prompt"
							bind:value={payloadPrompt}
							placeholder="Agent prompt to execute"
						/>
					</div>
				{/if}

				<div class="grid grid-cols-2 gap-4">
					<div class="space-y-2">
						<Label>Session</Label>
						<select
							bind:value={sessionTarget}
							class="w-full rounded-md border bg-background text-foreground px-3 py-2 text-sm"
						>
							<option value="main">Main</option>
							<option value="isolated">Isolated</option>
						</select>
					</div>

					<div class="flex items-center gap-2 pt-6">
						<input type="checkbox" id="one-shot" bind:checked={deleteAfterRun} />
						<Label for="one-shot">One-shot (delete after run)</Label>
					</div>
				</div>

				<div class="space-y-2">
					<div class="flex items-center gap-2">
						<input
							type="checkbox"
							id="active-hours"
							bind:checked={activeHoursEnabled}
						/>
						<Label for="active-hours">Restrict to active hours</Label>
					</div>
					{#if activeHoursEnabled}
						<div class="grid grid-cols-2 gap-4">
							<div class="space-y-1">
								<Label for="start-hour">Start hour</Label>
								<Input
									id="start-hour"
									type="number"
									min="0"
									max="23"
									bind:value={activeStartHour}
								/>
							</div>
							<div class="space-y-1">
								<Label for="end-hour">End hour</Label>
								<Input
									id="end-hour"
									type="number"
									min="0"
									max="23"
									bind:value={activeEndHour}
								/>
							</div>
						</div>
					{/if}
				</div>

				<Button onclick={handleCreate} class="w-full">Create Job</Button>
			</Card.Content>
		</Card.Root>
	{/if}

	<!-- Job List -->
	{#if schedulerStore.loading}
		<p class="text-sm text-muted-foreground">Loading...</p>
	{:else if schedulerStore.jobs.length === 0 && !showForm}
		<Card.Root>
			<Card.Content class="flex flex-col items-center justify-center py-12 text-center">
				<Calendar class="h-12 w-12 text-muted-foreground mb-4" />
				<h2 class="text-lg font-medium">No scheduled jobs</h2>
				<p class="text-muted-foreground mt-1">
					Create a job to automate tasks on a schedule.
				</p>
			</Card.Content>
		</Card.Root>
	{:else}
		<div class="space-y-3">
			{#each schedulerStore.jobs as job (job.id)}
				<Card.Root>
					<Card.Content class="py-4">
						<div class="flex items-center justify-between">
							<div class="space-y-1">
								<div class="flex items-center gap-2">
									<span class="font-medium">{job.name}</span>
									{#if !job.enabled}
										<span
											class="text-xs px-1.5 py-0.5 rounded bg-muted text-muted-foreground"
											>Disabled</span
										>
									{/if}
									{#if job.delete_after_run}
										<span
											class="text-xs px-1.5 py-0.5 rounded bg-yellow-500/10 text-yellow-500"
											>One-shot</span
										>
									{/if}
									{#if job.error_count > 0}
										<span
											class="text-xs px-1.5 py-0.5 rounded bg-red-500/10 text-red-500"
											>{job.error_count} errors</span
										>
									{/if}
								</div>
								<div class="flex items-center gap-3 text-xs text-muted-foreground">
									<span>{formatSchedule(job)}</span>
									<span>{formatPayload(job)}</span>
									{#if job.next_run}
										<span>Next: {formatTime(job.next_run)}</span>
									{/if}
									{#if job.active_hours}
										<span
											>{job.active_hours.start_hour}:00–{job.active_hours
												.end_hour}:00</span
										>
									{/if}
								</div>
							</div>
							<div class="flex items-center gap-1">
								<Button
									variant="ghost"
									size="icon"
									onclick={() => handleToggle(job.id)}
									title={job.enabled ? 'Disable' : 'Enable'}
								>
									{#if job.enabled}
										<Pause class="h-4 w-4" />
									{:else}
										<Play class="h-4 w-4" />
									{/if}
								</Button>
								<Button
									variant="ghost"
									size="icon"
									onclick={() => handleShowHistory(job.id)}
									title="History"
								>
									<History class="h-4 w-4" />
								</Button>
								<Button
									variant="ghost"
									size="icon"
									onclick={() => handleDelete(job.id)}
									title="Delete"
								>
									<Trash2 class="h-4 w-4 text-red-500" />
								</Button>
							</div>
						</div>
					</Card.Content>
				</Card.Root>
			{/each}
		</div>
	{/if}

	<!-- History Modal -->
	{#if showHistory}
		<Card.Root>
			<Card.Header>
				<div class="flex items-center justify-between">
					<Card.Title>Execution History</Card.Title>
					<Button variant="ghost" size="icon" onclick={() => (showHistory = null)}>
						<X class="h-4 w-4" />
					</Button>
				</div>
			</Card.Header>
			<Card.Content>
				{#if historyEntries.length === 0}
					<p class="text-sm text-muted-foreground">No executions yet.</p>
				{:else}
					<div class="space-y-2 max-h-64 overflow-y-auto">
						{#each historyEntries as entry (entry.id)}
							<div
								class="flex items-center justify-between text-sm border-b pb-2 last:border-0"
							>
								<div class="flex items-center gap-2">
									<span
										class="px-1.5 py-0.5 rounded text-xs {entry.status === 'success' ? 'bg-green-500/10 text-green-500' : ''} {entry.status === 'failed' ? 'bg-red-500/10 text-red-500' : ''} {entry.status === 'stuck' ? 'bg-yellow-500/10 text-yellow-500' : ''} {entry.status === 'skipped' ? 'bg-muted' : ''}"
									>
										{entry.status}
									</span>
									{#if entry.error}
										<span class="text-red-400 text-xs">{entry.error}</span>
									{/if}
								</div>
								<span class="text-xs text-muted-foreground">
									{formatTime(entry.started_at)}
								</span>
							</div>
						{/each}
					</div>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
</div>
