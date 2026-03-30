<script lang="ts">
	import '../app.css';
	import * as m from '$lib/paraglide/messages';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Separator } from '$lib/components/ui/separator';
	import { Button } from '$lib/components/ui/button';
	import AuthGate from '$lib/components/AuthGate.svelte';
	import SessionList from '$lib/components/SessionList.svelte';
	import { Toaster } from 'svelte-sonner';
	import Home from '@lucide/svelte/icons/home';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import Database from '@lucide/svelte/icons/database';
	import Settings from '@lucide/svelte/icons/settings';
	import Calendar from '@lucide/svelte/icons/calendar';
	import Workflow from '@lucide/svelte/icons/workflow';
	import BookOpen from '@lucide/svelte/icons/book-open';
	import FileText from '@lucide/svelte/icons/file-text';
	import Star from '@lucide/svelte/icons/star';
	import WifiOff from '@lucide/svelte/icons/wifi-off';
	import { inboxStore } from '$lib/stores/inbox.svelte';
	import '$lib/stores/theme.svelte';
	import { localeStore } from '$lib/stores/locale.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { notificationStore } from '$lib/stores/notifications.svelte';
	import { capabilitiesStore } from '$lib/stores/capabilities.svelte';
	import { getBaseUrl, getToken } from '$lib/api/client';
	import { getAppVersion, openInBrowser } from '$lib/tauri';
	import { onDestroy } from 'svelte';

	let { children } = $props();
	let appVersion = $state<string | null>(null);

	function handleVisibilityChange() {
		if (document.visibilityState === 'visible') {
			sessionsStore.load();
		}
	}

	/** Called by AuthGate once the gateway is authenticated and ready. */
	function handleGatewayReady() {
		sessionsStore.load();
		capabilitiesStore.load();
		document.addEventListener('visibilitychange', handleVisibilityChange);

		const baseUrl = getBaseUrl();
		const wsBase = baseUrl.replace(/^http/, 'ws');
		const token = getToken();
		const wsUrl = token
			? `${wsBase}/ws/notifications?token=${encodeURIComponent(token)}`
			: `${wsBase}/ws/notifications`;
		notificationStore.connect(wsUrl);

		getAppVersion().then((v) => {
			appVersion = v;
		});
	}

	onDestroy(() => {
		document.removeEventListener('visibilitychange', handleVisibilityChange);
		notificationStore.disconnect();
	});

	const navItems = $derived.by(() => {
		// Read localeStore.locale to create a reactive dependency on locale changes
		void localeStore.locale;
		return [
			{ href: '/', icon: Home, label: m.nav_home() },
			{ href: '/channels', icon: MessageSquare, label: m.nav_channels() },
			{ href: '/memory', icon: Database, label: m.nav_memory() },
			{ href: '/schedule', icon: Calendar, label: m.nav_schedule() },
			{ href: '/workflows', icon: Workflow, label: m.nav_workflows() }
		];
	});

	function handleApiDocs() {
		const baseUrl = getBaseUrl();
		openInBrowser(`${baseUrl}/api-docs`);
	}
</script>

<Toaster richColors />
<AuthGate onReady={handleGatewayReady}>
	<Sidebar.Provider>
		<Sidebar.Root>
			<Sidebar.Header class="sticky top-0 z-10 bg-sidebar-accent/50 border-b border-sidebar-border">
				<div class="flex items-center gap-2 px-2 py-1">
					<img src="/app-icon-32.png" alt={m.app_name()} class="h-6 w-6" />
					<span class="font-semibold text-lg">{m.app_name()}</span>
					{#if appVersion}
						<span class="text-xs text-muted-foreground">v{appVersion}</span>
					{/if}
				</div>
			</Sidebar.Header>

			<Sidebar.Content>
				{#key localeStore.locale}
				<Sidebar.Group>
					<Sidebar.GroupContent>
						<Sidebar.Menu>
							{#each navItems as item (item.href)}
								<Sidebar.MenuItem>
									<Sidebar.MenuButton
										isActive={page.url.pathname === item.href || (item.href !== '/' && page.url.pathname.startsWith(item.href))}
										onclick={() => goto(item.href)}
									>
										<item.icon class="h-4 w-4" />
										<span>{item.label}</span>
										{#if item.href === '/channels' && inboxStore.totalUnread > 0}
											<span class="ml-auto inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-primary px-1 text-xs font-bold text-primary-foreground">
												{inboxStore.totalUnread}
											</span>
										{/if}
									</Sidebar.MenuButton>
								</Sidebar.MenuItem>
							{/each}
						</Sidebar.Menu>
					</Sidebar.GroupContent>
				</Sidebar.Group>

				<Separator />

				<SessionList />
				{/key}
			</Sidebar.Content>

			<Sidebar.Footer class="sticky bottom-0 z-10 bg-sidebar-accent/50 border-t border-sidebar-border">
				{#key localeStore.locale}
				<Sidebar.Menu>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton onclick={() => openInBrowser('https://github.com/sprklai/zenii')}>
							<Star class="h-4 w-4" />
							<span>{m.nav_star_github()}</span>
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
					<Sidebar.MenuItem>
						<div class="flex items-center gap-0 rounded-md border border-sidebar-border overflow-hidden">
							<Sidebar.MenuButton onclick={() => openInBrowser('https://docs.zenii.sprklai.com/installation-and-usage')} class="flex-1 !rounded-none border-r border-sidebar-border">
								<FileText class="h-4 w-4" />
								<span>{m.nav_docs()}</span>
							</Sidebar.MenuButton>
							<Sidebar.MenuButton onclick={handleApiDocs} class="flex-1 !rounded-none">
								<BookOpen class="h-4 w-4" />
								<span>{m.nav_api_docs()}</span>
							</Sidebar.MenuButton>
						</div>
					</Sidebar.MenuItem>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton
							isActive={page.url.pathname.startsWith('/settings')}
							onclick={() => goto('/settings')}
						>
							<Settings class="h-4 w-4" />
							<span>{m.nav_settings()}</span>
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
				{/key}
			</Sidebar.Footer>
		</Sidebar.Root>

		<main class="flex-1 overflow-hidden">
			{#if notificationStore.disconnectedPermanently}
				{#key localeStore.locale}
				<div class="flex items-center gap-2 border-b border-destructive/30 bg-destructive/10 px-4 py-2 text-sm text-destructive">
					<WifiOff class="h-4 w-4 shrink-0" />
					<span>{m.nav_notifications_disconnected()}</span>
					<Button
						variant="outline"
						size="sm"
						class="ml-auto h-7 border-destructive/30 text-destructive hover:bg-destructive/10"
						onclick={() => notificationStore.retryConnection()}
					>
						{m.nav_reconnect_button()}
					</Button>
				</div>
				{/key}
			{/if}
			<div class="flex h-full items-start">
				<Sidebar.Trigger class="m-2 shrink-0" />
				<div class="flex-1 h-full overflow-auto p-2 sm:p-4 md:p-6">
					{#key localeStore.locale}
						{@render children()}
					{/key}
				</div>
			</div>
		</main>
	</Sidebar.Provider>
</AuthGate>
