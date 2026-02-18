/**
 * Tauri command wrappers for the identity system.
 *
 * ## TODO: migrate to gateway REST API (Phase 3 - identity CRUD endpoints)
 * Currently served via Tauri IPC. When the gateway /api/v1/identity/* routes
 * are implemented, replace `invoke()` calls with `GatewayClient` HTTP calls.
 */

import { invoke } from "@tauri-apps/api/core";

import type { IdentityFileInfo } from "./types";

/**
 * List all identity files with their metadata.
 */
export async function listIdentityFiles(): Promise<IdentityFileInfo[]> {
  return invoke<IdentityFileInfo[]>("list_identity_files_command");
}

/**
 * Get the raw content of an identity file.
 *
 * @param fileName - Canonical file name, e.g. "SOUL.md"
 */
export async function getIdentityFile(fileName: string): Promise<string> {
  return invoke<string>("get_identity_file_command", { fileName });
}

/**
 * Update the content of an identity file.
 *
 * @param fileName - Canonical file name, e.g. "SOUL.md"
 * @param content  - New file content (Markdown)
 */
export async function updateIdentityFile(
  fileName: string,
  content: string
): Promise<void> {
  return invoke("update_identity_file_command", { fileName, content });
}

/**
 * Build and return the assembled system prompt from all identity files.
 * Assembly order: SOUL → AGENTS → USER → TOOLS
 */
export async function getSystemPrompt(): Promise<string> {
  return invoke<string>("get_system_prompt_command");
}

export type { IdentityFileInfo, IdentityFileName } from "./types";
export { IDENTITY_FILE_NAMES } from "./types";
