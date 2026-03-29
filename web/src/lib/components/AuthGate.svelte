<script lang="ts">
	import * as m from '$lib/paraglide/messages';
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
		healthCheckNoAuth,
		apiGet
	} from '$lib/api/client';
	import { isTauri, onGatewayReady, onGatewayFailed, getBootStatus } from '$lib/tauri';
	import { onDestroy } from 'svelte';
	import OnboardingWizard from '$lib/components/OnboardingWizard.svelte';

	let { children, onReady }: { children: any; onReady?: () => void } = $props();

	let authenticated = $state(false);
	let readyFired = false;
	let showSetup = $state(false);
	let detectedTimezone = $state('');
	let missingFields = $state<string[]>([]);
	let hasUsableModel = $state(false);
	let connecting = $state(false);
	let booting = $state(false);
	let tokenInput = $state('');
	let error = $state('');
	let checking = $state(false);
	let connectionFailed = $state(false);
	let bootErrorMessage = $state('');

	let pollTimeoutId: ReturnType<typeof setTimeout> | undefined;
	let unlistenReady: (() => void) | null = null;
	let unlistenFailed: (() => void) | null = null;

	const MAX_RETRIES = 10;
	const BOOT_MAX_RETRIES = 40;

	// Diagnostic log for debugging startup issues
	let diagLog = $state<string[]>([]);
	function diag(msg: string) {
		const ts = new Date().toISOString().slice(11, 23);
		diagLog = [...diagLog, `[${ts}] ${msg}`];
		console.log(`[AuthGate] ${msg}`);
	}

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
		bootErrorMessage = '';
		let attempt = 0;
		diag('waitForBoot started');

		// Listen for Tauri events for instant notification
		if (isTauri) {
			unlistenReady = await onGatewayReady(() => {
				diag('EVENT: gateway-ready received');
				clearPollTimeout();
				authenticated = true;
				booting = false;
				checkSetupStatus();
			});
			unlistenFailed = await onGatewayFailed((message) => {
				diag(`EVENT: gateway-failed: ${message}`);
				clearPollTimeout();
				booting = false;
				connectionFailed = true;
				bootErrorMessage = message;
			});
			diag('Tauri event listeners registered');
		}

		const poll = async () => {
			if (attempt >= BOOT_MAX_RETRIES) {
				diag(`Max retries (${BOOT_MAX_RETRIES}) exhausted`);
				booting = false;
				connectionFailed = true;
				if (isTauri && !bootErrorMessage) {
					const status = await getBootStatus();
					diag(`Final IPC boot status: ${JSON.stringify(status)}`);
					if (status?.status === 'Failed') {
						bootErrorMessage = status.message;
					}
				}
				return;
			}

			// In Tauri mode, use IPC to check boot status (bypasses CORS/mixed-content issues on Windows)
			if (isTauri) {
				try {
					const status = await getBootStatus();
					if (attempt % 5 === 0) diag(`Poll #${attempt} IPC status: ${JSON.stringify(status)}`);
					if (status?.status === 'Ready') {
						diag('IPC: boot status Ready — authenticating');
						authenticated = true;
						booting = false;
						checkSetupStatus();
						return;
					}
					if (status?.status === 'Failed') {
						diag(`IPC: boot status Failed: ${status.message}`);
						booting = false;
						connectionFailed = true;
						bootErrorMessage = status.message;
						return;
					}
				} catch (e) {
					diag(`IPC getBootStatus error: ${e}`);
				}
			} else {
				const ok = await healthCheckNoAuth();
				if (attempt % 5 === 0) diag(`Poll #${attempt} HTTP health: ${ok}`);
				if (ok) {
					authenticated = true;
					booting = false;
					checkSetupStatus();
					return;
				}
			}

			attempt++;
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
				checkSetupStatus();
				return;
			}

			attempt++;
			const delay = Math.min(1000 * Math.pow(2, attempt - 1), 30000);
			pollTimeoutId = setTimeout(poll, delay);
		};

		await poll();
	}

	async function init() {
		diag(`init: isTauri=${isTauri}, protocol=${typeof window !== 'undefined' ? window.location.protocol : 'N/A'}, origin=${typeof window !== 'undefined' ? window.location.origin : 'N/A'}`);

		if (isTauri) {
			try {
				const status = await getBootStatus();
				diag(`init IPC boot status: ${JSON.stringify(status)}`);
				if (status?.status === 'Ready') {
					diag('init: gateway already ready via IPC');
					authenticated = true;
					checkSetupStatus();
				} else {
					diag('init: gateway not ready, starting waitForBoot');
					waitForBoot();
				}
			} catch (e) {
				diag(`init IPC error: ${e}`);
				waitForBoot();
			}
			return;
		}

		// Browser mode: try without auth first
		diag('init: browser mode, trying healthCheckNoAuth');
		const noAuthOk = await healthCheckNoAuth();
		diag(`init: healthCheckNoAuth=${noAuthOk}`);
		if (noAuthOk) {
			authenticated = true;
			checkSetupStatus();
			return;
		}

		if (getToken()) {
			waitForGateway();
		}
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
			error = m.auth_token_required();
			return;
		}
		checking = true;
		error = '';
		setToken(tokenInput.trim());
		const ok = await healthCheck();
		if (ok) {
			authenticated = true;
			checkSetupStatus();
		} else {
			error = m.auth_connection_failed_error();
		}
		checking = false;
	}

	async function checkSetupStatus() {
		try {
			const status = await apiGet<{
				needs_setup: boolean;
				missing: string[];
				detected_timezone?: string;
				has_usable_model?: boolean;
			}>('/setup/status');
			if (status.needs_setup) {
				detectedTimezone = status.detected_timezone ?? Intl.DateTimeFormat().resolvedOptions().timeZone ?? '';
				missingFields = status.missing ?? [];
				hasUsableModel = status.has_usable_model ?? false;
				showSetup = true;
			}
		} catch {
			// Non-fatal: if endpoint unavailable, skip onboarding
		}
	}

	// Fire onReady callback once when authenticated and not in setup wizard
	$effect(() => {
		if (authenticated && !showSetup && !readyFired) {
			readyFired = true;
			onReady?.();
		}
	});

	onDestroy(() => {
		clearPollTimeout();
		unlistenReady?.();
		unlistenFailed?.();
	});
</script>

{#if authenticated}
	{#if showSetup}
		<OnboardingWizard
			{detectedTimezone}
			missing={missingFields}
			{hasUsableModel}
			oncomplete={() => (showSetup = false)}
		/>
	{:else}
		{@render children()}
	{/if}
{:else if booting}
	<div class="flex h-screen items-center justify-center">
		<div class="flex flex-col items-center gap-4 max-w-lg">
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
			<p class="text-sm text-muted-foreground">{m.auth_boot_starting()}</p>
			{#if diagLog.length > 0}
				<pre class="w-full mt-2 text-[10px] leading-tight text-muted-foreground bg-muted p-2 rounded max-h-32 overflow-auto whitespace-pre-wrap">{diagLog.join('\n')}</pre>
			{/if}
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
			<p class="text-sm text-muted-foreground">{m.auth_connecting_status()}</p>
			<Button variant="ghost" size="sm" onclick={handleReset}>
				{m.auth_reset_connection_button()}
			</Button>
		</div>
	</div>
{:else if connectionFailed}
	<div class="flex h-screen items-center justify-center">
		<div class="flex flex-col items-center gap-4 max-w-lg text-center">
			{#if isTauri}
				<p class="text-sm text-destructive">
					{bootErrorMessage || m.auth_boot_error_fallback()}
				</p>
				<Button
					variant="default"
					size="sm"
					onclick={() => {
						connectionFailed = false;
						waitForBoot();
					}}
				>
					{m.auth_retry_button()}
				</Button>
			{:else}
				<p class="text-sm text-destructive">
					{m.auth_connection_failed({ url: getBaseUrl() })}
				</p>
				<div class="flex gap-2">
					<Button variant="outline" size="sm" onclick={handleReset}>
						{m.auth_reset_button()}
					</Button>
					<Button
						variant="default"
						size="sm"
						onclick={() => {
							connectionFailed = false;
							waitForGateway();
						}}
					>
						{m.auth_retry_button()}
					</Button>
				</div>
			{/if}
			{#if diagLog.length > 0}
				<details class="w-full text-left mt-4">
					<summary class="text-xs text-muted-foreground cursor-pointer">{m.auth_diag_log_summary({ count: diagLog.length })}</summary>
					<pre class="mt-2 text-[10px] leading-tight text-muted-foreground bg-muted p-2 rounded max-h-48 overflow-auto whitespace-pre-wrap">{diagLog.join('\n')}</pre>
				</details>
			{/if}
		</div>
	</div>
{:else}
	<Dialog.Root open={true}>
		<Dialog.Content class="sm:max-w-md">
			<Dialog.Header>
				<Dialog.Title>{m.auth_connect_title()}</Dialog.Title>
				<Dialog.Description>
					{m.auth_connect_description()}
				</Dialog.Description>
			</Dialog.Header>
			<form onsubmit={handleSubmit} class="space-y-4">
				<Input
					type="password"
					placeholder={m.auth_token_placeholder()}
					bind:value={tokenInput}
				/>
				{#if error}
					<p class="text-sm text-destructive">{error}</p>
				{/if}
				<Button type="submit" class="w-full" disabled={checking}>
					{checking ? m.auth_connecting() : m.auth_connect_button()}
				</Button>
			</form>
		</Dialog.Content>
	</Dialog.Root>
{/if}
