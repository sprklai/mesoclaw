import { sendNotification } from "@tauri-apps/plugin-notification";

import type { Settings } from "@/lib/tauri/settings/types";

import { ensureNotificationPermission } from "./permissions";

export interface NotificationOptions {
  title: string;
  body?: string;
  icon?: string;
}

/**
 * Send a notification if enabled and permission is granted
 * Returns true if the notification was sent, false otherwise
 */
export async function notify(
  options: NotificationOptions,
  settings: Settings
): Promise<boolean> {
  // Check if notifications are enabled in settings
  if (!settings.enableNotifications) {
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
