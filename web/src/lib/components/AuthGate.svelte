<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import {
		getToken,
		setToken,
		clearToken,
		getBaseUrl,
		clearBaseUrl,
		healthCheck,
		healthCheckNoAuth
	} from '$lib/api/client';
	import { isTauri } from '$lib/tauri';
	import { onDestroy } from 'svelte';

	let { children } = $props();

	let authenticated = $state(false);
	let connecting = $state(false);
	let booting = $state(false);
	let tokenInput = $state('');
	let error = $state('');
	let checking = $state(false);
	let connectionFailed = $state(false);

	let pollTimeoutId: ReturnType<typeof setTimeout> | undefined;

	const MAX_RETRIES = 10;
	const BOOT_MAX_RETRIES = 20;

	function clearPollTimeout() {
		if (pollTimeoutId !== undefined) {
			clearTimeout(pollTimeoutId);
			pollTimeoutId = undefined;
		}
	}

	/** Poll for embedded gateway startup (Tauri desktop mode, no auth needed). */
	async function waitForBoot() {
		booting = true;
		connectionFailed = false;
		let attempt = 0;

		const poll = async () => {
			if (attempt >= BOOT_MAX_RETRIES) {
				booting = false;
				connectionFailed = true;
				return;
			}

			const ok = await healthCheckNoAuth();
			if (ok) {
				authenticated = true;
				booting = false;
				return;
			}

			attempt++;
			// Short fixed intervals for boot (500ms), not exponential backoff
			pollTimeoutId = setTimeout(poll, 500);
		};

		await poll();
	}

	/** Poll for external gateway with auth token. */
	async function waitForGateway() {
		connecting = true;
		connectionFailed = false;
		let attempt = 0;

		const poll = async () => {
			if (attempt >= MAX_RETRIES) {
				connecting = false;
				connectionFailed = true;
				return;
			}

			const ok = await healthCheck();
			if (ok) {
				authenticated = true;
				connecting = false;
				return;
			}

			attempt++;
			const delay = Math.min(1000 * Math.pow(2, attempt - 1), 30000);
			pollTimeoutId = setTimeout(poll, delay);
		};

		await poll();
	}

	async function init() {
		// Try without auth first -- if health returns 200, gateway is ready
		const noAuthOk = await healthCheckNoAuth();
		if (noAuthOk) {
			authenticated = true;
			return;
		}

		if (isTauri) {
			// Desktop app: embedded gateway is still booting, wait for it
			waitForBoot();
		} else if (getToken()) {
			// Browser with cached token: poll with auth
			waitForGateway();
		}
		// Browser without token: fall through to token dialog
	}

	init();

	function handleReset() {
		clearPollTimeout();
		clearToken();
		clearBaseUrl();
		authenticated = false;
		connecting = false;
		connectionFailed = false;
		error = '';
		tokenInput = '';
	}

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		if (!tokenInput.trim()) {
			error = 'Token is required';
			return;
		}
		checking = true;
		error = '';
		setToken(tokenInput.trim());
		const ok = await healthCheck();
		if (ok) {
			authenticated = true;
		} else {
			error = 'Could not connect to daemon. Check the token and ensure the daemon is running.';
		}
		checking = false;
	}

	onDestroy(() => {
		clearPollTimeout();
	});
</script>

{#if authenticated}
	{@render children()}
{:else if booting}
	<div class="flex h-screen items-center justify-center">
		<div class="flex flex-col items-center gap-4">
			<svg
				class="h-8 w-8 animate-spin text-muted-foreground"
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
			>
				<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"
				></circle>
				<path
					class="opacity-75"
					fill="currentColor"
					d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
				></path>
			</svg>
			<p class="text-sm text-muted-foreground">Starting MesoClaw...</p>
		</div>
	</div>
{:else if connecting}
	<div class="flex h-screen items-center justify-center">
		<div class="flex flex-col items-center gap-4">
			<svg
				class="h-8 w-8 animate-spin text-muted-foreground"
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
			>
				<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"
				></circle>
				<path
					class="opacity-75"
					fill="currentColor"
					d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
				></path>
			</svg>
			<p class="text-sm text-muted-foreground">Connecting to MesoClaw...</p>
			<Button variant="ghost" size="sm" onclick={handleReset}>
				Reset connection
			</Button>
		</div>
	</div>
{:else if connectionFailed}
	<div class="flex h-screen items-center justify-center">
		<div class="flex flex-col items-center gap-4 max-w-md text-center">
			{#if isTauri}
				<p class="text-sm text-destructive">
					MesoClaw failed to start. Check the logs for errors.
				</p>
				<Button
					variant="default"
					size="sm"
					onclick={() => {
						connectionFailed = false;
						waitForBoot();
					}}
				>
					Retry
				</Button>
			{:else}
				<p class="text-sm text-destructive">
					Cannot reach MesoClaw at {getBaseUrl()}. Check that the daemon is running.
				</p>
				<div class="flex gap-2">
					<Button variant="outline" size="sm" onclick={handleReset}>
						Change URL / Reset
					</Button>
					<Button
						variant="default"
						size="sm"
						onclick={() => {
							connectionFailed = false;
							waitForGateway();
						}}
					>
						Retry
					</Button>
				</div>
			{/if}
		</div>
	</div>
{:else}
	<Dialog.Root open={true}>
		<Dialog.Content class="sm:max-w-md">
			<Dialog.Header>
				<Dialog.Title>Connect to MesoClaw</Dialog.Title>
				<Dialog.Description>
					Enter your gateway authentication token to connect.
				</Dialog.Description>
			</Dialog.Header>
			<form onsubmit={handleSubmit} class="space-y-4">
				<Input
					type="password"
					placeholder="Bearer token"
					bind:value={tokenInput}
				/>
				{#if error}
					<p class="text-sm text-destructive">{error}</p>
				{/if}
				<Button type="submit" class="w-full" disabled={checking}>
					{checking ? 'Connecting...' : 'Connect'}
				</Button>
			</form>
		</Dialog.Content>
	</Dialog.Root>
{/if}
