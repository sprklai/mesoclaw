<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import ArrowUpRight from '@lucide/svelte/icons/arrow-up-right';
	import { providersStore, type ProviderWithKeyStatus } from '$lib/stores/providers.svelte';
	import { configStore } from '$lib/stores/config.svelte';
	import {
		PromptInputModelSelect,
		PromptInputModelSelectTrigger,
		PromptInputModelSelectContent,
		PromptInputModelSelectItem,
		PromptInputModelSelectValue
	} from '$lib/components/ai-elements/prompt-input';
	import { onMount } from 'svelte';
	import * as m from '$lib/paraglide/messages';

	let { hideDefaultModel = false }: { hideDefaultModel?: boolean } = $props();

	const providerDocsUrls: Record<string, string> = {
		'openai': 'https://developers.openai.com/api/docs/models',
		'anthropic': 'https://platform.claude.com/docs/en/about-claude/models/overview',
		'gemini': 'https://ai.google.dev/gemini-api/docs/models',
		'openrouter': 'https://openrouter.ai/models',
		'vercel-ai-gateway': 'https://vercel.com/ai-gateway/models',
		'ollama': 'https://ollama.com/search',
	};

	let expandedId = $state<string | null>(null);
	let apiKeyInputs = $state<Record<string, string>>({});
	let showKey = $state<Record<string, boolean>>({});
	let saving = $state<Record<string, boolean>>({});
	let newModelInputs = $state<Record<string, string>>({});
	let addingModel = $state<Record<string, boolean>>({});
	let testing = $state<Record<string, boolean>>({});
	let testResult = $state<Record<string, { success: boolean; message: string; latency_ms?: number } | null>>({});
	let showAddProvider = $state(false);
	let newProvider = $state({ id: '', name: '', baseUrl: '', requiresApiKey: true });
	let addingProvider = $state(false);
	let deletingProvider = $state<Record<string, boolean>>({});
	let confirmOpen = $state(false);
	let deleteAction = $state<{ type: 'key' | 'provider' | 'model'; id: string; extra?: string } | null>(null);

	onMount(() => {
		providersStore.load();
		providersStore.loadDefault();
		configStore.load();
	});

	const currentModelLabel = $derived(
		providersStore.configuredModels.find((m) => m.value === providersStore.selectedModel)?.label ??
			''
	);

	const defaultProviderMissingKey = $derived.by(() => {
		const providerName = configStore.get('provider_name') as string | undefined;
		if (!providerName) return false;
		const provider = providersStore.providers.find((p) => p.id === providerName);
		if (!provider) return false;
		return provider.requires_api_key && !provider.has_api_key;
	});

	async function handleDefaultModelChange(value: string | undefined) {
		if (!value) return;
		providersStore.selectedModel = value;
		const [providerId, ...rest] = value.split(':');
		const modelId = rest.join(':');
		await providersStore.setDefault(providerId, modelId);
		await configStore.update({
			provider_name: providerId,
			provider_type: providerId,
			provider_model_id: modelId
		});
	}

	function toggle(id: string) {
		expandedId = expandedId === id ? null : id;
	}

	async function saveKey(provider: ProviderWithKeyStatus) {
		const value = apiKeyInputs[provider.id];
		if (!value?.trim()) return;
		saving[provider.id] = true;
		try {
			await providersStore.setApiKey(provider.id, value.trim());
			apiKeyInputs[provider.id] = '';
		} finally {
			saving[provider.id] = false;
		}
	}

	async function testConnection(provider: ProviderWithKeyStatus) {
		testing[provider.id] = true;
		testResult[provider.id] = null;
		try {
			testResult[provider.id] = await providersStore.testConnection(provider.id);
		} catch (e) {
			testResult[provider.id] = { success: false, message: String(e) };
		} finally {
			testing[provider.id] = false;
		}
	}

	function removeKey(provider: ProviderWithKeyStatus) {
		deleteAction = { type: 'key', id: provider.id };
		confirmOpen = true;
	}

	async function confirmRemoveKey(providerId: string) {
		saving[providerId] = true;
		try {
			await providersStore.removeApiKey(providerId);
		} finally {
			saving[providerId] = false;
		}
	}

	async function addModel(providerId: string) {
		const modelId = newModelInputs[providerId];
		if (!modelId?.trim()) return;
		addingModel[providerId] = true;
		try {
			await providersStore.addModel(providerId, modelId.trim());
			newModelInputs[providerId] = '';
		} finally {
			addingModel[providerId] = false;
		}
	}

	function deleteModel(providerId: string, modelId: string) {
		deleteAction = { type: 'model', id: providerId, extra: modelId };
		confirmOpen = true;
	}

	async function confirmDeleteModel(providerId: string, modelId: string) {
		addingModel[providerId] = true;
		try {
			await providersStore.deleteModel(providerId, modelId);
		} finally {
			addingModel[providerId] = false;
		}
	}

	async function addProvider() {
		if (!newProvider.id.trim() || !newProvider.name.trim() || !newProvider.baseUrl.trim()) return;
		addingProvider = true;
		try {
			await providersStore.addProvider(
				newProvider.id.trim(),
				newProvider.name.trim(),
				newProvider.baseUrl.trim(),
				newProvider.requiresApiKey,
			);
			newProvider = { id: '', name: '', baseUrl: '', requiresApiKey: true };
			showAddProvider = false;
		} finally {
			addingProvider = false;
		}
	}

	function deleteProvider(id: string) {
		deleteAction = { type: 'provider', id };
		confirmOpen = true;
	}

	async function confirmDeleteProvider(id: string) {
		deletingProvider[id] = true;
		try {
			await providersStore.deleteProvider(id);
			if (expandedId === id) expandedId = null;
		} finally {
			deletingProvider[id] = false;
		}
	}

	async function confirmDelete() {
		if (!deleteAction) return;
		const action = deleteAction;
		if (action.type === 'key') await confirmRemoveKey(action.id);
		else if (action.type === 'provider') await confirmDeleteProvider(action.id);
		else if (action.type === 'model' && action.extra) await confirmDeleteModel(action.id, action.extra);
	}

	function confirmTitle(): string {
		if (!deleteAction) return '';
		if (deleteAction.type === 'key') return m.settings_providers_confirm_remove_key_title();
		if (deleteAction.type === 'provider') return m.settings_providers_confirm_delete_provider_title();
		return m.settings_providers_confirm_delete_model_title();
	}

	function confirmDescription(): string {
		if (!deleteAction) return '';
		if (deleteAction.type === 'key') return m.settings_providers_confirm_remove_key_description();
		if (deleteAction.type === 'provider') return m.settings_providers_confirm_delete_provider_description();
		return m.settings_providers_confirm_delete_model_description();
	}

	function statusBadge(provider: ProviderWithKeyStatus): { label: string; variant: 'default' | 'secondary' | 'outline' | 'destructive' } {
		if (!provider.requires_api_key) return { label: m.settings_providers_badge_local(), variant: 'outline' };
		if (provider.has_api_key) return { label: m.settings_providers_badge_configured(), variant: 'default' };
		return { label: m.settings_providers_badge_not_configured(), variant: 'secondary' };
	}
</script>

{#if !hideDefaultModel}
	<div class="flex items-center justify-between mb-2">
		<h2 class="text-lg font-semibold">{m.settings_providers_title()}</h2>
		<Button size="sm" variant="outline" onclick={() => (showAddProvider = !showAddProvider)}>
			{showAddProvider ? m.settings_providers_cancel_button() : m.settings_providers_add_button()}
		</Button>
	</div>
	<p class="mb-1 text-sm text-muted-foreground">
		{m.settings_providers_instructions()}
	</p>
	<p class="mb-4 text-xs text-muted-foreground">
		{m.settings_providers_custom_hint()}
	</p>
{:else}
	<div class="flex justify-end mb-2">
		<Button size="sm" variant="outline" onclick={() => (showAddProvider = !showAddProvider)}>
			{showAddProvider ? m.settings_providers_cancel_button() : m.settings_providers_add_button()}
		</Button>
	</div>
{/if}

{#if !hideDefaultModel && defaultProviderMissingKey}
	<div class="mb-4 rounded-md border border-amber-500/50 bg-amber-500/10 px-4 py-3 text-sm text-amber-700 dark:text-amber-400">
		<strong>{m.settings_providers_missing_key_warning()}</strong> Your config defaults to
		<code class="rounded bg-amber-500/20 px-1">{configStore.get('provider_name')}</code> /
		<code class="rounded bg-amber-500/20 px-1">{configStore.get('provider_model_id')}</code>,
		{m.settings_providers_missing_key_detail()}
	</div>
{/if}

{#if !hideDefaultModel && providersStore.configuredModels.length > 0}
	<Card.Root class="mb-4">
		<Card.Header class="py-3">
			<Card.Title class="text-base">{m.settings_providers_default_model_title()}</Card.Title>
			<Card.Description class="text-sm">
				{m.settings_providers_default_model_description()}
			</Card.Description>
		</Card.Header>
		<Card.Content class="pt-0">
			<PromptInputModelSelect
				value={providersStore.selectedModel}
				onValueChange={handleDefaultModelChange}
			>
				<PromptInputModelSelectTrigger class="w-full border border-border">
					<PromptInputModelSelectValue
						value={currentModelLabel}
						placeholder={m.settings_providers_default_model_placeholder()}
					/>
				</PromptInputModelSelectTrigger>
				<PromptInputModelSelectContent>
					{#each providersStore.configuredModels as model}
						<PromptInputModelSelectItem value={model.value}>
							{model.label}
						</PromptInputModelSelectItem>
					{/each}
				</PromptInputModelSelectContent>
			</PromptInputModelSelect>
		</Card.Content>
	</Card.Root>
{/if}

{#if showAddProvider}
	<Card.Root>
		<Card.Header>
			<Card.Title>{m.settings_providers_add_custom_title()}</Card.Title>
		</Card.Header>
		<Card.Content class="space-y-3">
			<div class="grid grid-cols-2 gap-3">
				<div class="space-y-1">
					<label class="text-sm font-medium" for="new-id">{m.settings_providers_provider_id_label()}</label>
					<Input id="new-id" placeholder={m.settings_providers_provider_id_placeholder()} bind:value={newProvider.id} />
				</div>
				<div class="space-y-1">
					<label class="text-sm font-medium" for="new-name">{m.settings_providers_display_name_label()}</label>
					<Input id="new-name" placeholder={m.settings_providers_display_name_placeholder()} bind:value={newProvider.name} />
				</div>
			</div>
			<div class="space-y-1">
				<label class="text-sm font-medium" for="new-url">{m.settings_providers_base_url_label()}</label>
				<Input id="new-url" placeholder={m.settings_providers_base_url_placeholder()} bind:value={newProvider.baseUrl} />
			</div>
			<div class="flex items-center gap-2">
				<input
					type="checkbox"
					id="new-requires-key"
					bind:checked={newProvider.requiresApiKey}
					class="rounded"
				/>
				<label class="text-sm" for="new-requires-key">{m.settings_providers_requires_api_key_label()}</label>
			</div>
			<Button
				size="sm"
				disabled={!newProvider.id.trim() || !newProvider.name.trim() || !newProvider.baseUrl.trim() || addingProvider}
				onclick={addProvider}
			>
				{addingProvider ? m.settings_providers_adding_button() : m.settings_providers_add_provider_button()}
			</Button>
		</Card.Content>
	</Card.Root>
{/if}

{#if providersStore.loading}
	<div class="space-y-2">
		<Skeleton class="h-16 w-full" />
		<Skeleton class="h-16 w-full" />
		<Skeleton class="h-16 w-full" />
	</div>
{:else if providersStore.providers.length === 0}
	<p class="text-muted-foreground">{m.settings_providers_no_providers()}</p>
{:else}
	<div class="space-y-2">
		{#each providersStore.providers as provider (provider.id)}
			{@const status = statusBadge(provider)}
			<Card.Root>
				<button
					class="w-full text-left"
					onclick={() => toggle(provider.id)}
				>
					<Card.Header class="py-3">
						<div class="flex items-center justify-between">
							<div class="flex items-center gap-2">
								<Card.Title class="text-base">{provider.name}</Card.Title>
								<Badge variant={status.variant}>{status.label}</Badge>
							</div>
							<span class="text-xs text-muted-foreground">
								{expandedId === provider.id ? '▲' : '▼'}
							</span>
						</div>
					</Card.Header>
				</button>

				{#if expandedId === provider.id}
					<Card.Content class="pt-0 space-y-4">
						<div class="flex items-center justify-between">
							<div class="text-sm text-muted-foreground">
								<span class="font-medium">{m.settings_providers_base_url_prefix()}</span> {provider.base_url}
							</div>
							{#if provider.is_user_defined}
								<Button
									variant="destructive"
									size="sm"
									disabled={deletingProvider[provider.id]}
									onclick={() => deleteProvider(provider.id)}
								>
									{deletingProvider[provider.id] ? m.settings_providers_deleting_button() : m.settings_providers_delete_provider_button()}
								</Button>
							{/if}
						</div>

						{#if provider.requires_api_key}
							<div class="space-y-2">
								<label class="text-sm font-medium" for="key-{provider.id}">{m.settings_providers_api_key_label()}</label>
								<div class="flex gap-2">
									<Input
										id="key-{provider.id}"
										type={showKey[provider.id] ? 'text' : 'password'}
										placeholder={provider.has_api_key ? m.settings_providers_api_key_placeholder_set() : m.settings_providers_api_key_placeholder_empty()}
										bind:value={apiKeyInputs[provider.id]}
									/>
									<Button
										variant="ghost"
										size="sm"
										onclick={() => (showKey[provider.id] = !showKey[provider.id])}
									>
										{showKey[provider.id] ? m.settings_providers_hide_button() : m.settings_providers_show_button()}
									</Button>
								</div>
								<div class="flex gap-2">
									<Button
										size="sm"
										disabled={!apiKeyInputs[provider.id]?.trim() || saving[provider.id]}
										onclick={() => saveKey(provider)}
									>
										{saving[provider.id] ? m.settings_providers_saving_button() : m.settings_providers_save_key_button()}
									</Button>
									{#if provider.has_api_key}
										<Button
											variant="destructive"
											size="sm"
											disabled={saving[provider.id]}
											onclick={() => removeKey(provider)}
										>
											{m.settings_providers_remove_key_button()}
										</Button>
										<Button
											variant="outline"
											size="sm"
											disabled={testing[provider.id]}
											onclick={() => testConnection(provider)}
										>
											{testing[provider.id] ? m.settings_providers_testing_button() : m.settings_providers_test_connection_button()}
										</Button>
									{/if}
								</div>
								{#if testResult[provider.id] != null}
									{@const result = testResult[provider.id]!}
									<p class="text-sm {result.success ? 'text-green-600 dark:text-green-400' : 'text-red-600 dark:text-red-400'}">
										{#if result.success}
											{result.latency_ms ? m.settings_providers_connected_latency({ latency_ms: String(result.latency_ms) }) : m.settings_providers_connected()}
										{:else}
											{result.message}
										{/if}
									</p>
								{/if}
							</div>
						{:else}
							<p class="text-sm text-muted-foreground italic">{m.settings_providers_no_api_key_required()}</p>
						{/if}

						<div class="space-y-2">
							<span class="text-sm font-medium inline-flex items-center gap-1.5">
								{m.settings_providers_models_label()}
								{#if providerDocsUrls[provider.id]}
									<a
										href={providerDocsUrls[provider.id]}
										target="_blank"
										rel="noopener noreferrer"
										class="text-muted-foreground hover:text-primary transition-colors"
										title={m.settings_providers_browse_model_ids_tooltip()}
									>
										<ArrowUpRight class="h-3.5 w-3.5" />
									</a>
								{/if}
							</span>
							{#if provider.models.length > 0}
								<div class="flex flex-wrap gap-1">
									{#each provider.models as model (model.id)}
										<Badge variant="outline" class="text-xs flex items-center gap-1">
											{model.display_name}
											{#if model.is_custom}
												<button
													class="ml-1 text-muted-foreground hover:text-destructive"
													title={m.settings_providers_remove_model_tooltip()}
													onclick={() => deleteModel(provider.id, model.model_id)}
												>&times;</button>
											{/if}
										</Badge>
									{/each}
								</div>
							{:else}
								<p class="text-xs text-muted-foreground">{m.settings_providers_no_models()}</p>
							{/if}
							<div class="flex gap-2">
								<Input
									class="h-8 text-sm"
									placeholder={m.settings_providers_add_model_placeholder()}
									bind:value={newModelInputs[provider.id]}
								/>
								<Button
									size="sm"
									variant="outline"
									disabled={!newModelInputs[provider.id]?.trim() || addingModel[provider.id]}
									onclick={() => addModel(provider.id)}
								>
									{addingModel[provider.id] ? m.settings_providers_add_model_adding() : m.settings_providers_add_model_button()}
								</Button>
							</div>
						</div>
					</Card.Content>
				{/if}
			</Card.Root>
		{/each}
	</div>
{/if}

<ConfirmDialog
	bind:open={confirmOpen}
	title={confirmTitle()}
	description={confirmDescription()}
	confirmLabel={m.settings_providers_confirm_remove_label()}
	onConfirm={confirmDelete}
/>
