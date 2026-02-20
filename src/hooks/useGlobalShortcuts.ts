import { useNavigate } from "@tanstack/react-router";
import { useHotkeys } from "react-hotkeys-hook";
import { useCallback } from "react";

import { useChatSessionStore } from "@/stores/chatSessionStore";

export interface GlobalShortcutOptions {
  /** Whether shortcuts are enabled (default: true) */
  enabled?: boolean;
  /** Called when a shortcut is triggered (for debugging/logging) */
  onShortcut?: (shortcut: string, action: string) => void;
}

/**
 * Global keyboard shortcuts hook.
 * Provides application-wide keyboard shortcuts for navigation and actions.
 *
 * Shortcuts:
 * - Cmd/Ctrl + K: Open command palette (handled by CommandPalette component)
 * - Cmd/Ctrl + N: New chat session
 * - Cmd/Ctrl + Shift + C: Clear current chat
 * - G then C: Go to Chat
 * - G then A: Go to Agents
 * - G then S: Go to Settings
 * - G then H: Go to Channels (Home)
 * - G then M: Go to Memory
 * - G then L: Go to Logs
 */
export function useGlobalShortcuts(options: GlobalShortcutOptions = {}) {
  const { enabled = true, onShortcut } = options;
  const navigate = useNavigate();
  const createSession = useChatSessionStore((s) => s.createSession);
  const clearMessages = useChatSessionStore((s) => s.clearMessages);

  const handleShortcut = useCallback(
    (shortcut: string, action: () => void) => {
      if (onShortcut) {
        onShortcut(shortcut, action.name || "anonymous");
      }
      action();
    },
    [onShortcut],
  );

  // New chat session: Cmd/Ctrl + N
  useHotkeys(
    "mod+n",
    () => {
      handleShortcut("mod+n", async () => {
        await createSession("default", "default");
        navigate({ to: "/chat" });
      });
    },
    { enabled, preventDefault: true },
    [createSession, navigate, handleShortcut],
  );

  // Clear current chat: Cmd/Ctrl + Shift + C
  useHotkeys(
    "mod+shift+c",
    () => {
      handleShortcut("mod+shift+c", () => {
        clearMessages();
      });
    },
    { enabled, preventDefault: true },
    [clearMessages, handleShortcut],
  );

  // Navigation shortcuts using "G" prefix (like Gmail)
  // These use the "sequence" pattern: press G, then press another key

  // Go to Chat: G then C
  useHotkeys(
    "g+c",
    () => {
      handleShortcut("g+c", () => {
        navigate({ to: "/chat" });
      });
    },
    { enabled, preventDefault: true },
    [navigate, handleShortcut],
  );

  // Go to Agents: G then A
  useHotkeys(
    "g+a",
    () => {
      handleShortcut("g+a", () => {
        navigate({ to: "/agents" });
      });
    },
    { enabled, preventDefault: true },
    [navigate, handleShortcut],
  );

  // Go to Settings: G then S
  useHotkeys(
    "g+s",
    () => {
      handleShortcut("g+s", () => {
        navigate({ to: "/settings", search: { tab: "ai" } });
      });
    },
    { enabled, preventDefault: true },
    [navigate, handleShortcut],
  );

  // Go to Channels (Home): G then H
  useHotkeys(
    "g+h",
    () => {
      handleShortcut("g+h", () => {
        navigate({ to: "/channels" });
      });
    },
    { enabled, preventDefault: true },
    [navigate, handleShortcut],
  );

  // Go to Memory: G then M
  useHotkeys(
    "g+m",
    () => {
      handleShortcut("g+m", () => {
        navigate({ to: "/memory" });
      });
    },
    { enabled, preventDefault: true },
    [navigate, handleShortcut],
  );

  // Go to Logs: G then L
  useHotkeys(
    "g+l",
    () => {
      handleShortcut("g+l", () => {
        navigate({ to: "/logs" });
      });
    },
    { enabled, preventDefault: true },
    [navigate, handleShortcut],
  );
}

/**
 * List of all available global shortcuts for documentation/UI display.
 */
export const GLOBAL_SHORTCUTS = [
  { keys: "Cmd/Ctrl + K", description: "Open command palette" },
  { keys: "Cmd/Ctrl + N", description: "New chat session" },
  { keys: "Cmd/Ctrl + Shift + C", description: "Clear current chat" },
  { keys: "G → C", description: "Go to Chat" },
  { keys: "G → A", description: "Go to Agents" },
  { keys: "G → S", description: "Go to Settings" },
  { keys: "G → H", description: "Go to Channels" },
  { keys: "G → M", description: "Go to Memory" },
  { keys: "G → L", description: "Go to Logs" },
] as const;
