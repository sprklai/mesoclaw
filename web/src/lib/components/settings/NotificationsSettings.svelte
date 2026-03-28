<script lang="ts">
	import { onMount } from 'svelte';
	import * as Card from '$lib/components/ui/card';
	import { Checkbox } from '$lib/components/ui/checkbox';
	import { Label } from '$lib/components/ui/label';
	import { configStore } from '$lib/stores/config.svelte';
	import { channelsStore } from '$lib/stores/channels.svelte';
	import * as m from '$lib/paraglide/messages';

	const EVENT_TYPES = [
		{ key: 'scheduler_notification', label: () => m.settings_notifications_event_scheduler() },
		{ key: 'scheduler_job_completed', label: () => m.settings_notifications_event_job_completed() },
		{ key: 'channel_message', label: () => m.settings_notifications_event_channel_message() }
	] as const;

	const TARGETS = [
		{ key: 'toast', label: () => m.settings_notifications_target_toast(), channelId: null },
		{ key: 'desktop', label: () => m.settings_notifications_target_desktop(), channelId: null },
		{ key: 'telegram', label: () => m.settings_notifications_target_telegram(), channelId: 'telegram' },
		{ key: 'slack', label: () => m.settings_notifications_target_slack(), channelId: 'slack' },
		{ key: 'discord', label: () => m.settings_notifications_target_discord(), channelId: 'discord' }
	] as const;

	let saving = $state(false);

	onMount(() => {
		channelsStore.load();
	});

	function getRouting(): Record<string, string[]> {
		const raw = configStore.config.notification_routing;
		if (raw && typeof raw === 'object') {
			return raw as Record<string, string[]>;
		}
		return {
			scheduler_notification: ['toast', 'desktop'],
			scheduler_job_completed: ['toast', 'desktop'],
			channel_message: ['toast', 'desktop']
		};
	}

	function isEnabled(eventKey: string, targetKey: string): boolean {
		const routing = getRouting();
		const targets = routing[eventKey];
		return Array.isArray(targets) && targets.includes(targetKey);
	}

	function isChannelConfigured(channelId: string | null): boolean {
		if (!channelId) return true;
		const ch = channelsStore.channels.find((c) => c.id === channelId);
		return !!ch && ch.configuredKeys.size > 0;
	}

	async function toggle(eventKey: string, targetKey: string) {
		saving = true;
		try {
			const routing = getRouting();
			const current = Array.isArray(routing[eventKey]) ? [...routing[eventKey]] : [];
			const idx = current.indexOf(targetKey);
			if (idx >= 0) {
				current.splice(idx, 1);
			} else {
				current.push(targetKey);
			}
			const updated = { ...routing, [eventKey]: current };
			await configStore.update({ notification_routing: updated });
		} finally {
			saving = false;
		}
	}
</script>

<Card.Root>
	<Card.Header>
		<Card.Title>{m.settings_notifications_title()}</Card.Title>
		<Card.Description>
			{m.settings_notifications_description()}
		</Card.Description>
	</Card.Header>
	<Card.Content>
		<div class="overflow-x-auto">
			<table class="w-full text-sm">
				<thead>
					<tr class="border-b">
						<th class="text-left py-2 pr-4 font-medium text-muted-foreground">{m.settings_permissions_column_event()}</th>
						{#each TARGETS as target}
							<th class="text-center py-2 px-3 font-medium text-muted-foreground">
								{target.label()}
							</th>
						{/each}
					</tr>
				</thead>
				<tbody>
					{#each EVENT_TYPES as event}
						<tr class="border-b last:border-0">
							<td class="py-3 pr-4">
								<Label>{event.label()}</Label>
							</td>
							{#each TARGETS as target}
								{@const configured = isChannelConfigured(target.channelId)}
								<td class="text-center py-3 px-3">
									<div class="flex justify-center">
										<Checkbox
											checked={isEnabled(event.key, target.key)}
											disabled={saving || !configured}
											onCheckedChange={() => toggle(event.key, target.key)}
										/>
									</div>
									{#if !configured}
										<p class="text-[10px] text-muted-foreground mt-0.5">{m.settings_notifications_not_available()}</p>
									{/if}
								</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</Card.Content>
</Card.Root>
