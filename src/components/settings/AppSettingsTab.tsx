import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { SettingRow } from "@/components/setting-row";
import { SettingsSection } from "@/components/settings-section";
import { Select } from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
	LANGUAGE_OPTIONS,
	LOG_LEVEL_OPTIONS,
	THEME_OPTIONS,
} from "@/constants/settings";
import type { LogLevel, Settings, Theme } from "@/lib/tauri/settings/types";

interface AppSettingsTabProps {
	theme: Theme;
	settings: Settings;
	isSaving: boolean;
	onUpdateSetting: <K extends keyof Settings>(
		key: K,
		value: Settings[K],
	) => void;
	onAutostartChange: (enabled: boolean) => void;
	onTrayVisibilityChange: (visible: boolean) => void;
	onNotificationChange: (enabled: boolean) => void;
	autostartEnabled: boolean;
}

export function AppSettingsTab({
	theme,
	settings,
	isSaving,
	onUpdateSetting,
	onAutostartChange,
	onTrayVisibilityChange,
	onNotificationChange,
	autostartEnabled,
}: AppSettingsTabProps) {
	const { t, i18n } = useTranslation(["settings", "common"]);

	const handleLanguageChange = async (newLanguage: string) => {
		await i18n.changeLanguage(newLanguage);

		// Persist to Tauri settings (backup storage)
		try {
			await invoke("update_setting_command", {
				key: "language",
				value: newLanguage,
			});
		} catch (error) {
			console.error("Failed to persist language to Tauri settings:", error);
		}
	};

	return (
		<div className="space-y-6">
			{/* Appearance Section */}
			<SettingsSection
				title={t("settings:appearance.title")}
				description={t("settings:appearance.description")}
			>
				<SettingRow
					label={t("settings:appearance.theme.label")}
					description={t("settings:appearance.theme.description")}
				>
					<Select
						value={theme}
						onValueChange={(value) => onUpdateSetting("theme", value)}
						options={THEME_OPTIONS}
						disabled={isSaving}
						className="w-full sm:w-40"
					/>
				</SettingRow>

				<SettingRow
					label={t("settings:appearance.language.label")}
					description={t("settings:appearance.language.description")}
				>
					<Select
						value={i18n.language}
						onValueChange={handleLanguageChange}
						options={[...LANGUAGE_OPTIONS]}
						disabled={isSaving}
						className="w-full sm:w-40"
					/>
				</SettingRow>

				<SettingRow
					label={t("settings:appearance.sidebarExpanded.label")}
					description={t("settings:appearance.sidebarExpanded.description")}
					htmlFor="sidebar-expanded"
				>
					<Switch
						id="sidebar-expanded"
						checked={settings.sidebarExpanded}
						onCheckedChange={(checked) =>
							onUpdateSetting("sidebarExpanded", checked)
						}
						disabled={isSaving}
					/>
				</SettingRow>
			</SettingsSection>

			{/* Behavior Section */}
			<SettingsSection
				title={t("settings:behavior.title")}
				description={t("settings:behavior.description")}
			>
				<SettingRow
					label={t("settings:behavior.showInTray.label")}
					description={t("settings:behavior.showInTray.description")}
					htmlFor="show-in-tray"
				>
					<Switch
						id="show-in-tray"
						checked={settings.showInTray}
						onCheckedChange={onTrayVisibilityChange}
						disabled={isSaving}
					/>
				</SettingRow>

				<SettingRow
					label={t("settings:behavior.launchAtLogin.label")}
					description={t("settings:behavior.launchAtLogin.description")}
					htmlFor="launch-at-login"
				>
					<Switch
						id="launch-at-login"
						checked={autostartEnabled}
						onCheckedChange={onAutostartChange}
						disabled={isSaving}
					/>
				</SettingRow>
			</SettingsSection>

			{/* Notifications Section */}
			<SettingsSection
				title={t("settings:notifications.title")}
				description={t("settings:notifications.description")}
			>
				<SettingRow
					label={t("settings:notifications.enable.label")}
					description={t("settings:notifications.enable.description")}
					htmlFor="enable-notifications"
				>
					<Switch
						id="enable-notifications"
						checked={settings.enableNotifications}
						onCheckedChange={onNotificationChange}
						disabled={isSaving}
					/>
				</SettingRow>

				{/* DND schedule */}
				<SettingRow
					label={t(
						"settings:notifications.dndScheduleEnabled.label",
						"Enable DND schedule",
					)}
					description={t(
						"settings:notifications.dndScheduleEnabled.description",
						"Automatically suppress notifications during the hours below.",
					)}
					htmlFor="dnd-schedule-enabled"
				>
					<Switch
						id="dnd-schedule-enabled"
						checked={settings.dndScheduleEnabled}
						onCheckedChange={(checked) =>
							onUpdateSetting("dndScheduleEnabled", checked)
						}
						disabled={isSaving || !settings.enableNotifications}
					/>
				</SettingRow>

				<SettingRow
					label={t(
						"settings:notifications.dndStartHour.label",
						"DND start hour",
					)}
					description={t(
						"settings:notifications.dndStartHour.description",
						"Hour (0–23) when Do Not Disturb begins. Default: 22 (10 pm).",
					)}
					htmlFor="dnd-start-hour"
				>
					<input
						id="dnd-start-hour"
						type="number"
						min={0}
						max={23}
						value={settings.dndStartHour}
						onChange={(e) =>
							onUpdateSetting("dndStartHour", Number(e.target.value))
						}
						disabled={isSaving || !settings.enableNotifications}
						className="w-20 rounded-md border border-input bg-background px-3 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
					/>
				</SettingRow>

				<SettingRow
					label={t("settings:notifications.dndEndHour.label", "DND end hour")}
					description={t(
						"settings:notifications.dndEndHour.description",
						"Hour (0–23) when Do Not Disturb ends. Default: 7 (7 am).",
					)}
					htmlFor="dnd-end-hour"
				>
					<input
						id="dnd-end-hour"
						type="number"
						min={0}
						max={23}
						value={settings.dndEndHour}
						onChange={(e) =>
							onUpdateSetting("dndEndHour", Number(e.target.value))
						}
						disabled={isSaving || !settings.enableNotifications}
						className="w-20 rounded-md border border-input bg-background px-3 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-ring disabled:opacity-50"
					/>
				</SettingRow>

				{/* Per-category toggles */}
				<SettingRow
					label={t("settings:notifications.notifyHeartbeat.label", "Heartbeat")}
					description={t(
						"settings:notifications.notifyHeartbeat.description",
						"Show a notification on each heartbeat tick.",
					)}
					htmlFor="notify-heartbeat"
				>
					<Switch
						id="notify-heartbeat"
						checked={settings.notifyHeartbeat}
						onCheckedChange={(checked) =>
							onUpdateSetting("notifyHeartbeat", checked)
						}
						disabled={isSaving || !settings.enableNotifications}
					/>
				</SettingRow>

				<SettingRow
					label={t(
						"settings:notifications.notifyCronReminder.label",
						"Cron reminders",
					)}
					description={t(
						"settings:notifications.notifyCronReminder.description",
						"Show a notification when a scheduled job fires.",
					)}
					htmlFor="notify-cron-reminder"
				>
					<Switch
						id="notify-cron-reminder"
						checked={settings.notifyCronReminder}
						onCheckedChange={(checked) =>
							onUpdateSetting("notifyCronReminder", checked)
						}
						disabled={isSaving || !settings.enableNotifications}
					/>
				</SettingRow>

				<SettingRow
					label={t(
						"settings:notifications.notifyAgentComplete.label",
						"Agent complete",
					)}
					description={t(
						"settings:notifications.notifyAgentComplete.description",
						"Show a notification when an agent task finishes.",
					)}
					htmlFor="notify-agent-complete"
				>
					<Switch
						id="notify-agent-complete"
						checked={settings.notifyAgentComplete}
						onCheckedChange={(checked) =>
							onUpdateSetting("notifyAgentComplete", checked)
						}
						disabled={isSaving || !settings.enableNotifications}
					/>
				</SettingRow>

				<SettingRow
					label={t(
						"settings:notifications.notifyApprovalRequest.label",
						"Approval requests",
					)}
					description={t(
						"settings:notifications.notifyApprovalRequest.description",
						"Show a notification when an action requires your approval.",
					)}
					htmlFor="notify-approval-request"
				>
					<Switch
						id="notify-approval-request"
						checked={settings.notifyApprovalRequest}
						onCheckedChange={(checked) =>
							onUpdateSetting("notifyApprovalRequest", checked)
						}
						disabled={isSaving || !settings.enableNotifications}
					/>
				</SettingRow>
			</SettingsSection>

			{/* Developer Section */}
			<SettingsSection
				title={t("settings:developer.title")}
				description={t("settings:developer.description")}
			>
				<SettingRow
					label={t("settings:developer.enableLogging.label")}
					description={t("settings:developer.enableLogging.description")}
					htmlFor="enable-logging"
				>
					<Switch
						id="enable-logging"
						checked={settings.enableLogging}
						onCheckedChange={(checked) =>
							onUpdateSetting("enableLogging", checked)
						}
						disabled={isSaving}
					/>
				</SettingRow>

				<SettingRow
					label={t("settings:developer.logLevel.label")}
					description={t("settings:developer.logLevel.description")}
				>
					<Select<LogLevel>
						value={settings.logLevel}
						onValueChange={(value) => onUpdateSetting("logLevel", value)}
						options={LOG_LEVEL_OPTIONS}
						disabled={isSaving || !settings.enableLogging}
						className="w-full sm:w-40"
					/>
				</SettingRow>
			</SettingsSection>
		</div>
	);
}
