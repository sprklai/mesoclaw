import { APP_IDENTITY } from "@/config/app-identity";
import { useAppSettingsStore } from "@/stores/appSettingsStore";

const DEFAULT_PRODUCT_NAME = APP_IDENTITY.productName;

/**
 * Get the display name for the app, using a custom name if provided.
 * @param customDisplayName - Optional custom display name (falls back to default)
 * @returns The product name to display
 */
export function getAppDisplayName(customDisplayName?: string | null): string {
	return customDisplayName?.trim() || DEFAULT_PRODUCT_NAME;
}

/**
 * Get the app identity with a potentially customized product name.
 * Note: This returns a new object each time, as APP_IDENTITY is const.
 * @param customDisplayName - Optional custom display name (falls back to default)
 * @returns The app identity with the display name
 */
export function getAppIdentityWithDisplayName(
	customDisplayName?: string | null,
) {
	return {
		...APP_IDENTITY,
		productName: getAppDisplayName(customDisplayName),
	};
}

/**
 * Get the current app identity from the store (for non-hook contexts).
 * Uses the currently stored appDisplayName from the settings store.
 */
export function getCurrentAppIdentity() {
	const { appDisplayName } = useAppSettingsStore.getState();
	return getAppIdentityWithDisplayName(appDisplayName);
}

/**
 * Format the app name for use in agent system prompts.
 * Returns the display name with proper capitalization.
 */
export function getAppNameForAgent(): string {
	return getCurrentAppIdentity().productName;
}

/**
 * Get a greeting string using the current app display name.
 * @param userName - Optional user name to include in greeting
 */
export function getAppGreeting(userName?: string | null): string {
	const appName = getAppNameForAgent();
	if (userName?.trim()) {
		return `Hello ${userName.trim()}! I'm ${appName}.`;
	}
	return `Hello! I'm ${appName}.`;
}
