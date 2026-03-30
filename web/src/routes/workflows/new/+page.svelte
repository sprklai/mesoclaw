<script lang="ts">
	import { onMount } from 'svelte';
	import { goto, beforeNavigate } from '$app/navigation';
	import WorkflowCanvas from '$lib/components/workflow-builder/WorkflowCanvas.svelte';
	import WorkflowToolbar from '$lib/components/workflow-builder/WorkflowToolbar.svelte';
	import NodePalette from '$lib/components/workflow-builder/NodePalette.svelte';
	import DynamicNodeForm from '$lib/components/workflow-builder/DynamicNodeForm.svelte';
	import CodeView from '$lib/components/workflow-builder/CodeView.svelte';
	import * as AlertDialog from '$lib/components/ui/alert-dialog';
	import { Button } from '$lib/components/ui/button';
	import { builderStore } from '$lib/stores/workflow-builder.svelte';
	import { workflowsStore } from '$lib/stores/workflows.svelte';
	import { graphToWorkflow, workflowToToml } from '$lib/components/workflow-builder/graph-utils';
	import { exportWorkflowToml } from '$lib/components/workflow-builder/import-export';
	import { t } from '$lib/components/workflow-builder/i18n-utils';
	import { toast } from 'svelte-sonner';
	import * as m from '$lib/paraglide/messages';

	let codeContent = $state('');
	let codeError = $state<string | null>(null);
	let unsavedDialogOpen = $state(false);
	let pendingNavigationUrl = $state<string | null>(null);

	onMount(() => {
		builderStore.reset();
	});

	beforeNavigate((navigation) => {
		if (builderStore.isDirty) {
			navigation.cancel();
			pendingNavigationUrl = navigation.to?.url.pathname ?? '/workflows';
			unsavedDialogOpen = true;
		}
	});

	function handleLeave() {
		unsavedDialogOpen = false;
		const url = pendingNavigationUrl ?? '/workflows';
		pendingNavigationUrl = null;
		builderStore.reset(); // clears isDirty so beforeNavigate won't re-trigger
		goto(url);
	}

	function handleBeforeUnload(e: BeforeUnloadEvent) {
		if (builderStore.isDirty) {
			e.preventDefault();
		}
	}

	function handleKeyDown(e: KeyboardEvent) {
		if ((e.ctrlKey || e.metaKey) && e.key === 's') {
			e.preventDefault();
			handleSave();
		}
	}

	async function handleSave() {
		const wf = graphToWorkflow(builderStore.nodes, builderStore.edges, {
			name: builderStore.workflowName || 'Untitled',
			description: builderStore.workflowDescription,
			schedule: builderStore.workflowSchedule
		});

		const toml = workflowToToml(wf);

		try {
			const created = await workflowsStore.create(toml);
			builderStore.markSaved(created.id);
			goto(`/workflows/${created.id}`);
		} catch (err) {
			toast.error(err instanceof Error ? err.message : 'Failed to save workflow');
		}
	}

	async function handleRun() {
		if (!builderStore.workflowId) {
			await handleSave();
		}
		if (builderStore.workflowId) {
			builderStore.setRunning(true);
			try {
				await workflowsStore.run(builderStore.workflowId);
			} finally {
				builderStore.setRunning(false);
			}
		}
	}

	async function handleExport() {
		if (builderStore.workflowId) {
			const raw = await workflowsStore.getRawToml(builderStore.workflowId);
			exportWorkflowToml(raw, builderStore.workflowName || 'workflow');
		}
	}

	function handleBack() {
		goto('/workflows');
	}

	function handleCodeInput(value: string) {
		codeContent = value;
		codeError = null;
	}
</script>

<svelte:window onbeforeunload={handleBeforeUnload} onkeydown={handleKeyDown} />

<div class="flex flex-col h-[calc(100vh-3.5rem)]">
	<WorkflowToolbar
		onSave={handleSave}
		onRun={handleRun}
		onExport={handleExport}
		onBack={handleBack}
	/>

	{#if builderStore.viewMode === 'code'}
		<CodeView
			value={codeContent}
			oninput={handleCodeInput}
			error={codeError}
		/>
	{:else}
		<div class="flex flex-1 min-h-0">
			<div class="w-48 shrink-0">
				<NodePalette />
			</div>
			<WorkflowCanvas />
			{#if builderStore.selectedNodeId}
				<div class="w-64 shrink-0 border-l">
					<DynamicNodeForm />
				</div>
			{/if}
		</div>
	{/if}
</div>

<AlertDialog.Root bind:open={unsavedDialogOpen}>
	<AlertDialog.Content>
		<AlertDialog.Header>
			<AlertDialog.Title>{t('wb_unsaved_confirm')}</AlertDialog.Title>
			<AlertDialog.Description>{t('wb_unsaved_confirm_desc')}</AlertDialog.Description>
		</AlertDialog.Header>
		<AlertDialog.Footer>
			<AlertDialog.Cancel>{m.common_cancel()}</AlertDialog.Cancel>
			<Button variant="destructive" onclick={handleLeave}>{t('wb_unsaved_leave')}</Button>
		</AlertDialog.Footer>
	</AlertDialog.Content>
</AlertDialog.Root>
