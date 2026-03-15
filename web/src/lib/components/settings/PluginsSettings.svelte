<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { pluginsStore, type PluginDetail } from '$lib/stores/plugins.svelte';
	import { onMount } from 'svelte';

	let installSource = $state('');
	let installLocal = $state(false);
	let installAll = $state(false);
	let expandedName = $state<string | null>(null);
	let detail = $state<PluginDetail | null>(null);
	let confirmOpen = $state(false);
	let removeTarget = $state<string | null>(null);
	let toggling = $state<Record<string, boolean>>({});

	// Browse state
	let showBrowse = $state(false);
	let selected = $state<Set<string>>(new Set());

	onMount(async () => {
		await pluginsStore.load();
	});

	async function handleInstall() {
		const source = installSource.trim();
		if (!source) return;
		const ok = await pluginsStore.install(source, installLocal, installAll);
		if (ok) {
			installSource = '';
			installLocal = false;
			installAll = false;
		}
	}

	async function togglePlugin(name: string) {
		toggling[name] = true;
		try {
			await pluginsStore.toggle(name);
		} finally {
			toggling[name] = false;
		}
	}

	function confirmRemove(name: string) {
		removeTarget = name;
		confirmOpen = true;
	}

	async function handleRemove() {
		if (!removeTarget) return;
		await pluginsStore.remove(removeTarget);
		if (expandedName === removeTarget) {
			expandedName = null;
			detail = null;
		}
	}

	async function toggleExpand(name: string) {
		if (expandedName === name) {
			expandedName = null;
			detail = null;
		} else {
			expandedName = name;
			detail = await pluginsStore.getDetail(name);
		}
	}

	async function handleBrowse() {
		showBrowse = true;
		selected = new Set();
		await pluginsStore.loadAvailable();
	}

	function toggleSelect(name: string) {
		const next = new Set(selected);
		if (next.has(name)) {
			next.delete(name);
		} else {
			next.add(name);
		}
		selected = next;
	}

	function toggleSelectAll() {
		const installable = pluginsStore.available.filter((p) => !p.installed);
		if (selected.size === installable.length) {
			selected = new Set();
		} else {
			selected = new Set(installable.map((p) => p.name));
		}
	}

	async function handleInstallSelected() {
		const names = [...selected];
		if (names.length === 0) return;
		await pluginsStore.installSelected(names);
		selected = new Set();
	}

	const installableCount = $derived(
		pluginsStore.available.filter((p) => !p.installed).length
	);

	const allSelected = $derived(
		installableCount > 0 && selected.size === installableCount
	);
</script>

<!-- Install form -->
<Card.Root>
	<Card.Header class="py-3">
		<Card.Title class="text-base">Install Plugin</Card.Title>
	</Card.Header>
	<Card.Content class="space-y-3">
		<div class="flex gap-2">
			<Input
				placeholder="Git URL or local path"
				bind:value={installSource}
				onkeydown={(e: KeyboardEvent) => {
					if (e.key === 'Enter') handleInstall();
				}}
			/>
			<Button
				size="sm"
				disabled={!installSource.trim() || pluginsStore.installing}
				onclick={handleInstall}
			>
				{pluginsStore.installing ? 'Installing...' : 'Install'}
			</Button>
		</div>
		<div class="flex items-center gap-4">
			<div class="flex items-center gap-2">
				<input
					id="install-local"
					type="checkbox"
					class="h-4 w-4 rounded border-input"
					bind:checked={installLocal}
					onchange={() => { if (!installLocal) installAll = false; }}
				/>
				<label class="text-sm text-muted-foreground" for="install-local">
					Local directory
				</label>
			</div>
			{#if installLocal}
				<div class="flex items-center gap-2">
					<input
						id="install-all"
						type="checkbox"
						class="h-4 w-4 rounded border-input"
						bind:checked={installAll}
					/>
					<label class="text-sm text-muted-foreground" for="install-all">
						Install all plugins in directory
					</label>
				</div>
			{/if}
		</div>
		{#if pluginsStore.error}
			<p class="text-sm text-destructive">{pluginsStore.error}</p>
		{/if}
	</Card.Content>
</Card.Root>

<!-- Browse Official Plugins -->
{#if !showBrowse}
	<div class="flex justify-center py-2">
		<Button variant="outline" onclick={handleBrowse}>
			Browse Official Plugins
		</Button>
	</div>
{:else}
	<Card.Root>
		<Card.Header class="py-3">
			<div class="flex items-center justify-between">
				<Card.Title class="text-base">Official Plugins</Card.Title>
				<div class="flex items-center gap-2">
					{#if pluginsStore.available.length > 0 && !pluginsStore.browsing}
						<label class="flex items-center gap-1.5 text-sm text-muted-foreground cursor-pointer">
							<input
								type="checkbox"
								class="h-4 w-4 rounded border-input"
								checked={allSelected}
								onchange={toggleSelectAll}
							/>
							Select all
						</label>
						<Button
							size="sm"
							disabled={selected.size === 0 || pluginsStore.installing}
							onclick={handleInstallSelected}
						>
							{pluginsStore.installing
								? 'Installing...'
								: `Install Selected (${selected.size})`}
						</Button>
					{/if}
					<Button
						size="sm"
						variant="ghost"
						onclick={() => { showBrowse = false; }}
					>
						Close
					</Button>
				</div>
			</div>
		</Card.Header>
		<Card.Content>
			{#if pluginsStore.browsing}
				<div class="space-y-2">
					<Skeleton class="h-12 w-full" />
					<Skeleton class="h-12 w-full" />
					<Skeleton class="h-12 w-full" />
				</div>
			{:else if pluginsStore.available.length === 0}
				<p class="text-sm text-muted-foreground">No plugins found in official repository.</p>
			{:else}
				<div class="space-y-2">
					{#each pluginsStore.available as plugin (plugin.name)}
						<div
							class="flex items-center gap-3 rounded-md border p-3"
							class:opacity-60={plugin.installed}
						>
							<input
								type="checkbox"
								class="h-4 w-4 rounded border-input"
								checked={plugin.installed || selected.has(plugin.name)}
								disabled={plugin.installed}
								onchange={() => toggleSelect(plugin.name)}
							/>
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-2">
									<span class="font-medium text-sm">{plugin.name}</span>
									<Badge variant="outline" class="text-xs">v{plugin.version}</Badge>
									{#if plugin.installed}
										<Badge variant="secondary" class="text-xs">Installed</Badge>
									{/if}
									<span class="text-xs text-muted-foreground">
										{plugin.tools_count} tool{plugin.tools_count !== 1 ? 's' : ''}{#if plugin.skills_count > 0}, {plugin.skills_count} skill{plugin.skills_count !== 1 ? 's' : ''}{/if}
									</span>
								</div>
								<p class="text-xs text-muted-foreground mt-0.5 truncate">{plugin.description}</p>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</Card.Content>
	</Card.Root>
{/if}

<!-- Plugin list -->
{#if pluginsStore.loading}
	<div class="space-y-2">
		<Skeleton class="h-16 w-full" />
		<Skeleton class="h-16 w-full" />
	</div>
{:else if pluginsStore.plugins.length === 0}
	<p class="text-sm text-muted-foreground py-4">No plugins installed.</p>
{:else}
	<div class="space-y-2">
		{#each pluginsStore.plugins as plugin (plugin.name)}
			<Card.Root>
				<button class="w-full text-left" onclick={() => toggleExpand(plugin.name)}>
					<Card.Header class="py-3">
						<div class="flex items-center justify-between">
							<div class="flex items-center gap-2">
								<Card.Title class="text-base">{plugin.name}</Card.Title>
								<Badge variant="outline">v{plugin.version}</Badge>
								<span class="text-xs text-muted-foreground">
									{plugin.tools_count} tool{plugin.tools_count !== 1 ? 's' : ''}, {plugin.skills_count} skill{plugin.skills_count !== 1 ? 's' : ''}
								</span>
							</div>
							<div class="flex items-center gap-3">
								<Switch
									checked={plugin.enabled}
									disabled={toggling[plugin.name]}
									onCheckedChange={() => togglePlugin(plugin.name)}
									onclick={(e: MouseEvent) => e.stopPropagation()}
								/>
								<span class="text-xs text-muted-foreground">
									{expandedName === plugin.name ? '\u25B2' : '\u25BC'}
								</span>
							</div>
						</div>
						<p class="text-sm text-muted-foreground mt-1">{plugin.description}</p>
					</Card.Header>
				</button>

				{#if expandedName === plugin.name && detail}
					<Card.Content class="pt-0 space-y-3">
						<div class="grid grid-cols-2 gap-2 text-sm">
							{#if detail.manifest.plugin.author}
								<div>
									<span class="font-medium">Author:</span>
									{detail.manifest.plugin.author}
								</div>
							{/if}
							{#if detail.manifest.plugin.license}
								<div>
									<span class="font-medium">License:</span>
									{detail.manifest.plugin.license}
								</div>
							{/if}
							<div>
								<span class="font-medium">Installed:</span>
								{detail.installed_at.slice(0, 19)}
							</div>
						</div>

						{#if detail.manifest.tools.length > 0}
							<div>
								<h4 class="text-sm font-semibold mb-1">Tools</h4>
								<ul class="text-sm text-muted-foreground space-y-0.5">
									{#each detail.manifest.tools as tool}
										<li>
											<span class="font-medium text-foreground">{tool.name}</span> —
											{tool.description}
										</li>
									{/each}
								</ul>
							</div>
						{/if}

						{#if detail.manifest.skills.length > 0}
							<div>
								<h4 class="text-sm font-semibold mb-1">Skills</h4>
								<ul class="text-sm text-muted-foreground space-y-0.5">
									{#each detail.manifest.skills as skill}
										<li>{skill.name}</li>
									{/each}
								</ul>
							</div>
						{/if}

						<div class="flex gap-2 border-t pt-3">
							<Button
								size="sm"
								variant="destructive"
								onclick={() => confirmRemove(plugin.name)}
							>
								Remove
							</Button>
						</div>
					</Card.Content>
				{/if}
			</Card.Root>
		{/each}
	</div>
{/if}

<ConfirmDialog
	bind:open={confirmOpen}
	title="Remove plugin?"
	description="This will uninstall the plugin and remove all its files."
	confirmLabel="Remove"
	onConfirm={handleRemove}
/>
