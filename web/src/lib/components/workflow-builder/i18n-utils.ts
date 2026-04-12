import * as m from "$lib/paraglide/messages";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const msgs = m as any as Record<string, (...args: unknown[]) => string>;

/**
 * Resolve an i18n key from the workflow builder namespace.
 * Falls back to the raw key string if not found.
 */
export function t(key: string): string {
  return msgs[key]?.() ?? key;
}
