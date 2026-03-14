/**
 * Tauri desktop integration.
 * Detects Tauri environment and provides typed wrappers for IPC commands.
 */

/** True when running inside a Tauri webview, false in browser. */
export const isTauri: boolean =
  typeof window !== "undefined" &&
  ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

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

/** Open the Zenii data directory in the OS file manager. */
export async function openDataDir(): Promise<void> {
  if (!isTauri) return;
  const { invoke } = await import("@tauri-apps/api/core");
  await invoke("open_data_dir");
}

/** Show a native OS desktop notification. No-ops outside Tauri. */
export async function showNotification(
  title: string,
  body: string,
): Promise<void> {
  if (!isTauri) return;
  try {
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("show_notification", { title, body });
  } catch (e) {
    console.warn("Native notification failed:", e);
  }
}

/** Open a URL in the native browser via Tauri opener plugin. Falls back to window.open. */
export async function openInBrowser(url: string): Promise<void> {
  if (isTauri) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  } else {
    window.open(url, "_blank");
  }
}

/** Open the config file in the default editor, creating a backup first. Returns backup path or null. */
export async function openConfigFile(): Promise<string | null> {
  if (!isTauri) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string>("open_config_file");
}
