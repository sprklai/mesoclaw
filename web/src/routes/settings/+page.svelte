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
	import GeneralSettings from '$lib/components/settings/GeneralSettings.svelte';
	import PermissionsSettings from '$lib/components/settings/PermissionsSettings.svelte';
	import ConfigurationsSettings from '$lib/components/settings/ConfigurationsSettings.svelte';
	import ProvidersSettings from '$lib/components/settings/ProvidersSettings.svelte';
	import PersonaSettings from '$lib/components/settings/PersonaSettings.svelte';
	import ChannelsSettings from '$lib/components/settings/ChannelsSettings.svelte';
	import ServicesSettings from '$lib/components/settings/ServicesSettings.svelte';
	import EmbeddingsSettings from '$lib/components/settings/EmbeddingsSettings.svelte';
	import NotificationsSettings from '$lib/components/settings/NotificationsSettings.svelte';
	import PluginsSettings from '$lib/components/settings/PluginsSettings.svelte';
	import { Separator } from '$lib/components/ui/separator';
	import * as Dialog from '$lib/components/ui/dialog';
	import { getAppVersion } from '$lib/tauri';
	import { onMount } from 'svelte';

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

	function getHashTab(): string {
		const hash = window.location.hash.slice(1);
		return tabs.some((t) => t.id === hash) ? hash : 'general';
	}

	function setTab(id: string) {
		window.location.hash = id;
		activeTab = id;
	}

	onMount(async () => {
		activeTab = getHashTab();
		appVersion = await getAppVersion();
	});

	$effect(() => {
		function onHashChange() {
			activeTab = getHashTab();
		}
		window.addEventListener('hashchange', onHashChange);
		return () => window.removeEventListener('hashchange', onHashChange);
	});
</script>

<div class="flex flex-col md:flex-row gap-6 max-w-4xl mx-auto">
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
			class="flex items-center gap-1.5 px-3 py-1.5 rounded-md text-sm font-medium whitespace-nowrap transition-colors text-muted-foreground hover:bg-muted"
			onclick={() => { aboutOpen = true; }}
		>
			<Info class="h-3.5 w-3.5" />
			About
		</button>
	</div>

	<!-- Content area -->
	<div class="flex-1 min-w-0 space-y-6">
		<h1 class="text-2xl font-bold">{tabs.find((t) => t.id === activeTab)?.label ?? 'Settings'}</h1>

		{#if activeTab === 'general'}
			<GeneralSettings />
		{:else if activeTab === 'providers'}
			<ProvidersSettings />
		{:else if activeTab === 'persona'}
			<PersonaSettings />
		{:else if activeTab === 'channels'}
			<ChannelsSettings />
		{:else if activeTab === 'permissions'}
			<PermissionsSettings />
		{:else if activeTab === 'notifications'}
			<NotificationsSettings />
		{:else if activeTab === 'services'}
			<ServicesSettings />
		{:else if activeTab === 'embeddings'}
			<EmbeddingsSettings />
		{:else if activeTab === 'configurations'}
			<ConfigurationsSettings />
		{:else if activeTab === 'plugins'}
			<PluginsSettings />
		{/if}
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
			<p>AI-powered personal assistant</p>
			<div class="text-xs text-muted-foreground space-y-1">
				<p>SprklAI by NSRTech</p>
				<p>MIT License</p>
			</div>
			<div class="flex gap-3 text-sm">
				<a href="https://zenii.sprklai.com" target="_blank" rel="noopener" class="text-primary hover:underline">Website</a>
				<a href="https://github.com/sprklai/zenii" target="_blank" rel="noopener" class="text-primary hover:underline">GitHub</a>
			</div>
		</Dialog.Description>
	</Dialog.Content>
</Dialog.Root>
