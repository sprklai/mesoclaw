import { sendNotification } from "@tauri-apps/plugin-notification";

import type { Settings } from "@/lib/tauri/settings/types";

import { ensureNotificationPermission } from "./permissions";

export interface NotificationOptions {
  title: string;
  body?: string;
  icon?: string;
  /** Optional category used to check per-category preferences. */
  category?: "heartbeat" | "cron_reminder" | "agent_complete" | "approval_request";
}

/**
 * Return true if the current local hour falls inside the DND window defined
 * by [dndStartHour, dndEndHour).  The window wraps midnight when start > end
 * (e.g. 22–7 means 22:00–06:59 is quiet time).
 */
function isDndActive(settings: Settings): boolean {
  const hour = new Date().getHours();
  const start = settings.dndStartHour;
  const end = settings.dndEndHour;
  if (start <= end) {
    return hour >= start && hour < end;
  }
  // Wraps midnight
  return hour >= start || hour < end;
}

/**
 * Return true if the given category is enabled in per-category preferences.
 * Defaults to true when no category is provided.
 */
function isCategoryEnabled(
  category: NotificationOptions["category"],
  settings: Settings
): boolean {
  if (!category) return true;
  const map: Record<NonNullable<NotificationOptions["category"]>, boolean> = {
    heartbeat: settings.notifyHeartbeat,
    cron_reminder: settings.notifyCronReminder,
    agent_complete: settings.notifyAgentComplete,
    approval_request: settings.notifyApprovalRequest,
  };
  return map[category] ?? true;
}

/**
 * Send a notification if enabled, not in DND window, category is active,
 * and OS permission is granted.
 * Returns true if the notification was sent, false otherwise.
 */
export async function notify(
  options: NotificationOptions,
  settings: Settings
): Promise<boolean> {
  // Check if notifications are enabled in settings
  if (!settings.enableNotifications) {
    return false;
  }

  // Check DND time-window
  if (isDndActive(settings)) {
    return false;
  }

  // Check per-category preference
  if (!isCategoryEnabled(options.category, settings)) {
    return false;
  }

  // Ensure we have permission
  const granted = await ensureNotificationPermission();
  if (granted !== "granted") {
    return false;
  }

  // Send the notification
  sendNotification({
    title: options.title,
    body: options.body,
    icon: options.icon,
  });

  return true;
}

/**
 * Send a notification without checking settings (for critical alerts)
 * Still requires permission
 */
export async function notifyForced(
  options: NotificationOptions
): Promise<boolean> {
  const granted = await ensureNotificationPermission();
  if (granted !== "granted") {
    return false;
  }

  sendNotification({
    title: options.title,
    body: options.body,
    icon: options.icon,
  });

  return true;
}
