/**
 * Types matching the Rust identity types in src-tauri/src/identity/types.rs.
 *
 * ## TODO: migrate to gateway REST API (Phase 3 - identity endpoints)
 * Currently served via Tauri IPC commands registered in lib.rs.
 */

export interface IdentityMeta {
  name: string;
  version: string;
  description: string;
}

export interface Identity {
  soul: string;
  user: string;
  agents: string;
  identity: IdentityMeta;
  tools: string;
  heartbeat: string;
  boot: string;
}

/** Metadata for listing identity files in the UI. */
export interface IdentityFileInfo {
  name: string;
  /** Canonical filename, e.g. "SOUL.md" */
  fileName: string;
  description: string;
}

/** All canonical identity file names. */
export const IDENTITY_FILE_NAMES = [
  "SOUL.md",
  "USER.md",
  "AGENTS.md",
  "IDENTITY.md",
  "TOOLS.md",
  "HEARTBEAT.md",
  "BOOT.md",
] as const;

export type IdentityFileName = (typeof IDENTITY_FILE_NAMES)[number];
