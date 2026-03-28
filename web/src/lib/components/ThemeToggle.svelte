<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import Sun from '@lucide/svelte/icons/sun';
	import Moon from '@lucide/svelte/icons/moon';
	import Monitor from '@lucide/svelte/icons/monitor';
	import { themeStore } from '$lib/stores/theme.svelte';
	import * as m from '$lib/paraglide/messages';

	const icons = { light: Sun, dark: Moon, system: Monitor } as const;
	const labels = {
		light: () => m.settings_general_theme_light(),
		dark: () => m.settings_general_theme_dark(),
		system: () => m.settings_general_theme_system()
	} as const;
</script>

<Button variant="ghost" size="sm" onclick={() => themeStore.toggle()} class="w-full justify-start gap-2">
	{@const Icon = icons[themeStore.theme]}
	<Icon class="h-4 w-4" />
	<span class="text-xs">{labels[themeStore.theme]()}</span>
</Button>
