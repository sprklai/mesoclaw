<script lang="ts">
	import Activity from '@lucide/svelte/icons/activity';
	import Bell from '@lucide/svelte/icons/bell';
	import Bot from '@lucide/svelte/icons/bot';
	import Brain from '@lucide/svelte/icons/brain';
	import CalendarPlus from '@lucide/svelte/icons/calendar-plus';
	import Clock from '@lucide/svelte/icons/clock';
	import Database from '@lucide/svelte/icons/database';
	import FileDiff from '@lucide/svelte/icons/file-diff';
	import FilePlus from '@lucide/svelte/icons/file-plus';
	import FileText from '@lucide/svelte/icons/file-text';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import GitBranch from '@lucide/svelte/icons/git-branch';
	import GitFork from '@lucide/svelte/icons/git-fork';
	import Globe from '@lucide/svelte/icons/globe';
	import Monitor from '@lucide/svelte/icons/monitor';
	import PlayCircle from '@lucide/svelte/icons/play-circle';
	import Search from '@lucide/svelte/icons/search';
	import SearchCode from '@lucide/svelte/icons/search-code';
	import Send from '@lucide/svelte/icons/send';
	import Settings from '@lucide/svelte/icons/settings';
	import Settings2 from '@lucide/svelte/icons/settings-2';
	import Terminal from '@lucide/svelte/icons/terminal';
	import Timer from '@lucide/svelte/icons/timer';
	import ToggleLeft from '@lucide/svelte/icons/toggle-left';
	import Trash2 from '@lucide/svelte/icons/trash-2';
	import Zap from '@lucide/svelte/icons/zap';

	let { name, class: cls = '' }: { name: string; class?: string } = $props();

	const icons: Record<string, unknown> = {
		Activity,
		Bell,
		Bot,
		Brain,
		CalendarPlus,
		Clock,
		Database,
		FileDiff,
		FilePlus,
		FileText,
		FolderOpen,
		GitBranch,
		GitFork,
		Globe,
		Monitor,
		PlayCircle,
		Search,
		SearchCode,
		Send,
		Settings,
		Settings2,
		Terminal,
		Timer,
		ToggleLeft,
		Trash2,
		Zap
	};

	const Icon = $derived((icons[name] ?? Zap) as typeof Zap);
</script>

<Icon class={cls} />
