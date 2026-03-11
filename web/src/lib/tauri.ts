/**
 * Tauri desktop integration.
 * Detects Tauri environment and provides typed wrappers for IPC commands.
 */

/** True when running inside a Tauri webview, false in browser. */
export const isTauri: boolean =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

/** Hide the window to the system tray. */
export async function closeToTray(): Promise<void> {
  if (!isTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("close_to_tray");
}

/** Show and focus the main window. */
export async function showWindow(): Promise<void> {
  if (!isTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("show_window");
}

/** Get the desktop app version string. */
export async function getAppVersion(): Promise<string | null> {
  if (!isTauri) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("get_app_version");
}

/** Open the MesoClaw data directory in the OS file manager. */
export async function openDataDir(): Promise<void> {
  if (!isTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_data_dir");
}
