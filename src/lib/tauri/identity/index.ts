/**
 * Identity system CRUD operations.
 *
 * Migrated from Tauri IPC to gateway REST API (Phase 3).
 * Uses GatewayClient to call /api/v1/identity/* endpoints.
 *
 * `getSystemPrompt` remains on Tauri IPC because the gateway does not
 * expose a dedicated system-prompt assembly endpoint yet.
 */

import { invoke } from "@tauri-apps/api/core";

import {
  type IdentityFileInfo as GwIdentityFileInfo,
  getGatewayClient,
} from "@/lib/gateway-client";

import type { IdentityFileInfo } from "./types";

/**
 * List all identity files with their metadata.
 */
export async function listIdentityFiles(): Promise<IdentityFileInfo[]> {
  const client = getGatewayClient();
  if (!client) {
    throw new Error("Gateway client not initialised");
  }
  const res = await client.listIdentityFiles();
  // Map gateway response to the existing IdentityFileInfo shape.
  return res.files.map((f: GwIdentityFileInfo) => ({
    name: f.name,
    fileName: f.fileName,
    description: f.description,
  }));
}

/**
 * Get the raw content of an identity file.
 *
 * @param fileName - Canonical file name, e.g. "SOUL.md"
 */
export async function getIdentityFile(fileName: string): Promise<string> {
  const client = getGatewayClient();
  if (!client) {
    throw new Error("Gateway client not initialised");
  }
  const res = await client.getIdentityFile(fileName);
  return res.content;
}

/**
 * Update the content of an identity file.
 *
 * @param fileName - Canonical file name, e.g. "SOUL.md"
 * @param content  - New file content (Markdown)
 */
export async function updateIdentityFile(
  fileName: string,
  content: string,
): Promise<void> {
  const client = getGatewayClient();
  if (!client) {
    throw new Error("Gateway client not initialised");
  }
  await client.updateIdentityFile(fileName, content);
}

/**
 * Build and return the assembled system prompt from all identity files.
 * Assembly order: SOUL → AGENTS → USER → TOOLS
 *
 * NOTE: Remains on Tauri IPC — no gateway endpoint for prompt assembly yet.
 */
export async function getSystemPrompt(): Promise<string> {
  return invoke<string>("get_system_prompt_command");
}

export type { IdentityFileInfo, IdentityFileName } from "./types";
export { IDENTITY_FILE_NAMES } from "./types";
