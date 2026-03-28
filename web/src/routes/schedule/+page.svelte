<script lang="ts">
	import { onMount } from 'svelte';
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import Pencil from '@lucide/svelte/icons/pencil';
	import Calendar from '@lucide/svelte/icons/calendar';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Play from '@lucide/svelte/icons/play';
	import Pause from '@lucide/svelte/icons/pause';
	import History from '@lucide/svelte/icons/history';
	import X from '@lucide/svelte/icons/x';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import {
		schedulerStore,
		type ScheduledJob,
		type JobExecution
	} from '$lib/stores/scheduler.svelte';
	import { channelsStore } from '$lib/stores/channels.svelte';
	import * as m from '$lib/paraglide/messages';

	let showForm = $state(false);
	let showHistory = $state<string | null>(null);
	let historyEntries = $state<JobExecution[]>([]);
	let confirmOpen = $state(false);
	let deleteTarget = $state<string | null>(null);
	let editTarget = $state<string | null>(null);

	// Form state
	let jobName = $state('');
	let scheduleType = $state<'interval' | 'cron' | 'human'>('interval');
	let intervalSecs = $state(60);
	let cronExpr = $state('');
	let humanDate = $state('');
	let humanTime = $state('');
	let payloadType = $state<'notify' | 'heartbeat' | 'agent_turn' | 'send_via_channel'>('notify');
	let payloadMessage = $state('');
	let payloadPrompt = $state('');
	let payloadChannel = $state('');
	let sessionTarget = $state<'main' | 'isolated'>('main');
	let deleteAfterRun = $state(false);
	let activeHoursEnabled = $state(false);
	let activeStartHour = $state(9);
	let activeEndHour = $state(17);
	let formError = $state('');

	onMount(() => {
		schedulerStore.load();
		channelsStore.load();
	});

	function resetForm() {
		jobName = '';
		scheduleType = 'interval';
		intervalSecs = 60;
		cronExpr = '';
		humanDate = '';
		humanTime = '';
		payloadType = 'notify';
		payloadMessage = '';
		payloadPrompt = '';
		payloadChannel = '';
		sessionTarget = 'main';
		deleteAfterRun = false;
		activeHoursEnabled = false;
		activeStartHour = 9;
		activeEndHour = 17;
		formError = '';
		editTarget = null;
	}

	async function handleCreate() {
		formError = '';
		if (!jobName.trim()) {
			formError = m.schedule_validation_name_required();
			return;
		}

		if (scheduleType === 'cron') {
			const trimmed = cronExpr.trim();
			if (!trimmed) {
				formError = m.schedule_validation_cron_required();
				return;
			}
			const fields = trimmed.split(/\s+/);
			if (fields.length < 5 || fields.length > 6) {
				formError = m.schedule_validation_cron_fields();
				return;
			}
		}

		if (scheduleType === 'human') {
			if (!humanDate || !humanTime) {
				formError = m.schedule_validation_date_time_required();
				return;
			}
			const combined = `${humanDate}T${humanTime}`;
			if (new Date(combined) <= new Date()) {
				formError = m.schedule_validation_date_time_future();
				return;
			}
		}

		const schedule =
			scheduleType === 'interval'
				? { type: 'interval' as const, secs: intervalSecs }
				: scheduleType === 'cron'
					? { type: 'cron' as const, expr: cronExpr }
					: { type: 'human' as const, datetime: `${humanDate}T${humanTime}` };

		let payload: ScheduledJob['payload'];
		if (payloadType === 'heartbeat') {
			payload = { type: 'heartbeat' };
		} else if (payloadType === 'agent_turn') {
			if (!payloadPrompt.trim()) {
				formError = m.schedule_validation_prompt_required();
				return;
			}
			payload = { type: 'agent_turn', prompt: payloadPrompt };
		} else if (payloadType === 'send_via_channel') {
			if (!payloadChannel) {
				formError = m.schedule_validation_channel_required();
				return;
			}
			if (!payloadMessage.trim()) {
				formError = m.schedule_validation_message_channel_required();
				return;
			}
			payload = { type: 'send_via_channel', channel: payloadChannel, message: payloadMessage };
		} else {
			if (!payloadMessage.trim()) {
				formError = m.schedule_validation_message_notify_required();
				return;
			}
			payload = { type: 'notify', message: payloadMessage };
		}

		try {
			const jobData = {
				name: jobName.trim(),
				schedule,
				payload,
				session_target: sessionTarget,
				delete_after_run: scheduleType === 'human' ? true : deleteAfterRun,
				active_hours: activeHoursEnabled
					? { start_hour: activeStartHour, end_hour: activeEndHour }
					: null
			};
			if (editTarget) {
				await schedulerStore.updateJob(editTarget, jobData);
			} else {
				await schedulerStore.createJob(jobData);
			}
			resetForm();
			showForm = false;
		} catch (e) {
			formError = e instanceof Error ? e.message : editTarget ? m.schedule_update_error() : m.schedule_create_error();
		}
	}

	function handleStartEdit(job: ScheduledJob) {
		editTarget = job.id;
		jobName = job.name;
		if (job.schedule.type === 'interval') {
			scheduleType = 'interval';
			intervalSecs = job.schedule.secs;
		} else if (job.schedule.type === 'cron') {
			scheduleType = 'cron';
			cronExpr = job.schedule.expr;
		} else if (job.schedule.type === 'human') {
			scheduleType = 'human';
			const dt = job.schedule.datetime;
			humanDate = dt.split('T')[0] ?? '';
			humanTime = dt.split('T')[1]?.slice(0, 5) ?? '';
		}
		if (job.payload.type === 'heartbeat') {
			payloadType = 'heartbeat';
		} else if (job.payload.type === 'agent_turn') {
			payloadType = 'agent_turn';
			payloadPrompt = job.payload.prompt;
		} else if (job.payload.type === 'notify') {
			payloadType = 'notify';
			payloadMessage = job.payload.message;
		} else if (job.payload.type === 'send_via_channel') {
			payloadType = 'send_via_channel';
			payloadChannel = job.payload.channel;
			payloadMessage = job.payload.message;
		}
		sessionTarget = job.session_target;
		deleteAfterRun = job.delete_after_run;
		if (job.active_hours) {
			activeHoursEnabled = true;
			activeStartHour = job.active_hours.start_hour;
			activeEndHour = job.active_hours.end_hour;
		} else {
			activeHoursEnabled = false;
		}
		formError = '';
		showForm = true;
	}

	async function handleToggle(id: string) {
		await schedulerStore.toggleJob(id);
	}

	function handleDelete(id: string) {
		deleteTarget = id;
		confirmOpen = true;
	}

	async function confirmDelete() {
		if (!deleteTarget) return;
		await schedulerStore.deleteJob(deleteTarget);
	}

	async function handleShowHistory(id: string) {
		showHistory = id;
		historyEntries = await schedulerStore.getHistory(id);
	}

	function formatSchedule(job: ScheduledJob): string {
		if (job.schedule.type === 'interval') {
			const secs = job.schedule.secs;
			if (secs >= 3600) return m.schedule_format_every_hours({ value: Math.round(secs / 3600).toString() });
			if (secs >= 60) return m.schedule_format_every_minutes({ value: Math.round(secs / 60).toString() });
			return m.schedule_format_every_seconds({ value: secs.toString() });
		}
		if (job.schedule.type === 'human') {
			return m.schedule_format_one_time({ datetime: new Date(job.schedule.datetime).toLocaleString() });
		}
		return m.schedule_format_cron({ expr: job.schedule.expr });
	}

	function formatPayload(job: ScheduledJob): string {
		switch (job.payload.type) {
			case 'heartbeat':
				return m.schedule_format_heartbeat();
			case 'agent_turn':
				return m.schedule_format_agent({ prompt: job.payload.prompt.slice(0, 40) + '...' });
			case 'notify':
				return m.schedule_format_notify({ message: job.payload.message.slice(0, 40) });
			case 'send_via_channel':
				return m.schedule_format_channel({ channel: job.payload.channel });
			default:
				return m.schedule_format_unknown();
		}
	}

	function formatTime(iso: string | null): string {
		if (!iso) return '—';
		return new Date(iso).toLocaleString();
	}
</script>

<div class="max-w-3xl mx-auto space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-bold">{m.schedule_page_title()}</h1>
		<div class="flex items-center gap-3">
			{#if schedulerStore.status.running}
				<span class="text-xs text-green-500 font-medium">{m.schedule_status_running()}</span>
			{:else}
				<span class="text-xs text-muted-foreground">{m.schedule_status_stopped()}</span>
			{/if}
			<Button size="sm" onclick={() => { showForm = !showForm; if (showForm) resetForm(); }}>
				{#if showForm}
					<X class="h-4 w-4 mr-1" /> {m.schedule_cancel_button()}
				{:else}
					<Plus class="h-4 w-4 mr-1" /> {m.schedule_new_job_button()}
				{/if}
			</Button>
		</div>
	</div>

	<!-- Create Job Form -->
	{#if showForm}
		<Card.Root>
			<Card.Header>
				<Card.Title>{editTarget ? m.schedule_edit_title() : m.schedule_create_title()}</Card.Title>
			</Card.Header>
			<Card.Content class="space-y-4">
				{#if formError}
					<p class="text-sm text-red-500">{formError}</p>
				{/if}

				<div class="space-y-2">
					<Label for="job-name">{m.schedule_name_label()}</Label>
					<Input id="job-name" bind:value={jobName} placeholder={m.schedule_name_placeholder()} />
				</div>

				<div class="grid grid-cols-2 gap-4">
					<div class="space-y-2">
						<Label>{m.schedule_schedule_type_label()}</Label>
						<select
							bind:value={scheduleType}
							class="w-full rounded-md border bg-background text-foreground px-3 py-2 text-sm"
						>
							<option value="interval">{m.schedule_schedule_type_interval()}</option>
							<option value="cron">{m.schedule_schedule_type_cron()}</option>
							<option value="human">{m.schedule_schedule_type_one_time()}</option>
						</select>
					</div>

					{#if scheduleType === 'interval'}
						<div class="space-y-2">
							<Label for="interval-secs">{m.schedule_interval_label()}</Label>
							<Input
								id="interval-secs"
								type="number"
								min="1"
								bind:value={intervalSecs}
							/>
						</div>
					{:else if scheduleType === 'cron'}
						<div class="space-y-2">
							<Label for="cron-expr">{m.schedule_cron_label()}</Label>
							<Input
								id="cron-expr"
								bind:value={cronExpr}
								placeholder={m.schedule_cron_placeholder()}
							/>
						</div>
					{:else}
						<div class="space-y-2">
							<Label for="human-date">{m.schedule_date_label()}</Label>
							<input
								id="human-date"
								type="date"
								bind:value={humanDate}
								class="w-full rounded-md border bg-background text-foreground px-3 py-2 text-sm"
							/>
						</div>
					{/if}
				</div>

				{#if scheduleType === 'human'}
				<div class="space-y-2">
					<Label for="human-time">{m.schedule_time_label()}</Label>
					<input
						id="human-time"
						type="time"
						bind:value={humanTime}
						class="w-full rounded-md border bg-background text-foreground px-3 py-2 text-sm"
					/>
				</div>
				{/if}

				<div class="space-y-2">
					<Label>{m.schedule_payload_label()}</Label>
					<select
						bind:value={payloadType}
						class="w-full rounded-md border bg-background text-foreground px-3 py-2 text-sm"
					>
						<option value="notify">{m.schedule_payload_option_notify()}</option>
						<option value="heartbeat">{m.schedule_payload_option_heartbeat()}</option>
						<option value="agent_turn">{m.schedule_payload_option_agent_turn()}</option>
						<option value="send_via_channel">{m.schedule_payload_option_send_via_channel()}</option>
					</select>
				</div>

				{#if payloadType === 'notify'}
					<div class="space-y-2">
						<Label for="payload-message">{m.schedule_message_label()}</Label>
						<Input
							id="payload-message"
							bind:value={payloadMessage}
							placeholder={m.schedule_message_placeholder()}
						/>
					</div>
				{:else if payloadType === 'agent_turn'}
					<div class="space-y-2">
						<Label for="payload-prompt">{m.schedule_prompt_label()}</Label>
						<Input
							id="payload-prompt"
							bind:value={payloadPrompt}
							placeholder={m.schedule_prompt_placeholder()}
						/>
					</div>
				{:else if payloadType === 'send_via_channel'}
					<div class="grid grid-cols-2 gap-4">
						<div class="space-y-2">
							<Label for="payload-channel">{m.schedule_channel_label()}</Label>
							<select
								id="payload-channel"
								bind:value={payloadChannel}
								class="w-full rounded-md border bg-background text-foreground px-3 py-2 text-sm"
							>
								<option value="">{m.schedule_channel_placeholder()}</option>
								{#each channelsStore.channels.filter((c) => c.connected) as ch (ch.id)}
									<option value={ch.id}>{ch.name}</option>
								{/each}
							</select>
						</div>
						<div class="space-y-2">
							<Label for="channel-message">{m.schedule_channel_message_label()}</Label>
							<Input
								id="channel-message"
								bind:value={payloadMessage}
								placeholder={m.schedule_channel_message_placeholder()}
							/>
						</div>
					</div>
				{/if}

				<div class="grid grid-cols-2 gap-4">
					<div class="space-y-2">
						<Label>{m.schedule_session_label()}</Label>
						<select
							bind:value={sessionTarget}
							class="w-full rounded-md border bg-background text-foreground px-3 py-2 text-sm"
						>
							<option value="main">{m.schedule_session_option_main()}</option>
							<option value="isolated">{m.schedule_session_option_isolated()}</option>
						</select>
					</div>

					<div class="flex items-center gap-2 pt-6">
						{#if scheduleType === 'human'}
							<span class="text-xs text-muted-foreground">{m.schedule_auto_delete_note()}</span>
						{:else}
							<input type="checkbox" id="one-shot" bind:checked={deleteAfterRun} />
							<Label for="one-shot">{m.schedule_one_shot_label()}</Label>
						{/if}
					</div>
				</div>

				<div class="space-y-2">
					<div class="flex items-center gap-2">
						<input
							type="checkbox"
							id="active-hours"
							bind:checked={activeHoursEnabled}
						/>
						<Label for="active-hours">{m.schedule_active_hours_label()}</Label>
					</div>
					{#if activeHoursEnabled}
						<div class="grid grid-cols-2 gap-4">
							<div class="space-y-1">
								<Label for="start-hour">{m.schedule_active_hours_start_label()}</Label>
								<Input
									id="start-hour"
									type="number"
									min="0"
									max="23"
									bind:value={activeStartHour}
								/>
							</div>
							<div class="space-y-1">
								<Label for="end-hour">{m.schedule_active_hours_end_label()}</Label>
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

				<Button onclick={handleCreate} class="w-full">{editTarget ? m.schedule_update_button() : m.schedule_create_button()}</Button>
			</Card.Content>
		</Card.Root>
	{/if}

	<!-- Job List -->
	{#if schedulerStore.loading}
		<p class="text-sm text-muted-foreground">{m.schedule_loading()}</p>
	{:else if schedulerStore.jobs.length === 0 && !showForm}
		<Card.Root>
			<Card.Content class="flex flex-col items-center justify-center py-8 text-center">
				<Calendar class="h-12 w-12 text-muted-foreground mb-4" />
				<h2 class="text-lg font-medium">{m.schedule_empty_title()}</h2>
				<p class="text-muted-foreground mt-1">
					{m.schedule_empty_description()}
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
											>{m.schedule_disabled_badge()}</span
										>
									{/if}
									{#if job.delete_after_run}
										<span
											class="text-xs px-1.5 py-0.5 rounded bg-yellow-500/10 text-yellow-500"
											>{m.schedule_one_shot_badge()}</span
										>
									{/if}
									{#if job.error_count > 0}
										<span
											class="text-xs px-1.5 py-0.5 rounded bg-red-500/10 text-red-500"
											>{m.schedule_error_count({ count: job.error_count.toString() })}</span
										>
									{/if}
								</div>
								<div class="flex items-center gap-3 text-xs text-muted-foreground">
									<span>{formatSchedule(job)}</span>
									<span>{formatPayload(job)}</span>
									{#if job.next_run}
										<span>{m.schedule_next_run_label({ time: formatTime(job.next_run) })}</span>
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
									onclick={() => handleStartEdit(job)}
									title={m.schedule_edit_button_title()}
								>
									<Pencil class="h-4 w-4" />
								</Button>
								<Button
									variant="ghost"
									size="icon"
									onclick={() => handleToggle(job.id)}
									title={job.enabled ? m.schedule_disable_button_title() : m.schedule_enable_button_title()}
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
									title={m.schedule_history_button_title()}
								>
									<History class="h-4 w-4" />
								</Button>
								<Button
									variant="ghost"
									size="icon"
									onclick={() => handleDelete(job.id)}
									title={m.schedule_delete_button_title()}
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
					<Card.Title>{m.schedule_history_title()}</Card.Title>
					<Button variant="ghost" size="icon" onclick={() => (showHistory = null)}>
						<X class="h-4 w-4" />
					</Button>
				</div>
			</Card.Header>
			<Card.Content>
				{#if historyEntries.length === 0}
					<p class="text-sm text-muted-foreground">{m.schedule_history_empty()}</p>
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

<ConfirmDialog
	bind:open={confirmOpen}
	title={m.schedule_delete_confirm_title()}
	description={m.schedule_delete_confirm_description()}
	onConfirm={confirmDelete}
/>
