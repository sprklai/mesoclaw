<script lang="ts">
	import { onMount } from 'svelte';
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import WorkflowIcon from '@lucide/svelte/icons/workflow';
	import Plus from '@lucide/svelte/icons/plus';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Play from '@lucide/svelte/icons/play';
	import Square from '@lucide/svelte/icons/square';
	import Loader2 from '@lucide/svelte/icons/loader-2';
	import History from '@lucide/svelte/icons/history';
	import Pencil from '@lucide/svelte/icons/pencil';
	import X from '@lucide/svelte/icons/x';
	import Clock from '@lucide/svelte/icons/clock';
	import CheckCircle2 from '@lucide/svelte/icons/check-circle-2';
	import XCircle from '@lucide/svelte/icons/x-circle';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import {
		workflowsStore,
		type Workflow,
		type WorkflowRun,
		type StepOutput
	} from '$lib/stores/workflows.svelte';
	import * as m from '$lib/paraglide/messages';

	let showForm = $state(false);
	let showHistory = $state<string | null>(null);
	let historyEntries = $state<WorkflowRun[]>([]);
	let expandedRun = $state<string | null>(null);
	let confirmOpen = $state(false);
	let deleteTarget = $state<string | null>(null);
	let editTarget = $state<string | null>(null);

	// Form state
	let tomlContent = $state('');
	let formError = $state('');

	onMount(() => {
		workflowsStore.load();
	});

	function resetForm() {
		tomlContent = '';
		formError = '';
		editTarget = null;
	}

	async function handleCreate() {
		formError = '';
		if (!tomlContent.trim()) {
			formError = m.workflows_validation_toml_required();
			return;
		}

		try {
			if (editTarget) {
				await workflowsStore.update(editTarget, tomlContent.trim());
			} else {
				await workflowsStore.create(tomlContent.trim());
			}
			resetForm();
			showForm = false;
		} catch (e) {
			formError = e instanceof Error ? e.message : editTarget ? m.workflows_update_error() : m.workflows_create_error();
		}
	}

	async function handleStartEdit(wf: Workflow) {
		try {
			const raw = await workflowsStore.getRawToml(wf.id);
			editTarget = wf.id;
			tomlContent = raw;
			formError = '';
			showForm = true;
		} catch (e) {
			// Fallback: could show error
		}
	}

	async function handleRun(id: string) {
		try {
			await workflowsStore.run(id);
		} catch {
			// Could show a toast here
		}
	}

	async function handleCancel(id: string) {
		await workflowsStore.cancel(id);
	}

	function handleDelete(id: string) {
		deleteTarget = id;
		confirmOpen = true;
	}

	async function confirmDelete() {
		if (!deleteTarget) return;
		await workflowsStore.remove(deleteTarget);
	}

	async function handleShowHistory(id: string) {
		if (showHistory === id) {
			showHistory = null;
			return;
		}
		showHistory = id;
		historyEntries = await workflowsStore.history(id);
	}

	function toggleRunExpand(runId: string) {
		expandedRun = expandedRun === runId ? null : runId;
	}

	function statusBadgeClass(status: string): string {
		switch (status.toLowerCase()) {
			case 'running':
				return 'bg-yellow-500/10 text-yellow-500';
			case 'completed':
				return 'bg-green-500/10 text-green-500';
			case 'failed':
				return 'bg-red-500/10 text-red-500';
			case 'cancelled':
				return 'bg-muted text-muted-foreground';
			default:
				return 'bg-muted text-muted-foreground';
		}
	}

	function formatTime(iso: string | null): string {
		if (!iso) return '—';
		return new Date(iso).toLocaleString();
	}
</script>

<div class="max-w-3xl mx-auto space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-bold">{m.workflows_page_title()}</h1>
		<Button size="sm" onclick={() => { showForm = !showForm; if (showForm) resetForm(); }}>
			{#if showForm}
				<X class="h-4 w-4 mr-1" /> {m.workflows_cancel_button()}
			{:else}
				<Plus class="h-4 w-4 mr-1" /> {m.workflows_new_button()}
			{/if}
		</Button>
	</div>

	<!-- Create Workflow Form -->
	{#if showForm}
		<Card.Root>
			<Card.Header>
				<Card.Title>{editTarget ? m.workflows_edit_title() : m.workflows_create_title()}</Card.Title>
			</Card.Header>
			<Card.Content class="space-y-4">
				{#if formError}
					<p class="text-sm text-red-500">{formError}</p>
				{/if}

				<div class="space-y-2">
					<label for="toml-content" class="text-sm font-medium">{m.workflows_toml_label()}</label>
					<textarea
						id="toml-content"
						bind:value={tomlContent}
						placeholder={m.workflows_toml_placeholder()}
						rows="12"
						class="w-full rounded-md border bg-background text-foreground px-3 py-2 text-sm font-mono resize-y"
					></textarea>
				</div>

				<Button onclick={handleCreate} class="w-full">{editTarget ? m.workflows_update_button() : m.workflows_create_button()}</Button>
			</Card.Content>
		</Card.Root>
	{/if}

	<!-- Workflow List -->
	{#if workflowsStore.loading}
		<p class="text-sm text-muted-foreground">{m.workflows_loading()}</p>
	{:else if workflowsStore.workflows.length === 0 && !showForm}
		<Card.Root>
			<Card.Content class="flex flex-col items-center justify-center py-8 text-center">
				<WorkflowIcon class="h-12 w-12 text-muted-foreground mb-4" />
				<h2 class="text-lg font-medium">{m.workflows_empty_title()}</h2>
				<p class="text-muted-foreground mt-1">
					{m.workflows_empty_description()}
				</p>
			</Card.Content>
		</Card.Root>
	{:else}
		<div class="space-y-3">
			{#each workflowsStore.workflows as workflow (workflow.id)}
				<Card.Root>
					<Card.Content class="py-4">
						<div class="flex items-center justify-between">
							<div class="space-y-1">
								<div class="flex items-center gap-2">
									<span class="font-medium">{workflow.name}</span>
									{#if workflow.schedule}
										<span class="text-xs px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-500">
											<Clock class="inline h-3 w-3 mr-0.5" />{workflow.schedule}
										</span>
									{/if}
								</div>
								<div class="flex items-center gap-3 text-xs text-muted-foreground">
									{#if workflow.description}
										<span>{workflow.description}</span>
									{/if}
									<span>{workflow.steps.length !== 1 ? m.workflows_step_count_plural({ count: workflow.steps.length.toString() }) : m.workflows_step_count({ count: workflow.steps.length.toString() })}</span>
								</div>
							</div>
							<div class="flex items-center gap-1">
								<Button
									variant="ghost"
									size="icon"
									onclick={() => handleStartEdit(workflow)}
									title={m.workflows_edit_button_title()}
								>
									<Pencil class="h-4 w-4" />
								</Button>
								{#if workflowsStore.isRunning(workflow.id)}
									<Button
										variant="ghost"
										size="icon"
										onclick={() => handleCancel(workflow.id)}
										title={m.workflows_stop_button_title()}
									>
										<Square class="h-4 w-4 text-red-500" />
									</Button>
								{:else}
									<Button
										variant="ghost"
										size="icon"
										onclick={() => handleRun(workflow.id)}
										title={m.workflows_run_button_title()}
									>
										<Play class="h-4 w-4" />
									</Button>
								{/if}
								<Button
									variant="ghost"
									size="icon"
									onclick={() => handleShowHistory(workflow.id)}
									title={m.workflows_history_button_title()}
								>
									<History class="h-4 w-4" />
								</Button>
								<Button
									variant="ghost"
									size="icon"
									onclick={() => handleDelete(workflow.id)}
									title={m.workflows_delete_button_title()}
								>
									<Trash2 class="h-4 w-4 text-red-500" />
								</Button>
							</div>
						</div>
						{#if workflowsStore.isRunning(workflow.id)}
							{@const progress = workflowsStore.getProgress(workflow.id)}
							{#if progress}
								<div class="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
									<Loader2 class="h-3 w-3 animate-spin" />
									<span>
										{m.workflows_progress_steps({ completed: progress.completedSteps.length.toString(), total: workflow.steps.length.toString() })}
										{#if progress.completedSteps.length > 0}
											{m.workflows_progress_last({ stepName: progress.completedSteps[progress.completedSteps.length - 1].stepName })}
										{/if}
									</span>
								</div>
							{/if}
						{/if}
					</Card.Content>
				</Card.Root>

				<!-- Inline History for this workflow -->
				{#if showHistory === workflow.id}
					<Card.Root>
						<Card.Header>
							<div class="flex items-center justify-between">
								<Card.Title>{m.workflows_history_title()}</Card.Title>
								<Button variant="ghost" size="icon" onclick={() => (showHistory = null)}>
									<X class="h-4 w-4" />
								</Button>
							</div>
						</Card.Header>
						<Card.Content>
							{#if historyEntries.length === 0}
								<p class="text-sm text-muted-foreground">{m.workflows_history_empty()}</p>
							{:else}
								<div class="space-y-2 max-h-80 overflow-y-auto">
									{#each historyEntries as run (run.id)}
										<div class="border rounded-md p-3">
											<button
												class="w-full flex items-center justify-between text-sm cursor-pointer"
												onclick={() => toggleRunExpand(run.id)}
											>
												<div class="flex items-center gap-2">
													<span class="px-1.5 py-0.5 rounded text-xs {statusBadgeClass(run.status)}">
														{run.status}
													</span>
													{#if run.error}
														<span class="text-red-400 text-xs">{run.error}</span>
													{/if}
												</div>
												<div class="flex items-center gap-2">
													<span class="text-xs text-muted-foreground">
														{formatTime(run.started_at)}
													</span>
													{#if expandedRun === run.id}
														<ChevronUp class="h-4 w-4 text-muted-foreground" />
													{:else}
														<ChevronDown class="h-4 w-4 text-muted-foreground" />
													{/if}
												</div>
											</button>

											{#if expandedRun === run.id && run.step_results.length > 0}
												<div class="mt-3 space-y-2 border-t pt-3">
													{#each run.step_results as step}
														<div class="flex items-start gap-2 text-xs">
															{#if step.success}
																<CheckCircle2 class="h-3.5 w-3.5 text-green-500 mt-0.5 shrink-0" />
															{:else}
																<XCircle class="h-3.5 w-3.5 text-red-500 mt-0.5 shrink-0" />
															{/if}
															<div class="min-w-0 flex-1">
																<div class="flex items-center gap-2">
																	<span class="font-medium">{step.step_name}</span>
																	<span class="text-muted-foreground">{m.workflows_duration_ms({ value: step.duration_ms.toString() })}</span>
																</div>
																{#if step.error}
																	<p class="text-red-400 mt-0.5">{step.error}</p>
																{:else if step.output}
																	<p class="text-muted-foreground mt-0.5 truncate">{step.output}</p>
																{/if}
															</div>
														</div>
													{/each}
												</div>
											{/if}
										</div>
									{/each}
								</div>
							{/if}
						</Card.Content>
					</Card.Root>
				{/if}
			{/each}
		</div>
	{/if}
</div>

<ConfirmDialog
	bind:open={confirmOpen}
	title={m.workflows_delete_confirm_title()}
	description={m.workflows_delete_confirm_description()}
	onConfirm={confirmDelete}
/>
