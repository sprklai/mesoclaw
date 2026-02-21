import { APP_IDENTITY } from "@/config/app-identity";

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
