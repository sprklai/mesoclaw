/**
 * Tauri desktop integration.
 * Detects Tauri environment and provides typed wrappers for IPC commands.
 */

/** True when running inside a Tauri webview, false in browser. */
export const isTauri: boolean =
  typeof window !== "undefined" &&
  (window.location.protocol === "tauri:" ||
    "__TAURI_INTERNALS__" in window ||
    "__TAURI__" in window);

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

/** Boot status reported by the embedded gateway. */
export type BootStatus =
  | { status: "Booting" }
  | { status: "Ready" }
  | { status: "Failed"; message: string };

/** Query the current boot status of the embedded gateway. */
export async function getBootStatus(): Promise<BootStatus | null> {
  if (!isTauri) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<BootStatus>("get_boot_status");
}

/** Listen for the gateway-ready event. Returns an unlisten function. */
export async function onGatewayReady(
  callback: () => void,
): Promise<(() => void) | null> {
  if (!isTauri) return null;
  const { listen } = await import("@tauri-apps/api/event");
  return listen("gateway-ready", callback);
}

/** Listen for the gateway-failed event. Returns an unlisten function. */
export async function onGatewayFailed(
  callback: (message: string) => void,
): Promise<(() => void) | null> {
  if (!isTauri) return null;
  const { listen } = await import("@tauri-apps/api/event");
  return listen<string>("gateway-failed", (event) => callback(event.payload));
}

/** Update info returned by the check_for_update command. */
export interface UpdateInfo {
  version: string;
  body: string | null;
}

/** Check for available updates. Returns update info or null if up to date. */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  if (!isTauri) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<UpdateInfo | null>("check_for_update");
}

/** Listen for the update-available event (emitted by background check). */
export async function onUpdateAvailable(
  callback: (info: UpdateInfo) => void,
): Promise<(() => void) | null> {
  if (!isTauri) return null;
  const { listen } = await import("@tauri-apps/api/event");
  return listen<UpdateInfo>("update-available", (event) =>
    callback(event.payload),
  );
}

/** Download and install an update, then relaunch the app. */
export async function installUpdate(
  onProgress?: (percent: number) => void,
): Promise<void> {
  if (!isTauri) return;
  const { check } = await import("@tauri-apps/plugin-updater");
  const { relaunch } = await import("@tauri-apps/plugin-process");

  const update = await check();
  if (!update) return;

  let totalLength = 0;
  let downloaded = 0;

  await update.downloadAndInstall((event) => {
    if (event.event === "Started" && event.data.contentLength) {
      totalLength = event.data.contentLength;
    } else if (event.event === "Progress") {
      downloaded += event.data.chunkLength;
      if (totalLength > 0 && onProgress) {
        onProgress(Math.round((downloaded / totalLength) * 100));
      }
    }
  });

  await relaunch();
}
