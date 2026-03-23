<script lang="ts">
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import Cpu from '@lucide/svelte/icons/cpu';
	import User from '@lucide/svelte/icons/user';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import KeyRound from '@lucide/svelte/icons/key-round';
	import Brain from '@lucide/svelte/icons/brain';
	import Puzzle from '@lucide/svelte/icons/puzzle';
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
		{ id: 'general', label: 'General', icon: SettingsIcon },
		{ id: 'providers', label: 'AI Providers', icon: Cpu },
		{ id: 'persona', label: 'Persona', icon: User },
		{ id: 'channels', label: 'Channels', icon: MessageSquare },
		{ id: 'permissions', label: 'Permissions', icon: Shield },
		{ id: 'notifications', label: 'Notifications', icon: Bell },
		{ id: 'services', label: 'Services', icon: KeyRound },
		{ id: 'embeddings', label: 'Embeddings', icon: Brain },
		{ id: 'configurations', label: 'Configurations', icon: FileText },
		{ id: 'plugins', label: 'Plugins', icon: Puzzle },
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
			Updates
			{#if updateAvailable}
				<span class="absolute top-1.5 left-7 h-2 w-2 rounded-full bg-primary animate-pulse"></span>
			{/if}
		</button>

		<button
			class="flex items-center gap-2 px-3 py-2 rounded-md text-sm font-medium transition-colors text-left text-muted-foreground hover:bg-muted hover:text-foreground"
			onclick={() => { aboutOpen = true; }}
		>
			<Info class="h-4 w-4" />
			About
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
			Updates
			{#if updateAvailable}
				<span class="absolute top-0.5 right-0.5 h-2 w-2 rounded-full bg-primary animate-pulse"></span>
			{/if}
		</button>
		<button
			class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium whitespace-nowrap transition-colors text-muted-foreground hover:bg-muted"
			onclick={() => { aboutOpen = true; }}
		>
			<Info class="h-3.5 w-3.5" />
			About
		</button>
	</div>

	<!-- Content area -->
	<div class="flex-1 min-w-0 space-y-4">
		<h1 class="text-2xl font-bold">{tabs.find((t) => t.id === activeTab)?.label ?? 'Settings'}</h1>

		{#await activeComponent}
			<div class="flex items-center justify-center py-12">
				<div class="h-6 w-6 animate-spin rounded-full border-2 border-muted-foreground border-t-transparent"></div>
			</div>
		{:then TabComponent}
			<TabComponent />
		{:catch error}
			<p class="text-destructive text-sm">Failed to load settings tab: {error.message}</p>
		{/await}
	</div>
</div>

<Dialog.Root bind:open={aboutOpen}>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<div class="flex items-center gap-3">
				<img src="/app-icon-32.png" alt="Zenii" class="h-10 w-10" />
				<div>
					<Dialog.Title class="text-xl">Zenii</Dialog.Title>
					{#if appVersion}
						<p class="text-sm text-muted-foreground">v{appVersion}</p>
					{/if}
				</div>
			</div>
		</Dialog.Header>
		<Dialog.Description class="space-y-3">
			<p>Your private AI backend</p>
			<div class="text-xs text-muted-foreground space-y-1">
				<p>SprklAI by NSRTech</p>
				<p>MIT License</p>
			</div>
			<div class="flex gap-3 text-sm">
				<a href="https://zenii.sprklai.com" target="_blank" rel="noopener" class="text-primary hover:underline">Website</a>
				<a href="https://github.com/sprklai/zenii" target="_blank" rel="noopener" class="text-primary hover:underline">GitHub</a>
			</div>
			<Separator class="my-2" />
			<div class="text-xs text-muted-foreground leading-relaxed">
				<p class="font-medium text-foreground">Disclaimer</p>
				<p>Zenii uses large language models (LLMs) to generate responses and can execute system-level actions (shell commands, file operations) on your behalf. LLM outputs may be inaccurate, incomplete, or inappropriate. System actions run with your user permissions. Always review AI-suggested actions before confirming. Use at your own risk.</p>
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
					<Dialog.Title class="text-xl">Software Update</Dialog.Title>
					{#if appVersion}
						<p class="text-sm text-muted-foreground">Current: v{appVersion}</p>
					{/if}
				</div>
			</div>
		</Dialog.Header>
		<Dialog.Description class="space-y-4">
			{#if updateChecking}
				<div class="flex items-center gap-3 py-4">
					<div class="h-5 w-5 animate-spin rounded-full border-2 border-primary border-t-transparent"></div>
					<p class="text-sm">Checking for updates...</p>
				</div>
			{:else if updateInstalling}
				<div class="space-y-3 py-2">
					<p class="text-sm font-medium">Installing update v{updateAvailable?.version}...</p>
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
						<p class="text-sm font-medium">Version {updateAvailable.version} is available</p>
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
						Install & Restart
					</button>
				</div>
			{:else}
				<div class="flex items-center gap-3 py-4">
					<span class="text-green-500">&#10003;</span>
					<p class="text-sm">You're up to date!</p>
				</div>
			{/if}

			<Separator />
			<div class="space-y-1.5">
				<p class="text-xs text-muted-foreground">Or check manually:</p>
				<div class="flex gap-3">
					<button
						class="inline-flex items-center gap-1 text-xs text-primary hover:underline"
						onclick={() => openInBrowser('https://github.com/sprklai/zenii/releases/')}
					>
						GitHub Releases <ExternalLink class="h-3 w-3" />
					</button>
					<button
						class="inline-flex items-center gap-1 text-xs text-primary hover:underline"
						onclick={() => openInBrowser('https://zenii.sprklai.com/#download')}
					>
						Download Page <ExternalLink class="h-3 w-3" />
					</button>
				</div>
			</div>
		</Dialog.Description>
	</Dialog.Content>
</Dialog.Root>
