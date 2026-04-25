<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Switch } from '$lib/components/ui/switch';
	import { Label } from '$lib/components/ui/label';
	import { Textarea } from '$lib/components/ui/textarea';
	import { configStore } from '$lib/stores/config.svelte';
	import { onMount } from 'svelte';

	// ── Sub-tab state ────────────────────────────────────────────────────────────
	let activeTab = $state<'server' | 'clients'>('server');

	// ── Server sub-tab state ─────────────────────────────────────────────────────
	const snippetJson = `{
  "mcpServers": {
    "zenii": {
      "command": "zenii-mcp-server"
    }
  }
}`;
	let copied = $state(false);

	let prefix = $state('');
	let exposed = $state('');
	let hidden = $state('');
	let serverSaving = $state(false);
	let serverSaveMsg = $state('');
	let serverSaveError = $state('');

	// ── Clients sub-tab state ────────────────────────────────────────────────────
	interface McpTransportStdio {
		type: 'stdio';
		command: string;
		args: string[];
		env: Record<string, string>;
	}
	interface McpTransportHttp {
		type: 'http';
		url: string;
		headers: Record<string, string>;
	}
	interface McpServerConfig {
		id: string;
		transport: McpTransportStdio | McpTransportHttp;
		tools_prefix: string | null;
		enabled: boolean;
	}

	let servers = $state<McpServerConfig[]>([]);
	let showForm = $state(false);
	let editId = $state<string | null>(null);
	let clientsSaving = $state(false);

	let formId = $state('');
	let formTransport = $state<'stdio' | 'http'>('stdio');
	let formCommand = $state('');
	let formArgs = $state('');
	let formEnv = $state('');
	let formUrl = $state('');
	let formHeaders = $state('');
	let formPrefix = $state('');
	let formEnabled = $state(true);
	let formError = $state('');

	// ── Helpers ──────────────────────────────────────────────────────────────────
	function parseComma(s: string): string[] {
		return s
			.split(',')
			.map((x) => x.trim())
			.filter(Boolean);
	}

	function parseKeyVal(s: string): Record<string, string> {
		const result: Record<string, string> = {};
		for (const part of s.split(',')) {
			const idx = part.indexOf('=');
			if (idx === -1) continue;
			const key = part.slice(0, idx).trim();
			const val = part.slice(idx + 1).trim();
			if (key) result[key] = val;
		}
		return result;
	}

	function serializeKeyVal(obj: Record<string, string>): string {
		return Object.entries(obj)
			.map(([k, v]) => `${k}=${v}`)
			.join(', ');
	}

	// ── Mount ─────────────────────────────────────────────────────────────────────
	onMount(async () => {
		await configStore.load();
		prefix = String(configStore.get('mcp_server_tool_prefix') ?? '');
		const rawExposed = configStore.get('mcp_server_exposed_tools');
		exposed = Array.isArray(rawExposed) ? rawExposed.join(', ') : '';
		const rawHidden = configStore.get('mcp_server_hidden_tools');
		hidden = Array.isArray(rawHidden) ? rawHidden.join(', ') : '';
		const rawServers = configStore.get('mcp_client_servers');
		servers = Array.isArray(rawServers) ? (rawServers as McpServerConfig[]) : [];
	});

	// ── Server sub-tab actions ────────────────────────────────────────────────────
	async function copySnippet() {
		await navigator.clipboard.writeText(snippetJson);
		copied = true;
		setTimeout(() => {
			copied = false;
		}, 2000);
	}

	async function saveServerSettings() {
		serverSaving = true;
		serverSaveMsg = '';
		serverSaveError = '';
		try {
			await configStore.update({
				mcp_server_tool_prefix: prefix,
				mcp_server_exposed_tools: parseComma(exposed),
				mcp_server_hidden_tools: parseComma(hidden),
			});
			serverSaveMsg = 'Settings saved.';
		} catch (e) {
			serverSaveError = e instanceof Error ? e.message : String(e);
		} finally {
			serverSaving = false;
		}
	}

	// ── Clients sub-tab — presets ─────────────────────────────────────────────────
	interface Preset {
		label: string;
		id: string;
		command: string;
		args: string;
		env: string;
	}

	const presets: Preset[] = [
		{
			label: 'GitHub',
			id: 'github',
			command: 'npx',
			args: '-y, @modelcontextprotocol/server-github',
			env: 'GITHUB_PERSONAL_ACCESS_TOKEN=',
		},
		{
			label: 'Filesystem',
			id: 'filesystem',
			command: 'npx',
			args: '-y, @modelcontextprotocol/server-filesystem, /path/to/dir',
			env: '',
		},
		{
			label: 'Postgres',
			id: 'postgres',
			command: 'npx',
			args: '-y, @modelcontextprotocol/server-postgres, postgresql://localhost/mydb',
			env: '',
		},
		{
			label: 'Memory',
			id: 'memory',
			command: 'npx',
			args: '-y, @modelcontextprotocol/server-memory',
			env: '',
		},
		{
			label: 'Brave Search',
			id: 'brave-search',
			command: 'npx',
			args: '-y, @modelcontextprotocol/server-brave-search',
			env: 'BRAVE_API_KEY=',
		},
	];

	function applyPreset(preset: Preset) {
		resetForm();
		formId = preset.id;
		formTransport = 'stdio';
		formCommand = preset.command;
		formArgs = preset.args;
		formEnv = preset.env;
		showForm = true;
	}

	// ── Clients sub-tab — form ────────────────────────────────────────────────────
	function resetForm() {
		formId = '';
		formTransport = 'stdio';
		formCommand = '';
		formArgs = '';
		formEnv = '';
		formUrl = '';
		formHeaders = '';
		formPrefix = '';
		formEnabled = true;
		formError = '';
		editId = null;
		showForm = false;
	}

	function startAdd() {
		resetForm();
		showForm = true;
	}

	function startEdit(server: McpServerConfig) {
		editId = server.id;
		formId = server.id;
		formTransport = server.transport.type as 'stdio' | 'http';
		formPrefix = server.tools_prefix ?? '';
		formEnabled = server.enabled;
		formError = '';

		if (server.transport.type === 'stdio') {
			const t = server.transport as McpTransportStdio;
			formCommand = t.command;
			formArgs = t.args.join(', ');
			formEnv = serializeKeyVal(t.env);
			formUrl = '';
			formHeaders = '';
		} else {
			const t = server.transport as McpTransportHttp;
			formUrl = t.url;
			formHeaders = serializeKeyVal(t.headers);
			formCommand = '';
			formArgs = '';
			formEnv = '';
		}
		showForm = true;
	}

	async function saveServer() {
		if (clientsSaving) return;
		formError = '';

		if (!formId.trim()) {
			formError = 'ID is required';
			return;
		}
		if (formId.includes(' ')) {
			formError = 'ID cannot contain spaces';
			return;
		}
		const existing = servers.find((s) => s.id === formId.trim());
		if (existing && editId !== formId.trim()) {
			formError = 'ID already exists';
			return;
		}
		if (formTransport === 'stdio' && !formCommand.trim()) {
			formError = 'Command is required';
			return;
		}
		if (formTransport === 'http' && !formUrl.startsWith('http')) {
			formError = 'URL must start with http:// or https://';
			return;
		}

		const transport: McpTransportStdio | McpTransportHttp =
			formTransport === 'stdio'
				? {
						type: 'stdio',
						command: formCommand.trim(),
						args: parseComma(formArgs),
						env: parseKeyVal(formEnv),
					}
				: {
						type: 'http',
						url: formUrl.trim(),
						headers: parseKeyVal(formHeaders),
					};

		const newServer: McpServerConfig = {
			id: formId.trim(),
			transport,
			tools_prefix: formPrefix.trim() || null,
			enabled: formEnabled,
		};

		let updated: McpServerConfig[];
		if (editId) {
			updated = servers.map((s) => (s.id === editId ? newServer : s));
		} else {
			updated = [...servers, newServer];
		}

		clientsSaving = true;
		try {
			await configStore.update({ mcp_client_servers: updated });
			await configStore.load();
			const raw = configStore.get('mcp_client_servers');
			servers = Array.isArray(raw) ? (raw as McpServerConfig[]) : [];
			resetForm();
		} catch (e) {
			formError = e instanceof Error ? e.message : String(e);
		} finally {
			clientsSaving = false;
		}
	}

	async function deleteServer(id: string) {
		if (clientsSaving) return;
		clientsSaving = true;
		try {
			const updated = servers.filter((s) => s.id !== id);
			await configStore.update({ mcp_client_servers: updated });
			await configStore.load();
			const raw = configStore.get('mcp_client_servers');
			servers = Array.isArray(raw) ? (raw as McpServerConfig[]) : [];
		} finally {
			clientsSaving = false;
		}
	}

	async function toggleServer(id: string) {
		if (clientsSaving) return;
		clientsSaving = true;
		try {
			const updated = servers.map((s) =>
				s.id === id ? { ...s, enabled: !s.enabled } : s
			);
			await configStore.update({ mcp_client_servers: updated });
			await configStore.load();
			const raw = configStore.get('mcp_client_servers');
			servers = Array.isArray(raw) ? (raw as McpServerConfig[]) : [];
		} finally {
			clientsSaving = false;
		}
	}

	// ── Derived ───────────────────────────────────────────────────────────────────
	let transportSummary = $derived.by(() => {
		return (server: McpServerConfig): string => {
			if (server.transport.type === 'stdio') {
				const t = server.transport as McpTransportStdio;
				return `stdio: ${t.command} ${t.args.slice(0, 2).join(' ')}`;
			}
			return `http: ${(server.transport as McpTransportHttp).url}`;
		};
	});
</script>

<!-- Sub-tab nav -->
<div class="flex gap-1 border-b pb-2 mb-4">
	<button
		class="px-4 py-1.5 rounded-md text-sm font-medium transition-colors
			{activeTab === 'server' ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
		onclick={() => { activeTab = 'server'; }}
	>
		Server
	</button>
	<button
		class="px-4 py-1.5 rounded-md text-sm font-medium transition-colors
			{activeTab === 'clients' ? 'bg-accent text-accent-foreground' : 'text-muted-foreground hover:bg-muted hover:text-foreground'}"
		onclick={() => { activeTab = 'clients'; }}
	>
		Clients
	</button>
</div>

<!-- ── Server sub-tab ──────────────────────────────────────────────────────── -->
{#if activeTab === 'server'}
	<!-- Card 1: Connect your AI clients -->
	<Card.Root>
		<Card.Header class="py-3">
			<Card.Title class="text-base">Connect your AI clients</Card.Title>
			<Card.Description>
				Use Zenii's tools from Claude Desktop, Cursor, Windsurf, or any MCP-compatible client.
			</Card.Description>
		</Card.Header>
		<Card.Content class="space-y-3">
			<p class="text-sm text-muted-foreground">
				Add the following to your MCP client configuration file:
			</p>
			<div class="relative">
				<pre class="rounded bg-muted px-3 py-3 text-xs overflow-x-auto font-mono">{snippetJson}</pre>
				<Button
					size="sm"
					variant="outline"
					class="absolute top-2 right-2 text-xs"
					onclick={copySnippet}
				>
					{copied ? 'Copied!' : 'Copy'}
				</Button>
			</div>
		</Card.Content>
	</Card.Root>

	<!-- Card 2: Tool Visibility -->
	<Card.Root>
		<Card.Header class="py-3">
			<Card.Title class="text-base">Tool Visibility</Card.Title>
			<Card.Description>
				Control which tools are exposed to MCP clients and how they are named.
			</Card.Description>
		</Card.Header>
		<Card.Content class="space-y-4">
			<div class="space-y-1.5">
				<Label for="mcp-prefix">Tool name prefix</Label>
				<Input
					id="mcp-prefix"
					placeholder="e.g. zenii_"
					bind:value={prefix}
				/>
				<p class="text-xs text-muted-foreground">Optional prefix added to all exposed tool names.</p>
			</div>

			<div class="space-y-1.5">
				<Label for="mcp-exposed">Exposed tools <span class="font-normal text-muted-foreground">(empty = all)</span></Label>
				<Textarea
					id="mcp-exposed"
					placeholder="web_search, file_read, ..."
					rows={3}
					bind:value={exposed}
				/>
				<p class="text-xs text-muted-foreground">Comma-separated list of tool names to expose. Leave empty to expose all tools.</p>
			</div>

			<div class="space-y-1.5">
				<Label for="mcp-hidden">Hidden tools</Label>
				<Textarea
					id="mcp-hidden"
					placeholder="shell, process, ..."
					rows={3}
					bind:value={hidden}
				/>
				<p class="text-xs text-muted-foreground">Comma-separated list of tool names to hide from MCP clients.</p>
			</div>

			{#if serverSaveMsg}
				<p class="text-sm text-green-600 dark:text-green-400">{serverSaveMsg}</p>
			{/if}
			{#if serverSaveError}
				<p class="text-sm text-destructive">{serverSaveError}</p>
			{/if}

			<Button onclick={saveServerSettings} disabled={serverSaving}>
				{serverSaving ? 'Saving…' : 'Save'}
			</Button>
		</Card.Content>
	</Card.Root>
{/if}

<!-- ── Clients sub-tab ─────────────────────────────────────────────────────── -->
{#if activeTab === 'clients'}
	<!-- Quick Add Presets -->
	<Card.Root>
		<Card.Header class="py-3">
			<Card.Title class="text-base">Quick Add</Card.Title>
			<Card.Description>Add a popular MCP server with one click.</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="flex flex-wrap gap-2">
				{#each presets as preset (preset.id)}
					<Button
						size="sm"
						variant="outline"
						onclick={() => applyPreset(preset)}
					>
						{preset.label}
					</Button>
				{/each}
				<Button size="sm" onclick={startAdd}>+ Custom</Button>
			</div>
		</Card.Content>
	</Card.Root>

	<!-- Add/Edit Form -->
	{#if showForm}
		<Card.Root>
			<Card.Header class="py-3">
				<Card.Title class="text-base">{editId ? 'Edit Server' : 'Add MCP Server'}</Card.Title>
			</Card.Header>
			<Card.Content class="space-y-4">
				<div class="space-y-1.5">
					<Label for="form-id">Server ID</Label>
					<Input
						id="form-id"
						placeholder="e.g. github"
						bind:value={formId}
						disabled={editId !== null}
					/>
				</div>

				<!-- Transport radio -->
				<div class="space-y-1.5">
					<Label>Transport</Label>
					<div class="flex gap-4">
						<label class="flex items-center gap-2 text-sm cursor-pointer">
							<input
								type="radio"
								name="transport"
								value="stdio"
								checked={formTransport === 'stdio'}
								onchange={() => { formTransport = 'stdio'; }}
							/>
							Stdio
						</label>
						<label class="flex items-center gap-2 text-sm cursor-pointer">
							<input
								type="radio"
								name="transport"
								value="http"
								checked={formTransport === 'http'}
								onchange={() => { formTransport = 'http'; }}
							/>
							HTTP
						</label>
					</div>
				</div>

				{#if formTransport === 'stdio'}
					<div class="space-y-1.5">
						<Label for="form-cmd">Command</Label>
						<Input id="form-cmd" placeholder="npx" bind:value={formCommand} />
					</div>
					<div class="space-y-1.5">
						<Label for="form-args">Arguments <span class="font-normal text-muted-foreground">(comma-separated)</span></Label>
						<Input id="form-args" placeholder="-y, @modelcontextprotocol/server-github" bind:value={formArgs} />
					</div>
					<div class="space-y-1.5">
						<Label for="form-env">Environment variables <span class="font-normal text-muted-foreground">(KEY=VAL, KEY2=VAL2)</span></Label>
						<Input id="form-env" placeholder="GITHUB_PERSONAL_ACCESS_TOKEN=" bind:value={formEnv} />
					</div>
				{:else}
					<div class="space-y-1.5">
						<Label for="form-url">URL</Label>
						<Input id="form-url" placeholder="https://my-mcp-server.example.com" bind:value={formUrl} />
					</div>
					<div class="space-y-1.5">
						<Label for="form-headers">Headers <span class="font-normal text-muted-foreground">(KEY=VAL, KEY2=VAL2)</span></Label>
						<Input id="form-headers" placeholder="Authorization=Bearer token123" bind:value={formHeaders} />
					</div>
				{/if}

				<div class="space-y-1.5">
					<Label for="form-tools-prefix">Tools prefix <span class="font-normal text-muted-foreground">(optional)</span></Label>
					<Input id="form-tools-prefix" placeholder="Leave empty to use global prefix" bind:value={formPrefix} />
				</div>

				<div class="flex items-center gap-2">
					<Switch bind:checked={formEnabled} id="form-enabled" />
					<Label for="form-enabled">Enabled</Label>
				</div>

				{#if formError}
					<p class="text-sm text-destructive">{formError}</p>
				{/if}

				<div class="flex gap-2 pt-1">
					<Button onclick={saveServer} disabled={clientsSaving}>{clientsSaving ? 'Saving…' : 'Save'}</Button>
					<Button variant="outline" onclick={resetForm} disabled={clientsSaving}>Cancel</Button>
				</div>
			</Card.Content>
		</Card.Root>
	{/if}

	<!-- Server list -->
	{#if servers.length === 0 && !showForm}
		<p class="text-sm text-muted-foreground py-4">
			No MCP clients configured. Add a server above to get started.
		</p>
	{:else if servers.length > 0}
		<div class="space-y-2">
			{#each servers as server (server.id)}
				<Card.Root>
					<Card.Content class="py-3">
						<div class="flex items-center gap-3">
							<Switch
								checked={server.enabled}
								disabled={clientsSaving}
								onCheckedChange={() => toggleServer(server.id)}
							/>
							<div class="flex-1 min-w-0">
								<div class="flex items-center gap-2">
									<span class="font-medium text-sm">{server.id}</span>
									{#if server.tools_prefix}
										<Badge variant="outline" class="text-xs">{server.tools_prefix}</Badge>
									{/if}
								</div>
								<p class="text-xs text-muted-foreground truncate">{transportSummary(server)}</p>
							</div>
							<div class="flex items-center gap-2 shrink-0">
								<Button
									size="sm"
									variant="ghost"
									disabled={clientsSaving}
									onclick={() => startEdit(server)}
								>
									Edit
								</Button>
								<Button
									size="sm"
									variant="ghost"
									class="text-destructive hover:text-destructive"
									disabled={clientsSaving}
									onclick={() => deleteServer(server.id)}
								>
									{clientsSaving ? '…' : 'Delete'}
								</Button>
							</div>
						</div>
					</Card.Content>
				</Card.Root>
			{/each}
		</div>
	{/if}
{/if}
