export interface CategoryStyle {
	iconBg: string;
	iconText: string;
	accent: string;
	handleBorder: string;
}

export const CATEGORY_STYLES: Record<string, CategoryStyle> = {
	triggers: {
		iconBg: 'bg-violet-500/15',
		iconText: 'text-violet-500',
		accent: 'border-l-violet-500',
		handleBorder: '!border-violet-500'
	},
	ai: {
		iconBg: 'bg-indigo-500/15',
		iconText: 'text-indigo-500',
		accent: 'border-l-indigo-500',
		handleBorder: '!border-indigo-500'
	},
	search: {
		iconBg: 'bg-cyan-500/15',
		iconText: 'text-cyan-500',
		accent: 'border-l-cyan-500',
		handleBorder: '!border-cyan-500'
	},
	system: {
		iconBg: 'bg-slate-500/15',
		iconText: 'text-slate-400',
		accent: 'border-l-slate-400',
		handleBorder: '!border-slate-400'
	},
	files: {
		iconBg: 'bg-orange-500/15',
		iconText: 'text-orange-500',
		accent: 'border-l-orange-500',
		handleBorder: '!border-orange-500'
	},
	memory: {
		iconBg: 'bg-emerald-500/15',
		iconText: 'text-emerald-500',
		accent: 'border-l-emerald-500',
		handleBorder: '!border-emerald-500'
	},
	channels: {
		iconBg: 'bg-pink-500/15',
		iconText: 'text-pink-500',
		accent: 'border-l-pink-500',
		handleBorder: '!border-pink-500'
	},
	config: {
		iconBg: 'bg-amber-500/15',
		iconText: 'text-amber-500',
		accent: 'border-l-amber-500',
		handleBorder: '!border-amber-500'
	},
	schedule: {
		iconBg: 'bg-blue-500/15',
		iconText: 'text-blue-500',
		accent: 'border-l-blue-500',
		handleBorder: '!border-blue-500'
	},
	flow: {
		iconBg: 'bg-yellow-500/15',
		iconText: 'text-yellow-500',
		accent: 'border-l-yellow-500',
		handleBorder: '!border-yellow-500'
	}
};

export function getCategoryStyle(category: string): CategoryStyle {
	return CATEGORY_STYLES[category] ?? CATEGORY_STYLES['system'];
}
