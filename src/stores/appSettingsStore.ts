import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";
import { persist } from "zustand/middleware";

import { APP_IDENTITY } from "@/config/app-identity";

export interface AppSettings {
	// Appearance
	theme: "light" | "dark" | "system";
	sidebarExpanded: boolean;

	// Behavior
	systemTray: boolean;
	launchAtLogin: boolean;

	// Notifications
	notificationsEnabled: boolean;

	// Privacy & Data
	cloudLlmEnabled: boolean;
	explanationVerbosity: "concise" | "standard" | "detailed";

	// User Identity
	userName: string | null;
	appDisplayName: string | null;

	// Onboarding
	onboardingCompleted: boolean;
}

/**
 * App identity with custom display name applied.
 * This is the merged identity used throughout the app.
 */
export interface AppIdentity {
	productName: string;
	slug: string;
	reverseDomain: string;
	bundleIdentifier: string;
	iconAssetPath: string;
	keychainService: string;
	credentialsService: string;
	strongholdClientName: string;
	strongholdVaultPassword: string;
	openRouterHttpReferer: string;
	openRouterTitle: string;
	trayTooltip: string;
	skillsConfigDirName: string;
}

interface AppSettingsActions {
	setTheme: (theme: AppSettings["theme"]) => void;
	setSidebarExpanded: (expanded: boolean) => void;
	setSystemTray: (enabled: boolean) => void;
	setLaunchAtLogin: (enabled: boolean) => void;
	setNotificationsEnabled: (enabled: boolean) => void;
	setCloudLlmEnabled: (enabled: boolean) => void;
	setExplanationVerbosity: (
		verbosity: AppSettings["explanationVerbosity"],
	) => void;
	loadUserIdentity: () => Promise<void>;
	setUserIdentity: (
		userName: string | null,
		appDisplayName: string | null,
	) => Promise<void>;
	completeOnboarding: () => void;
	resetSettings: () => void;
	/** Get the current app identity with custom display name applied */
	getAppIdentity: () => AppIdentity;
}

type AppSettingsStore = AppSettings & AppSettingsActions;

const DEFAULT_SETTINGS: AppSettings = {
	theme: "system",
	sidebarExpanded: true,
	systemTray: true,
	launchAtLogin: false,
	notificationsEnabled: true,
	cloudLlmEnabled: true,
	explanationVerbosity: "standard",
	userName: null,
	appDisplayName: null,
	onboardingCompleted: false,
};

export const useAppSettingsStore = create<AppSettingsStore>()(
	persist(
		(set, get) => ({
			...DEFAULT_SETTINGS,

			setTheme: (theme) => set({ theme }),
			setSidebarExpanded: (sidebarExpanded) => set({ sidebarExpanded }),
			setSystemTray: (systemTray) => set({ systemTray }),
			setLaunchAtLogin: (launchAtLogin) => set({ launchAtLogin }),
			setNotificationsEnabled: (notificationsEnabled) =>
				set({ notificationsEnabled }),
			setCloudLlmEnabled: (cloudLlmEnabled) => set({ cloudLlmEnabled }),
			setExplanationVerbosity: (explanationVerbosity) =>
				set({ explanationVerbosity }),

			loadUserIdentity: async () => {
				try {
					const identity = await invoke<{
						userName: string | null;
						appDisplayName: string | null;
					}>("get_user_identity_command");
					set({
						userName: identity.userName,
						appDisplayName: identity.appDisplayName,
					});
				} catch (error) {
					console.error("Failed to load user identity:", error);
				}
			},

			setUserIdentity: async (userName, appDisplayName) => {
				try {
					await invoke("set_user_identity_command", {
						userName,
						appDisplayName,
					});
					set({ userName, appDisplayName });
				} catch (error) {
					console.error("Failed to set user identity:", error);
					throw error;
				}
			},

			completeOnboarding: () => set({ onboardingCompleted: true }),

			resetSettings: () => set(DEFAULT_SETTINGS),

			getAppIdentity: () => {
				const { appDisplayName } = get();
				const productName = appDisplayName?.trim() || APP_IDENTITY.productName;
				return {
					...APP_IDENTITY,
					productName,
				};
			},
		}),
		{
			name: "app-settings-storage",
		},
	),
);

/**
 * Hook to get the current app identity with custom display name.
 * Use this throughout the app for consistent identity access.
 */
export function useAppIdentity(): AppIdentity {
	const getAppIdentity = useAppSettingsStore((state) => state.getAppIdentity);
	return getAppIdentity();
}

/**
 * Get the current display name synchronously (for non-hook contexts).
 */
export function getAppDisplayName(): string {
	const { appDisplayName } = useAppSettingsStore.getState();
	return appDisplayName?.trim() || APP_IDENTITY.productName;
}
