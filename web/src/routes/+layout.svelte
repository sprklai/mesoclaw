<script lang="ts">
	import '../app.css';
	import * as Sidebar from '$lib/components/ui/sidebar';
	import { Button } from '$lib/components/ui/button';
	import { Separator } from '$lib/components/ui/separator';
	import AuthGate from '$lib/components/AuthGate.svelte';
	import SessionList from '$lib/components/SessionList.svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import { Toaster } from 'svelte-sonner';
	import Home from '@lucide/svelte/icons/home';
	import MessageSquare from '@lucide/svelte/icons/message-square';
	import Database from '@lucide/svelte/icons/database';
	import Settings from '@lucide/svelte/icons/settings';
	import Calendar from '@lucide/svelte/icons/calendar';
	import { inboxStore } from '$lib/stores/inbox.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { sessionsStore } from '$lib/stores/sessions.svelte';
	import { notificationStore } from '$lib/stores/notifications.svelte';
	import { onMount, onDestroy } from 'svelte';

	let { children } = $props();

	onMount(() => {
		sessionsStore.load();
		notificationStore.connect('http://127.0.0.1:18981');
	});

	onDestroy(() => {
		notificationStore.disconnect();
	});

	const navItems = [
		{ href: '/', icon: Home, label: 'Home' },
		{ href: '/channels', icon: MessageSquare, label: 'Channels' },
		{ href: '/memory', icon: Database, label: 'Memory' },
		{ href: '/schedule', icon: Calendar, label: 'Schedule' }
	];
</script>

<Toaster richColors />
<AuthGate>
	<Sidebar.Provider>
		<Sidebar.Root>
			<Sidebar.Header class="sticky top-0 z-10 bg-sidebar-accent/50 border-b border-sidebar-border">
				<div class="flex items-center gap-2 px-2 py-1">
					<img src="/logo.png" alt="MesoClaw" class="h-6 w-6" />
					<span class="font-semibold text-lg">MesoClaw</span>
				</div>
			</Sidebar.Header>

			<Sidebar.Content>
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
			</Sidebar.Content>

			<Sidebar.Footer class="sticky bottom-0 z-10 bg-sidebar-accent/50 border-t border-sidebar-border">
				<Sidebar.Menu>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton
							isActive={page.url.pathname.startsWith('/settings')}
							onclick={() => goto('/settings')}
						>
							<Settings class="h-4 w-4" />
							<span>Settings</span>
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
				<ThemeToggle />
			</Sidebar.Footer>
		</Sidebar.Root>

		<main class="flex-1 overflow-hidden">
			<div class="flex h-full items-start">
				<Sidebar.Trigger class="m-2 shrink-0" />
				<div class="flex-1 h-full overflow-auto p-2 sm:p-4 md:p-6">
					{@render children()}
				</div>
			</div>
		</main>
	</Sidebar.Provider>
</AuthGate>
