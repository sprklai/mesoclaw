<script lang="ts">
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import Cpu from '@lucide/svelte/icons/cpu';
	import User from '@lucide/svelte/icons/user';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Brain from '@lucide/svelte/icons/brain';
	import Puzzle from '@lucide/svelte/icons/puzzle';
	import Network from '@lucide/svelte/icons/network';
	import FileText from '@lucide/svelte/icons/file-text';
	import Bell from '@lucide/svelte/icons/bell';
	import Shield from '@lucide/svelte/icons/shield';
	import Info from '@lucide/svelte/icons/info';
	import Download from '@lucide/svelte/icons/download';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import type { Component } from 'svelte';
	import { Separator } from '$lib/components/ui/separator';
	import * as Dialog from '$lib/components/ui/dialog';
	import { getAppVersion, checkForUpdate, installUpdate, onUpdateAvailable, openInBrowser } from '$lib/tauri';
	import type { UpdateInfo } from '$lib/tauri';
	import { onMount } from 'svelte';
	import * as m from '$lib/paraglide/messages';

	const tabLoaders: Record<string, () => Promise<{ default: Component }>> = {
		general: () => import('$lib/components/settings/GeneralSettings.svelte'),
		providers: () => import('$lib/components/settings/ProvidersSettings.svelte'),
		persona: () => import('$lib/components/settings/PersonaSettings.svelte'),
		channels: () => import('$lib/components/settings/ChannelsSettings.svelte'),
		permissions: () => import('$lib/components/settings/PermissionsSettings.svelte'),
		notifications: () => import('$lib/components/settings/NotificationsSettings.svelte'),
		services: () => import('$lib/components/settings/ServicesSettings.svelte'),
		embeddings: () => import('$lib/components/settings/EmbeddingsSettings.svelte'),
		configurations: () => import('$lib/components/settings/ConfigurationsSettings.svelte'),
		plugins: () => import('$lib/components/settings/PluginsSettings.svelte'),
		mcp: () => import('$lib/components/settings/McpSettings.svelte'),
	};

	const componentCache = new Map<string, Component>();

	async function loadTab(tabId: string): Promise<Component> {
		const cached = componentCache.get(tabId);
		if (cached) return cached;
		const loader = tabLoaders[tabId];
		if (!loader) throw new Error(`Unknown tab: ${tabId}`);
		const mod = await loader();
		componentCache.set(tabId, mod.default);
		return mod.default;
	}

	const tabs = [
		{ id: 'general', label: m.settings_tab_general(), icon: SettingsIcon },
		{ id: 'providers', label: m.settings_tab_providers(), icon: Cpu },
		{ id: 'persona', label: m.settings_tab_persona(), icon: User },
		{ id: 'channels', label: m.settings_tab_channels(), icon: MessageSquare },
		{ id: 'permissions', label: m.settings_tab_permissions(), icon: Shield },
		{ id: 'notifications', label: m.settings_tab_notifications(), icon: Bell },
		{ id: 'services', label: m.settings_tab_services(), icon: KeyRound },
		{ id: 'embeddings', label: m.settings_tab_embeddings(), icon: Brain },
		{ id: 'configurations', label: m.settings_tab_configurations(), icon: FileText },
		{ id: 'plugins', label: m.settings_tab_plugins(), icon: Puzzle },
		{ id: 'mcp', label: 'MCP', icon: Network },
	];

	let activeTab = $state('general');
	let appVersion = $state<string | null>(null);
	let aboutOpen = $state(false);
	let updateOpen = $state(false);
	let updateAvailable = $state<UpdateInfo | null>(null);
	let updateChecking = $state(false);
	let updateProgress = $state<number | null>(null);
	let updateInstalling = $state(false);
	let activeComponent = $derived(loadTab(activeTab));

	function getHashTab(): string {
		const hash = window.location.hash.slice(1);
		return tabs.some((t) => t.id === hash) ? hash : 'general';
	}

	function setTab(id: string) {
		window.location.hash = id;
		activeTab = id;
	}

	async function handleCheckUpdate() {
		updateChecking = true;
		updateProgress = null;
		try {
			const info = await checkForUpdate();
			updateAvailable = info;
		} catch (e) {
			console.error('Update check failed:', e);
		} finally {
			updateChecking = false;
		}
	}

	async function handleInstallUpdate() {
		updateInstalling = true;
		updateProgress = 0;
		try {
			await installUpdate((percent) => {
				updateProgress = percent;
			});
		} catch (e) {
			console.error('Update install failed:', e);
			updateInstalling = false;
			updateProgress = null;
		}
	}

	onMount(async () => {
		activeTab = getHashTab();
		appVersion = await getAppVersion();

		// Listen for background update-available event
		onUpdateAvailable((info) => {
			updateAvailable = info;
		});
	});

	$effect(() => {
		function onHashChange() {
			activeTab = getHashTab();
		}
		window.addEventListener('hashchange', onHashChange);
		return () => window.removeEventListener('hashchange', onHashChange);
	});
</script>

<div class="flex flex-col md:flex-row gap-4 max-w-4xl mx-auto">
	<!-- Desktop sidebar -->
	<nav class="hidden md:flex flex-col w-48 shrink-0 space-y-1">
		{#each tabs as tab (tab.id)}
			<button
				class="flex items-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-colors text-left
					{activeTab === tab.id ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
				onclick={() => setTab(tab.id)}
			>
				<tab.icon class="h-4 w-4" />
				{tab.label}
			</button>
		{/each}

		<Separator class="my-2" />

		<button
			class="relative flex items-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-colors text-left text-muted-foreground hover:bg-muted hover:text-foreground"
			onclick={() => { updateOpen = true; handleCheckUpdate(); }}
		>
			<Download class="h-4 w-4" />
			{m.settings_tab_updates()}
			{#if updateAvailable}
				<span class="absolute top-1.5 left-7 h-2 w-2 rounded-full bg-primary animate-pulse"></span>
			{/if}
		</button>

		<button
			class="flex items-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-colors text-left text-muted-foreground hover:bg-muted hover:text-foreground"
			onclick={() => { aboutOpen = true; }}
		>
			<Info class="h-4 w-4" />
			{m.settings_tab_about()}
		</button>
	</nav>

	<!-- Mobile horizontal tabs -->
	<div class="md:hidden overflow-x-auto flex gap-1 border-b pb-2">
		{#each tabs as tab (tab.id)}
			<button
				class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium whitespace-nowrap transition-colors
					{activeTab === tab.id ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-muted'}"
				onclick={() => setTab(tab.id)}
			>
				<tab.icon class="h-3.5 w-3.5" />
				{tab.label}
			</button>
		{/each}
		<button
			class="relative flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium whitespace-nowrap transition-colors text-muted-foreground hover:bg-muted"
			onclick={() => { updateOpen = true; handleCheckUpdate(); }}
		>
			<Download class="h-3.5 w-3.5" />
			{m.settings_tab_updates()}
			{#if updateAvailable}
				<span class="absolute top-0.5 right-0.5 h-2 w-2 rounded-full bg-primary animate-pulse"></span>
			{/if}
		</button>
		<button
			class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium whitespace-nowrap transition-colors text-muted-foreground hover:bg-muted"
			onclick={() => { aboutOpen = true; }}
		>
			<Info class="h-3.5 w-3.5" />
			{m.settings_tab_about()}
		</button>
	</div>

	<!-- Content area -->
	<div class="flex-1 min-w-0 space-y-4">
		<h1 class="text-2xl font-bold">{tabs.find((t) => t.id === activeTab)?.label ?? m.settings_fallback_title()}</h1>

		{#await activeComponent}
			<div class="flex items-center justify-center py-12">
				<div class="h-6 w-6 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent"></div>
			</div>
		{:then TabComponent}
			<TabComponent />
		{:catch error}
			<p class="text-destructive text-sm">{m.settings_loading_error({ message: error.message })}</p>
		{/await}
	</div>
</div>

<Dialog.Root bind:open={aboutOpen}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<div class="flex items-center gap-3">
				<img src="/app-icon-32.png" alt="Zenii" class="h-10 w-10" />
				<div>
					<Dialog.Title class="text-xl">{m.settings_about_title()}</Dialog.Title>
					{#if appVersion}
						<p class="text-sm text-muted-foreground">{m.settings_about_version({ version: appVersion })}</p>
					{/if}
				</div>
			</div>
		</Dialog.Header>
		<Dialog.Description class="space-y-3">
			<p>{m.settings_about_tagline()}</p>
			<div class="text-xs text-muted-foreground space-y-1">
				<p>{m.settings_about_company()}</p>
				<p>{m.settings_about_license()}</p>
			</div>
			<div class="flex gap-3 text-sm">
				<a href="https://zenii.sprklai.com" target="_blank" rel="noopener" class="text-primary hover:underline">{m.settings_about_website()}</a>
				<a href="https://github.com/sprklai/zenii" target="_blank" rel="noopener" class="text-primary hover:underline">{m.settings_about_github()}</a>
			</div>
			<Separator class="my-2" />
			<div class="text-xs text-muted-foreground leading-relaxed">
				<p class="font-medium text-foreground">{m.settings_about_disclaimer_title()}</p>
				<p>{m.settings_about_disclaimer_body()}</p>
			</div>
		</Dialog.Description>
	</Dialog.Content>
</Dialog.Root>

<Dialog.Root bind:open={updateOpen}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<div class="flex items-center gap-3">
				<Download class="h-8 w-8 text-primary" />
				<div>
					<Dialog.Title class="text-xl">{m.settings_updates_title()}</Dialog.Title>
					{#if appVersion}
						<p class="text-sm text-muted-foreground">{m.settings_updates_current_version({ version: appVersion })}</p>
					{/if}
				</div>
			</div>
		</Dialog.Header>
		<Dialog.Description class="space-y-4">
			{#if updateChecking}
				<div class="flex items-center gap-3 py-4">
					<div class="h-5 w-5 animate-spin rounded-full border-2 border-primary border-t-transparent"></div>
					<p class="text-sm">{m.settings_updates_checking()}</p>
				</div>
			{:else if updateInstalling}
				<div class="space-y-3 py-2">
					<p class="text-sm font-medium">{m.settings_updates_installing({ version: updateAvailable?.version ?? '' })}</p>
					<div class="w-full bg-muted rounded-full h-2">
						<div
							class="bg-primary h-2 rounded-full transition-all duration-300"
							style="width: {updateProgress ?? 0}%"
						></div>
					</div>
					<p class="text-xs text-muted-foreground text-center">{updateProgress ?? 0}%</p>
				</div>
			{:else if updateAvailable}
				<div class="space-y-3">
					<div class="flex items-center gap-2">
						<span class="h-2 w-2 rounded-full bg-primary"></span>
						<p class="text-sm font-medium">{m.settings_updates_available({ version: updateAvailable.version })}</p>
					</div>
					{#if updateAvailable.body}
						<div class="text-xs text-muted-foreground bg-muted rounded-md p-3 max-h-40 overflow-y-auto">
							{updateAvailable.body}
						</div>
					{/if}
					<button
						class="w-full px-4 py-2 rounded-md text-sm font-medium bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
						onclick={handleInstallUpdate}
					>
						{m.settings_updates_install_button()}
					</button>
				</div>
			{:else}
				<div class="flex items-center gap-3 py-4">
					<span class="text-green-500">&#10003;</span>
					<p class="text-sm">{m.settings_updates_up_to_date()}</p>
				</div>
			{/if}

			<Separator />
			<div class="space-y-1.5">
				<p class="text-xs text-muted-foreground">{m.settings_updates_check_manually()}</p>
				<div class="flex gap-3">
					<button
						class="inline-flex items-center gap-1 text-xs text-primary hover:underline"
						onclick={() => openInBrowser('https://github.com/sprklai/zenii/releases/')}
					>
						{m.settings_updates_github_releases()} <ExternalLink class="h-3 w-3" />
					</button>
					<button
						class="inline-flex items-center gap-1 text-xs text-primary hover:underline"
						onclick={() => openInBrowser('https://zenii.sprklai.com/#download')}
					>
						{m.settings_updates_download_page()} <ExternalLink class="h-3 w-3" />
					</button>
				</div>
			</div>
		</Dialog.Description>
	</Dialog.Content>
</Dialog.Root>
