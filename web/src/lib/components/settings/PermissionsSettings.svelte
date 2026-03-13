<script lang="ts">
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { Badge } from '$lib/components/ui/badge';
	import * as Card from '$lib/components/ui/card';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { permissionsStore, type ToolPermissionInfo } from '$lib/stores/permissions.svelte';
	import { onMount } from 'svelte';

	let loading = $state(true);

	onMount(async () => {
		await permissionsStore.loadSurfaces();
		await permissionsStore.loadAllPermissions();
		loading = false;
	});

	function riskColor(risk: string): 'default' | 'secondary' | 'destructive' | 'outline' {
		switch (risk) {
			case 'high':
				return 'destructive';
			case 'medium':
				return 'outline';
			default:
				return 'secondary';
		}
	}

	function surfaceLabel(s: string): string {
		return s.charAt(0).toUpperCase() + s.slice(1);
	}

	function isLocalSurface(s: string): boolean {
		return s === 'desktop' || s === 'cli' || s === 'tui';
	}

	/** Get the canonical tool list (from first surface that has data). */
	function getToolList(): ToolPermissionInfo[] {
		for (const tools of permissionsStore.allPermissions.values()) {
			if (tools.length > 0) return tools;
		}
		return [];
	}

	/** Group tools by risk level in display order: high, medium, low. */
	function groupedTools(): { risk: string; tools: ToolPermissionInfo[] }[] {
		const all = getToolList();
		const groups: Record<string, ToolPermissionInfo[]> = { high: [], medium: [], low: [] };
		for (const tool of all) {
			const level = tool.risk_level ?? 'low';
			if (groups[level]) {
				groups[level].push(tool);
			} else {
				groups['low'].push(tool);
			}
		}
		return ['high', 'medium', 'low']
			.filter((r) => groups[r].length > 0)
			.map((r) => ({ risk: r, tools: groups[r] }));
	}

	/** Look up a tool's state for a given surface. */
	function toolState(surface: string, toolName: string): 'allowed' | 'denied' {
		const tools = permissionsStore.allPermissions.get(surface);
		if (!tools) return 'denied';
		const tool = tools.find((t) => t.name === toolName);
		return tool?.state === 'allowed' ? 'allowed' : 'denied';
	}

	async function toggleTool(surface: string, toolName: string) {
		const current = toolState(surface, toolName);
		const newState = current === 'allowed' ? 'denied' : 'allowed';
		await permissionsStore.setPermission(surface, toolName, newState);
	}
</script>

<div class="flex items-center justify-between mb-4">
	<h2 class="text-lg font-semibold">Tool Permissions</h2>
	<span class="text-xs text-muted-foreground">Configure which tools are available on each surface</span>
</div>

{#if loading}
	<div class="space-y-2">
		<Skeleton class="h-14 w-full" />
		<Skeleton class="h-14 w-full" />
		<Skeleton class="h-14 w-full" />
	</div>
{:else if permissionsStore.error}
	<Card.Root>
		<Card.Content class="py-6 text-center text-destructive">
			{permissionsStore.error}
		</Card.Content>
	</Card.Root>
{:else if getToolList().length === 0}
	<Card.Root>
		<Card.Content class="py-6 text-center text-muted-foreground">
			No tools registered. Start the daemon to see available tools.
		</Card.Content>
	</Card.Root>
{:else}
	<div class="overflow-x-auto rounded-lg border border-border">
		<table class="w-full text-sm">
			<thead>
				<tr class="border-b border-border bg-muted/50">
					<th class="text-left px-3 py-2.5 font-medium text-muted-foreground min-w-[220px]">Tool</th>
					{#each permissionsStore.surfaces as surface (surface)}
						<th class="text-center px-2 py-2.5 font-medium text-muted-foreground whitespace-nowrap min-w-[70px]">
							<div class="flex flex-col items-center gap-0.5">
								<span>{surfaceLabel(surface)}</span>
								{#if isLocalSurface(surface)}
									<span class="text-[9px] leading-none bg-green-500/20 text-green-600 dark:text-green-400 px-1 py-0.5 rounded">Local</span>
								{/if}
							</div>
						</th>
					{/each}
				</tr>
			</thead>
			<tbody>
				{#each groupedTools() as group (group.risk)}
					<!-- Risk group header -->
					<tr class="bg-muted/30">
						<td colspan={permissionsStore.surfaces.length + 1} class="px-3 py-1.5">
							<Badge variant={riskColor(group.risk)} class="text-[10px] uppercase tracking-wider">
								{group.risk} risk
							</Badge>
						</td>
					</tr>
					{#each group.tools as tool (tool.name)}
						<tr class="border-b border-border/50 hover:bg-muted/20 transition-colors">
							<td class="px-3 py-2">
								<div class="flex flex-col">
									<span class="font-mono text-xs">{tool.name}</span>
									{#if tool.description}
										<span class="text-[11px] text-muted-foreground leading-tight mt-0.5">{tool.description}</span>
									{/if}
								</div>
							</td>
							{#each permissionsStore.surfaces as surface (surface)}
								<td class="text-center px-2 py-2">
									<div class="flex justify-center">
										<Checkbox
											checked={toolState(surface, tool.name) === 'allowed'}
											onCheckedChange={() => toggleTool(surface, tool.name)}
										/>
									</div>
								</td>
							{/each}
						</tr>
					{/each}
				{/each}
			</tbody>
		</table>
	</div>

	<p class="text-xs text-muted-foreground mt-3">
		Local surfaces (Desktop, CLI, TUI) have all tools enabled by default. Remote channels deny high-risk tools by default.
	</p>
{/if}
