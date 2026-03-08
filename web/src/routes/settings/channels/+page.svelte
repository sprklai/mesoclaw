<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { Skeleton } from '$lib/components/ui/skeleton';
	import { channelsStore, type ChannelWithStatus } from '$lib/stores/channels.svelte';
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';

	let expandedId = $state<string | null>(null);
	let credInputs = $state<Record<string, string>>({});
	let showField = $state<Record<string, boolean>>({});
	let revealedValues = $state<Record<string, string>>({});
	let revealing = $state<Record<string, boolean>>({});
	let saving = $state<Record<string, boolean>>({});
	let testing = $state<Record<string, boolean>>({});
	let testResult = $state<Record<string, { healthy: boolean; error?: string; latency_ms?: number } | null>>({});

	// Telegram settings local state
	let tgDmPolicy = $state('allowlist');
	let tgPollingTimeout = $state(30);
	let tgGroupMentionOnly = $state(true);

	onMount(async () => {
		await channelsStore.load();
		tgDmPolicy = channelsStore.channelConfig.telegram_dm_policy;
		tgPollingTimeout = channelsStore.channelConfig.telegram_polling_timeout;
		tgGroupMentionOnly = channelsStore.channelConfig.telegram_group_mention_only;
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

	async function toggleReveal(channelId: string, field: string) {
		const k = inputKey(channelId, field);
		if (showField[k]) {
			// Hide: clear revealed value
			showField[k] = false;
			delete revealedValues[k];
		} else {
			// Show: fetch actual value from backend
			revealing[k] = true;
			const val = await channelsStore.getCredentialValue(channelId, field);
			revealing[k] = false;
			if (val !== null) {
				revealedValues[k] = val;
				showField[k] = true;
			}
		}
	}

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

	async function removeCredential(channelId: string, field: string) {
		const k = inputKey(channelId, field);
		saving[k] = true;
		try {
			await channelsStore.removeCredential(channelId, field);
		} finally {
			saving[k] = false;
		}
	}

	async function testConnection(channelId: string) {
		testing[channelId] = true;
		testResult[channelId] = null;
		try {
			testResult[channelId] = await channelsStore.testConnection(channelId);
		} finally {
			testing[channelId] = false;
		}
	}

	async function saveTelegramConfig() {
		await channelsStore.updateConfig({
			telegram_dm_policy: tgDmPolicy,
			telegram_polling_timeout: tgPollingTimeout,
			telegram_group_mention_only: tgGroupMentionOnly,
		});
	}
</script>

<div class="max-w-2xl mx-auto space-y-6">
	<div class="flex items-center gap-3">
		<Button variant="ghost" size="sm" onclick={() => goto('/settings')}>
			&larr; Back
		</Button>
		<h1 class="text-2xl font-bold">Channels</h1>
	</div>

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
							<!-- Credential fields -->
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
										{#if showField[k] && revealedValues[k]}
											<Input
												id="cred-{k}"
												type="text"
												value={revealedValues[k]}
												readonly
												class="font-mono text-sm bg-muted"
											/>
										{:else}
											<Input
												id="cred-{k}"
												type={secret ? 'password' : 'text'}
												placeholder={isSet ? '\u2022\u2022\u2022\u2022\u2022\u2022\u2022\u2022  (value is set)' : cred.placeholder}
												bind:value={credInputs[k]}
											/>
										{/if}
										{#if secret && isSet}
											<Button
												variant="ghost"
												size="sm"
												disabled={revealing[k]}
												onclick={() => toggleReveal(channel.id, cred.key)}
											>
												{#if revealing[k]}
													...
												{:else}
													{showField[k] ? 'Hide' : 'Show'}
												{/if}
											</Button>
										{/if}
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

							<!-- Test Connection -->
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
									{#if testResult[channel.id]}
										{#if testResult[channel.id]?.healthy}
											<span class="text-sm text-green-600">
												Connected successfully
												{#if testResult[channel.id]?.latency_ms}
													({testResult[channel.id]?.latency_ms}ms)
												{/if}
											</span>
										{:else}
											<span class="text-sm text-destructive">
												{testResult[channel.id]?.error ?? 'Connection failed'}
											</span>
										{/if}
									{/if}
								</div>
							</div>

							<!-- Telegram-specific settings -->
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
</div>
