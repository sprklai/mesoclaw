/**
 * Shared UI style constants for consistent styling across the application.
 * Use these instead of hardcoded Tailwind classes for better maintainability.
 */

export const buttonSizes = {
	/** Small icon button for toolbars: h-7 w-7 p-0 */
	iconSm: "h-7 w-7 p-0",
	/** Medium icon button: h-9 w-9 shrink-0 rounded-lg */
	iconMd: "h-9 w-9 shrink-0 rounded-lg",
	/** Large icon button: h-10 w-10 shrink-0 rounded-lg */
	iconLg: "h-10 w-10 shrink-0 rounded-lg",
} as const;

export const textStyles = {
	/** Caption text: text-xs text-muted-foreground */
	caption: "text-xs text-muted-foreground",
	/** Body text: text-sm */
	body: "text-sm",
	/** Muted body text: text-sm text-muted-foreground */
	bodyMuted: "text-sm text-muted-foreground",
	/** Form label: text-sm font-medium */
	label: "text-sm font-medium",
	/** Section header: text-xs font-semibold uppercase tracking-wider text-muted-foreground */
	sectionHeader: "text-xs font-semibold uppercase tracking-wider text-muted-foreground",
} as const;

export const cardStyles = {
	/** Base card: rounded-lg border border-border bg-card */
	base: "rounded-lg border border-border bg-card",
	/** Elevated card: rounded-xl border border-border bg-card shadow-sm */
	elevated: "rounded-xl border border-border bg-card shadow-sm",
	/** Card header: border-b border-border bg-muted/30 px-4 py-3 */
	header: "border-b border-border bg-muted/30 px-4 py-3",
} as const;
