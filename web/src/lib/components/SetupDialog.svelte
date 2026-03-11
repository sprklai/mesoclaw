<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import { configStore } from '$lib/stores/config.svelte';

	let { open = $bindable(true) }: { open: boolean } = $props();

	let location = $state('');
	let timezone = $state(Intl.DateTimeFormat().resolvedOptions().timeZone ?? '');
	let saving = $state(false);
	let error = $state('');

	async function handleSave() {
		if (!location.trim()) {
			error = 'Location is required (e.g., Toronto, Canada)';
			return;
		}
		saving = true;
		error = '';
		try {
			const updates: Record<string, string> = { user_location: location.trim() };
			if (timezone.trim()) {
				updates.user_timezone = timezone.trim();
			}
			await configStore.update(updates);
			open = false;
		} catch (e) {
			error = e instanceof Error ? e.message : String(e);
		} finally {
			saving = false;
		}
	}

	function handleSkip() {
		open = false;
	}
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-md">
		<Dialog.Header>
			<Dialog.Title>Welcome to MesoClaw</Dialog.Title>
			<Dialog.Description>
				Set your location so the AI can give you relevant, location-aware responses (weather, news, events, etc.).
			</Dialog.Description>
		</Dialog.Header>
		<div class="space-y-4">
			<div class="space-y-1">
				<label class="text-sm font-medium" for="setup-location">Location</label>
				<Input
					id="setup-location"
					bind:value={location}
					placeholder="e.g., Toronto, Canada"
				/>
			</div>
			<div class="space-y-1">
				<label class="text-sm font-medium" for="setup-timezone">Timezone</label>
				<Input
					id="setup-timezone"
					bind:value={timezone}
					placeholder="e.g., America/Toronto"
				/>
				<p class="text-xs text-muted-foreground">Auto-detected from your browser. Edit if incorrect.</p>
			</div>
			{#if error}
				<p class="text-sm text-destructive">{error}</p>
			{/if}
		</div>
		<Dialog.Footer class="flex gap-2 sm:justify-between">
			<Button variant="ghost" size="sm" onclick={handleSkip} disabled={saving}>
				Skip for now
			</Button>
			<Button onclick={handleSave} disabled={saving}>
				{saving ? 'Saving...' : 'Get Started'}
			</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
