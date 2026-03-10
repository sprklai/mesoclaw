<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import { channelsStore, type ChannelWithStatus } from '$lib/stores/channels.svelte';
	import { onMount } from 'svelte';

	let expandedId = $state<string | null>(null);
	let confirmOpen = $state(false);
	let deleteTarget = $state<{ channelId: string; field: string } | null>(null);
	let credInputs = $state<Record<string, string>>({});
	let saving = $state<Record<string, boolean>>({});
	let testing = $state<Record<string, boolean>>({});
	let testResult = $state<Record<string, { healthy: boolean; error?: string; latency_ms?: number } | null>>({});

	let tgDmPolicy = $state('allowlist');
	let tgPollingTimeout = $state(30);
	let tgGroupMentionOnly = $state(true);

	onMount(async () => {
		await channelsStore.load();
		tgDmPolicy = channelsStore.channelConfig.telegram_dm_policy;
		tgPollingTimeout = channelsStore.channelConfig.telegram_polling_timeout_secs;
		tgGroupMentionOnly = channelsStore.channelConfig.telegram_require_group_mention;
	});

	function toggle(id: string) {
		expandedId = expandedId === id ? null : id;
	}

	function inputKey(channelId: string, field: string): string {
		return `${channelId}:${field}`;
	}

	function isSecretField(field: string): boolean {
		return field === 'token' || field === 'bot_token' || field === 'app_token' || field === 'access_token';
	}

	// Credential values are never exposed over the gateway for security.
	// The reveal button has been removed.

	function statusDotClass(ch: ChannelWithStatus): string {
		if (ch.connected) return 'bg-green-500';
		if (ch.configuredKeys.size > 0) return 'bg-muted-foreground/40';
		return 'bg-muted-foreground/40';
	}

	function statusLabel(ch: ChannelWithStatus): string {
		return ch.status;
	}

	async function saveCredential(channelId: string, field: string) {
		const k = inputKey(channelId, field);
		const value = credInputs[k];
		if (!value?.trim()) return;
		saving[k] = true;
		try {
			await channelsStore.setCredential(channelId, field, value.trim());
			credInputs[k] = '';
		} finally {
			saving[k] = false;
		}
	}

	function removeCredential(channelId: string, field: string) {
		deleteTarget = { channelId, field };
		confirmOpen = true;
	}

	async function confirmRemoveCredential() {
		if (!deleteTarget) return;
		const { channelId, field } = deleteTarget;
		const k = inputKey(channelId, field);
		saving[k] = true;
		try {
			await channelsStore.removeCredential(channelId, field);
		} finally {
			saving[k] = false;
		}
	}

	let disconnecting = $state<Record<string, boolean>>({});
	let connecting = $state<Record<string, boolean>>({});

	async function testConnection(channelId: string) {
		testing[channelId] = true;
		testResult[channelId] = null;
		try {
			const result = await channelsStore.testConnection(channelId);
			testResult[channelId] = result;
			if (result.healthy) {
				connecting[channelId] = true;
				testing[channelId] = false;
				const connected = await channelsStore.connectChannel(channelId);
				connecting[channelId] = false;
				// Verify actual status from registry
				const ch = channelsStore.channels.find((c) => c.id === channelId);
				if (!connected || !ch?.connected) {
					testResult[channelId] = {
						healthy: false,
						error: 'Test passed but connection failed. Check backend logs.',
						latency_ms: result.latency_ms,
					};
				}
			}
		} finally {
			testing[channelId] = false;
			connecting[channelId] = false;
		}
	}

	async function disconnectChannel(channelId: string) {
		disconnecting[channelId] = true;
		try {
			await channelsStore.disconnectChannel(channelId);
			testResult[channelId] = null;
		} finally {
			disconnecting[channelId] = false;
		}
	}

	async function saveTelegramConfig() {
		await channelsStore.updateConfig({
			telegram_dm_policy: tgDmPolicy,
			telegram_polling_timeout_secs: tgPollingTimeout,
			telegram_require_group_mention: tgGroupMentionOnly,
		});
	}
</script>

{#if channelsStore.loading}
	<div class="space-y-2">
		<Skeleton class="h-16 w-full" />
		<Skeleton class="h-16 w-full" />
		<Skeleton class="h-16 w-full" />
		<Skeleton class="h-16 w-full" />
	</div>
{:else}
	<div class="space-y-2">
		{#each channelsStore.channels as channel (channel.id)}
			<Card.Root>
				<button
					class="w-full text-left"
					onclick={() => toggle(channel.id)}
				>
					<Card.Header class="py-3">
						<div class="flex items-center justify-between">
							<div class="flex items-center gap-2">
								<span
									class="inline-block h-2.5 w-2.5 rounded-full {statusDotClass(channel)}"
									title={statusLabel(channel)}
								></span>
								<Card.Title class="text-base">{channel.name}</Card.Title>
								<Badge variant="outline">{channel.description}</Badge>
								<span class="text-xs text-muted-foreground">
									{statusLabel(channel)}
								</span>
							</div>
							<span class="text-xs text-muted-foreground">
								{expandedId === channel.id ? '\u25B2' : '\u25BC'}
							</span>
						</div>
					</Card.Header>
				</button>

				{#if expandedId === channel.id}
					<Card.Content class="pt-0 space-y-4">
						{#each channel.credentials as cred (cred.key)}
							{@const k = inputKey(channel.id, cred.key)}
							{@const isSet = channel.configuredKeys.has(cred.key)}
							{@const secret = isSecretField(cred.key)}
							<div class="space-y-2">
								<label class="text-sm font-medium" for="cred-{k}">
									{cred.label}
									{#if isSet}
										<Badge variant="default" class="ml-2 text-xs">Set</Badge>
									{/if}
								</label>
								<div class="flex gap-2">
									<Input
										id="cred-{k}"
										type={secret ? 'password' : 'text'}
										placeholder={isSet ? '\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022  (value is set)' : cred.placeholder}
										bind:value={credInputs[k]}
									/>
								</div>
								<div class="flex gap-2">
									<Button
										size="sm"
										disabled={!credInputs[k]?.trim() || saving[k]}
										onclick={() => saveCredential(channel.id, cred.key)}
									>
										{saving[k] ? 'Saving...' : 'Save'}
									</Button>
									{#if isSet}
										<Button
											variant="destructive"
											size="sm"
											disabled={saving[k]}
											onclick={() => removeCredential(channel.id, cred.key)}
										>
											Remove
										</Button>
									{/if}
								</div>
							</div>
						{/each}

						<div class="border-t pt-3 space-y-2">
							<div class="flex items-center gap-2">
								<Button
									size="sm"
									variant="outline"
									disabled={testing[channel.id]}
									onclick={() => testConnection(channel.id)}
								>
									{testing[channel.id] ? 'Testing...' : 'Test Connection'}
								</Button>
								{#if channel.connected}
									<Button
										size="sm"
										variant="destructive"
										disabled={disconnecting[channel.id]}
										onclick={() => disconnectChannel(channel.id)}
									>
										{disconnecting[channel.id] ? 'Disconnecting...' : 'Disconnect'}
									</Button>
								{/if}
								{#if connecting[channel.id]}
									<span class="text-sm text-muted-foreground">
										Test passed, connecting...
									</span>
								{:else if testResult[channel.id]}
									{#if testResult[channel.id]?.healthy && channel.connected}
										<span class="text-sm text-green-600">
											Connected
											{#if testResult[channel.id]?.latency_ms}
												({testResult[channel.id]?.latency_ms}ms)
											{/if}
										</span>
									{:else if !testResult[channel.id]?.healthy}
										<span class="text-sm text-destructive">
											{testResult[channel.id]?.error ?? 'Connection failed'}
										</span>
									{/if}
								{/if}
							</div>
						</div>

						{#if channel.id === 'telegram'}
							<div class="border-t pt-4 space-y-3">
								<h3 class="text-sm font-semibold">Telegram Settings</h3>
								<div class="space-y-1">
									<label class="text-sm font-medium" for="tg-dm-policy">DM Policy</label>
									<select
										id="tg-dm-policy"
										class="flex h-9 w-full rounded-md border border-input bg-background text-foreground px-3 py-1 text-sm shadow-sm"
										bind:value={tgDmPolicy}
									>
										<option value="allowlist">Allowlist Only</option>
										<option value="open">Open</option>
										<option value="disabled">Disabled</option>
									</select>
								</div>
								<div class="space-y-1">
									<label class="text-sm font-medium" for="tg-polling">Polling Timeout (seconds)</label>
									<Input
										id="tg-polling"
										type="number"
										min={5}
										max={60}
										bind:value={tgPollingTimeout}
									/>
								</div>
								<div class="flex items-center gap-2">
									<input
										id="tg-group-mention"
										type="checkbox"
										class="h-4 w-4 rounded border-input"
										bind:checked={tgGroupMentionOnly}
									/>
									<label class="text-sm font-medium" for="tg-group-mention">
										Group: respond only when mentioned
									</label>
								</div>
								<Button size="sm" onclick={saveTelegramConfig}>
									Save Telegram Settings
								</Button>
							</div>
						{/if}
					</Card.Content>
				{/if}
			</Card.Root>
		{/each}
	</div>
{/if}

<ConfirmDialog
	bind:open={confirmOpen}
	title="Remove credential?"
	description="This will remove the stored credential for this channel."
	confirmLabel="Remove"
	onConfirm={confirmRemoveCredential}
/>
