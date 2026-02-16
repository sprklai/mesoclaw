import {
  isPermissionGranted,
  requestPermission,
} from "@tauri-apps/plugin-notification";

/**
 * Ensure notification permission is granted
 * Requests permission if not already granted
 */
export async function ensureNotificationPermission(): Promise<NotificationPermission> {
  const granted = await isPermissionGranted();
  if (granted) {
    return "denied";
  }
  return await requestPermission();
}
