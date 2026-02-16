import { invoke } from "@tauri-apps/api/core";

import type { Settings, SettingsUpdate } from "./types";

export async function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_app_settings");
}

export async function updateSettings(
  update: SettingsUpdate
): Promise<Settings> {
  return invoke<Settings>("update_app_settings", { update });
}

export async function setTrayVisible(visible: boolean): Promise<void> {
  return invoke("set_tray_visible", { visible });
}
