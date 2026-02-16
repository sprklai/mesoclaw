/**
 * Online status utility.
 *
 * This module provides a consistent way to check online status across the application.
 * It combines browser online status with backend connection status from the workspace store.
 *
 * This is defined in lib/ rather than hooks/ to avoid circular dependencies when used
 * in Zustand stores (stores cannot import React hooks).
 */

// Workspace store removed - implement connection status check as needed
// import { useWorkspaceStore } from "@/stores/workspace-store";

export interface OnlineStatus {
  /** Combined online status (browser online AND backend connected) */
  isOnline: boolean;
  /** Browser's navigator.onLine status */
  isBrowserOnline: boolean;
  /** Backend connection status (connected to database) */
  isBackendOnline: boolean;
  /** Detailed connection status */
  connectionStatus: "connected" | "disconnected" | "connecting";
}

/**
 * Get the current online status.
 *
 * Returns browser online status only.
 * Safe to call from Zustand stores (doesn't use React hooks).
 *
 * @returns Online status object with browser connection information
 */
export function getOnlineStatus(): OnlineStatus {
  const isBrowserOnline = typeof navigator !== "undefined" && navigator.onLine;

  return {
    isOnline: isBrowserOnline,
    isBrowserOnline,
    isBackendOnline: true, // No backend connection tracking in boilerplate
    connectionStatus: "connected",
  };
}
