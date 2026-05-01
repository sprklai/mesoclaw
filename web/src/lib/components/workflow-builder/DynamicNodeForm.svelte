<script lang="ts">
	import { nodeRegistry } from './node-registry';
	import { builderStore } from '$lib/stores/workflow-builder.svelte';
	import { Switch } from '$lib/components/ui/switch';
	import { t } from './i18n-utils';

	const node = $derived(builderStore.selectedNode);
	const definition = $derived(node ? nodeRegistry.get(node.data.definitionType as string) : undefined);

	const fpValue = $derived.by(() => {
		if (!node) return 'stop';
		const fp = node.data.failure_policy;
		if (!fp || fp === 'stop') return 'stop';
		if (fp === 'continue') return 'continue';
		if (typeof fp === 'object' && (fp as Record<string, unknown>).fallback) return 'fallback';
		return String(fp);
	});

	/** Step name validation: only [a-z0-9_], non-empty */
	const STEP_NAME_RE = /^[a-z0-9_]+$/;
	const stepNameValue = $derived((node?.data.stepName as string) ?? '');
	const stepNameError = $derived.by(() => {
		if (!stepNameValue) return t('wb_validation_step_name_empty');
		if (!STEP_NAME_RE.test(stepNameValue)) return t('wb_validation_step_name_invalid');
		return '';
	});

	/** Fallback step validation */
	const fallbackStep = $derived.by(() => {
		if (!node) return '';
		const fp = node.data.failure_policy;
		if (typeof fp === 'object' && fp !== null) {
			return ((fp as Record<string, Record<string, string>>).fallback?.step) ?? '';
		}
		return '';
	});
	const fallbackStepError = $derived(
		fpValue === 'fallback' && !fallbackStep ? t('wb_validation_fallback_step_required') : ''
	);

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
			<label for="step-name-{node.id}" class="text-xs font-medium text-muted-foreground">{t('wb_config_name_label')}</label>
			<input
				id="step-name-{node.id}"
				type="text"
				value={node.data.stepName ?? ''}
				oninput={(e) => updateStepName((e.target as HTMLInputElement).value)}
				placeholder={t('wb_config_name_placeholder')}
				class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm {stepNameError ? 'border-red-500' : ''}"
			/>
			{#if stepNameError}
				<p class="text-xs text-red-500">{stepNameError}</p>
			{/if}
		</div>

		<!-- Dynamic fields from definition -->
		{#each definition.fields as field (field.key)}
			<div class="space-y-1">
				<label for="{node.id}-{field.key}" class="text-xs font-medium text-muted-foreground">
					{t(field.label)}
					{#if field.required}<span class="text-red-400">*</span>{/if}
				</label>

				{#if field.type === 'text'}
					<input
						id="{node.id}-{field.key}"
						type="text"
						value={String(node.data[field.key] ?? field.default ?? '')}
						oninput={(e) => updateField(field.key, (e.target as HTMLInputElement).value)}
						placeholder={field.placeholder ? t(field.placeholder) : ''}
						class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
					/>

				{:else if field.type === 'textarea'}
					<textarea
						id="{node.id}-{field.key}"
						value={String(node.data[field.key] ?? field.default ?? '')}
						oninput={(e) => updateField(field.key, (e.target as HTMLTextAreaElement).value)}
						placeholder={field.placeholder ? t(field.placeholder) : ''}
						rows="3"
						class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm font-mono resize-y"
					></textarea>

				{:else if field.type === 'number'}
					<input
						id="{node.id}-{field.key}"
						type="number"
						value={String(node.data[field.key] ?? field.default ?? '')}
						oninput={(e) => updateField(field.key, Number((e.target as HTMLInputElement).value))}
						class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
					/>

				{:else if field.type === 'select'}
					<select
						id="{node.id}-{field.key}"
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
						id="{node.id}-{field.key}"
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
						id="{node.id}-{field.key}"
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
				<label for="timeout-{node.id}" class="text-xs font-medium text-muted-foreground">{t('wb_config_timeout_label')}</label>
				<input
					id="timeout-{node.id}"
					type="number"
					value={String(node.data.timeout_secs ?? '')}
					oninput={(e) => updateCommon('timeout_secs', Number((e.target as HTMLInputElement).value) || undefined)}
					class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
				/>
			</div>
			<div class="space-y-1">
				<label for="retry-max-{node.id}" class="text-xs font-medium text-muted-foreground">{t('wb_config_retry_max_label')}</label>
				<input
					id="retry-max-{node.id}"
					type="number"
					value={String(((node.data.retry as Record<string, unknown>)?.max_retries) ?? '')}
					oninput={(e) => {
						const val = Number((e.target as HTMLInputElement).value);
						if (!val) { updateCommon('retry', undefined); return; }
						const prev = (node.data.retry as Record<string, unknown>) ?? {};
						updateCommon('retry', { max_retries: val, retry_delay_ms: Number(prev.retry_delay_ms ?? 1000) });
					}}
					placeholder="3"
					class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
				/>
			</div>
			<div class="space-y-1">
				<label for="retry-delay-{node.id}" class="text-xs font-medium text-muted-foreground">{t('wb_config_retry_delay_label')}</label>
				<input
					id="retry-delay-{node.id}"
					type="number"
					value={String(((node.data.retry as Record<string, unknown>)?.retry_delay_ms) ?? '')}
					oninput={(e) => {
						const val = Number((e.target as HTMLInputElement).value);
						const prev = (node.data.retry as Record<string, unknown>) ?? {};
						const maxRetries = Number(prev.max_retries ?? 3);
						if (!val && !maxRetries) { updateCommon('retry', undefined); return; }
						updateCommon('retry', { max_retries: maxRetries, retry_delay_ms: val || 1000 });
					}}
					placeholder="1000"
					class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
				/>
			</div>
			<div class="space-y-1">
				<label for="failure-policy-{node.id}" class="text-xs font-medium text-muted-foreground">{t('wb_config_failure_policy_label')}</label>
				<select
					id="failure-policy-{node.id}"
					value={fpValue}
					onchange={(e) => {
						const val = (e.target as HTMLSelectElement).value;
						if (val === 'fallback') {
							updateCommon('failure_policy', { fallback: { step: '' } });
						} else {
							updateCommon('failure_policy', val);
						}
					}}
					class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm"
				>
					<option value="stop">{t('wb_config_failure_policy_stop')}</option>
					<option value="continue">{t('wb_config_failure_policy_continue')}</option>
					<option value="fallback">{t('wb_config_failure_policy_fallback')}</option>
				</select>
			</div>
			{#if fpValue === 'fallback'}
				<div class="space-y-1">
					<label for="fallback-step-{node.id}" class="text-xs font-medium text-muted-foreground">{t('wb_config_fallback_step_label')}</label>
					<select
						id="fallback-step-{node.id}"
						value={String((node.data.failure_policy as Record<string, Record<string, string>>)?.fallback?.step ?? '')}
						onchange={(e) => updateCommon('failure_policy', { fallback: { step: (e.target as HTMLSelectElement).value } })}
						class="w-full rounded-md border bg-background text-foreground px-2 py-1.5 text-sm {fallbackStepError ? 'border-red-500' : ''}"
					>
						<option value="">{t('wb_validation_fallback_step_required')}</option>
						{#each getNodeStepNames() as name}
							<option value={name}>{name}</option>
						{/each}
					</select>
					{#if fallbackStepError}
						<p class="text-xs text-red-500">{fallbackStepError}</p>
					{/if}
				</div>
			{/if}
		</div>
	{/if}
</div>
