<script lang="ts">
	import { nodeRegistry, type NodeFieldDef } from './node-registry';
	import { builderStore } from '$lib/stores/workflow-builder.svelte';
	import { Switch } from '$lib/components/ui/switch';
	import { Button } from '$lib/components/ui/button';
	import { t } from './i18n-utils';

	const node = $derived(builderStore.selectedNode);
	const definition = $derived(node ? nodeRegistry.get(node.data.definitionType as string) : undefined);

	function getNodeStepNames(): string[] {
		return builderStore.nodes
			.filter(n => n.id !== node?.id)
			.map(n => (n.data.stepName as string) || n.id);
	}

	function updateField(key: string, value: unknown) {
		if (!node) return;
		builderStore.updateNodeData(node.id, { [key]: value });
	}

	function updateStepName(value: string) {
		if (!node) return;
		builderStore.renameNode(node.id, value);
	}

	function updateCommon(key: string, value: unknown) {
		if (!node) return;
		builderStore.updateNodeData(node.id, { [key]: value });
	}

	function toggleStepRef(key: string, stepName: string) {
		if (!node) return;
		const current = (node.data[key] as string[]) || [];
		const next = current.includes(stepName)
			? current.filter(s => s !== stepName)
			: [...current, stepName];
		builderStore.updateNodeData(node.id, { [key]: next });
	}
</script>

<div class="p-4 space-y-4 overflow-y-auto h-full">
	{#if !node || !definition}
		<p class="text-sm text-muted-foreground">{t('wb_config_no_selection')}</p>
	{:else}
		<h3 class="text-sm font-semibold">{t(definition.label)}</h3>

		<!-- Step Name (always present) -->
		<div class="space-y-1">
			<label class="text-xs font-medium text-muted-foreground">{t('wb_config_name_label')}</label>
			<input
				type="text"
				value={node.data.stepName ?? ''}
				oninput={(e) => updateStepName((e.target as HTMLInputElement).value)}
				placeholder={t('wb_config_name_placeholder')}
				class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
			/>
		</div>

		<!-- Dynamic fields from definition -->
		{#each definition.fields as field (field.key)}
			<div class="space-y-1">
				<label class="text-xs font-medium text-muted-foreground">
					{t(field.label)}
					{#if field.required}<span class="text-red-400">*</span>{/if}
				</label>

				{#if field.type === 'text'}
					<input
						type="text"
						value={String(node.data[field.key] ?? field.default ?? '')}
						oninput={(e) => updateField(field.key, (e.target as HTMLInputElement).value)}
						placeholder={field.placeholder ? t(field.placeholder) : ''}
						class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
					/>

				{:else if field.type === 'textarea'}
					<textarea
						value={String(node.data[field.key] ?? field.default ?? '')}
						oninput={(e) => updateField(field.key, (e.target as HTMLTextAreaElement).value)}
						placeholder={field.placeholder ? t(field.placeholder) : ''}
						rows="3"
						class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm font-mono resize-y"
					></textarea>

				{:else if field.type === 'number'}
					<input
						type="number"
						value={String(node.data[field.key] ?? field.default ?? '')}
						oninput={(e) => updateField(field.key, Number((e.target as HTMLInputElement).value))}
						class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
					/>

				{:else if field.type === 'select'}
					<select
						value={String(node.data[field.key] ?? field.default ?? '')}
						onchange={(e) => updateField(field.key, (e.target as HTMLSelectElement).value)}
						class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
					>
						{#if field.options}
							{#each field.options as opt}
								<option value={opt.value}>{t(opt.label)}</option>
							{/each}
						{/if}
					</select>

				{:else if field.type === 'boolean'}
					<div class="flex items-center gap-2">
						<Switch
							checked={node.data[field.key] === true}
							onCheckedChange={(v) => updateField(field.key, v)}
						/>
						<span class="text-xs text-muted-foreground">{node.data[field.key] ? 'On' : 'Off'}</span>
					</div>

				{:else if field.type === 'json'}
					<textarea
						value={typeof node.data[field.key] === 'string' ? String(node.data[field.key]) : JSON.stringify(node.data[field.key] ?? field.default ?? {}, null, 2)}
						oninput={(e) => {
							try {
								const parsed = JSON.parse((e.target as HTMLTextAreaElement).value);
								updateField(field.key, parsed);
							} catch {
								// Keep raw text until valid
							}
						}}
						rows="4"
						class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm font-mono resize-y"
					></textarea>

				{:else if field.type === 'step-ref'}
					<select
						value={String(node.data[field.key] ?? '')}
						onchange={(e) => updateField(field.key, (e.target as HTMLSelectElement).value)}
						class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
					>
						<option value="">—</option>
						{#each getNodeStepNames() as name}
							<option value={name}>{name}</option>
						{/each}
					</select>

				{:else if field.type === 'step-refs'}
					<div class="space-y-1 max-h-32 overflow-y-auto">
						{#each getNodeStepNames() as name}
							<label class="flex items-center gap-2 text-xs">
								<input
									type="checkbox"
									checked={((node.data[field.key] as string[]) || []).includes(name)}
									onchange={() => toggleStepRef(field.key, name)}
									class="rounded border"
								/>
								{name}
							</label>
						{/each}
					</div>
				{/if}
			</div>
		{/each}

		<!-- Common fields -->
		<div class="border-t pt-3 mt-3 space-y-3">
			<div class="space-y-1">
				<label class="text-xs font-medium text-muted-foreground">{t('wb_config_timeout_label')}</label>
				<input
					type="number"
					value={String(node.data.timeout_secs ?? '')}
					oninput={(e) => updateCommon('timeout_secs', Number((e.target as HTMLInputElement).value) || undefined)}
					class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
				/>
			</div>
			<div class="space-y-1">
				<label class="text-xs font-medium text-muted-foreground">{t('wb_config_retry_label')}</label>
				<input
					type="number"
					value={String(node.data.retry ?? '')}
					oninput={(e) => updateCommon('retry', Number((e.target as HTMLInputElement).value) || undefined)}
					class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
				/>
			</div>
			<div class="space-y-1">
				<label class="text-xs font-medium text-muted-foreground">{t('wb_config_failure_policy_label')}</label>
				<select
					value={String(node.data.failure_policy ?? 'stop')}
					onchange={(e) => updateCommon('failure_policy', (e.target as HTMLSelectElement).value)}
					class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
				>
					<option value="stop">{t('wb_config_failure_policy_stop')}</option>
					<option value="continue">{t('wb_config_failure_policy_continue')}</option>
					<option value="fallback">{t('wb_config_failure_policy_fallback')}</option>
				</select>
			</div>
		</div>
	{/if}
</div>
