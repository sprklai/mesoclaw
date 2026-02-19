/**
 * Zustand store for the Channel Management UI (Phase 7.2).
 *
 * Tracks the lifecycle status, configuration, and message counts for each
 * registered channel (Telegram, webhooks, etc.).
 *
 * Channel IPC commands are stubs; they will be wired up once the backend
 * Tauri commands are exposed in Phase 7 follow-up work.
 */

import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

// ─── Types ────────────────────────────────────────────────────────────────────

export type ChannelStatus =
  | "connected"
  | "disconnected"
  | "reconnecting"
  | "error";

/** Telegram-specific connection configuration. */
export interface TelegramChannelConfig {
  /** Bot token obtained from BotFather. */
  token: string;
  /** Comma-separated string of allowed Telegram chat IDs. */
  allowedChatIds: string;
  /** Long-polling timeout in seconds (default: 30). */
  pollingTimeoutSecs: number;
}

/** Discriminated union for per-channel configuration. */
export type ChannelConfig =
  | { type: "telegram"; telegram: TelegramChannelConfig }
  | { type: "webhook" }
  | { type: "tauri-ipc" };

/** Live status and metadata for a registered channel. */
export interface ChannelEntry {
  /** Unique channel identifier (e.g. "telegram", "tauri-ipc"). */
  name: string;
  /** Human-readable display name. */
  displayName: string;
  /** Current connection status. */
  status: ChannelStatus;
  /** Total inbound + outbound message count since last connect. */
  messageCount: number;
  /** Most recent error message, if any. */
  lastError: string | null;
  /** Per-channel configuration. */
  config: ChannelConfig;
}

// ─── Default channels ─────────────────────────────────────────────────────────

const DEFAULT_CHANNELS: ChannelEntry[] = [
  {
    name: "tauri-ipc",
    displayName: "Desktop IPC",
    status: "connected",
    messageCount: 0,
    lastError: null,
    config: { type: "tauri-ipc" },
  },
  {
    name: "telegram",
    displayName: "Telegram",
    status: "disconnected",
    messageCount: 0,
    lastError: null,
    config: {
      type: "telegram",
      telegram: {
        token: "",
        allowedChatIds: "",
        pollingTimeoutSecs: 30,
      },
    },
  },
];

// ─── Store ────────────────────────────────────────────────────────────────────

interface ChannelStore {
  channels: ChannelEntry[];
  selectedChannel: string | null;
  isLoading: boolean;
  error: string | null;

  /** Load channel statuses from the backend. */
  loadChannels: () => Promise<void>;
  /** Attempt to connect the named channel. */
  connectChannel: (name: string) => Promise<void>;
  /** Disconnect the named channel. */
  disconnectChannel: (name: string) => Promise<void>;
  /** Test connectivity without fully connecting. */
  testConnection: (name: string) => Promise<boolean>;
  /** Update the Telegram configuration and persist it. */
  updateTelegramConfig: (config: TelegramChannelConfig) => Promise<void>;
  /** Select a channel to show its config panel. */
  selectChannel: (name: string | null) => void;
  /** Internal: update the status of a named channel. */
  setChannelStatus: (name: string, status: ChannelStatus, error?: string | null) => void;
}

export const useChannelStore = create<ChannelStore>((set, get) => ({
  channels: DEFAULT_CHANNELS,
  selectedChannel: null,
  isLoading: false,
  error: null,

  loadChannels: async () => {
    set({ isLoading: true, error: null });
    try {
      const entries = await invoke<Array<{ name: string; connected: boolean; error: string | null }>>(
        "list_channels_command",
      );
      set((state) => ({
        channels: state.channels.map((ch) => {
          const entry = entries.find((e) => e.name === ch.name);
          if (!entry) return ch;
          return {
            ...ch,
            status: (entry.connected ? "connected" : "disconnected") as ChannelStatus,
            lastError: entry.error,
          };
        }),
        isLoading: false,
      }));
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  connectChannel: async (name) => {
    get().setChannelStatus(name, "reconnecting");
    try {
      // ## TODO: implement backend command connect_channel_command
      await invoke("connect_channel_command", { name });
      get().setChannelStatus(name, "connected");
    } catch (err) {
      get().setChannelStatus(name, "error", String(err));
    }
  },

  disconnectChannel: async (name) => {
    try {
      // ## TODO: implement backend command disconnect_channel_command
      await invoke("disconnect_channel_command", { name });
      get().setChannelStatus(name, "disconnected");
    } catch (err) {
      get().setChannelStatus(name, "error", String(err));
    }
  },

  testConnection: async (name) => {
    try {
      // ## TODO: implement backend command test_channel_connection_command
      await invoke("test_channel_connection_command", { name });
      return true;
    } catch {
      return false;
    }
  },

  updateTelegramConfig: async (config) => {
    try {
      // Save bot token to OS keyring if provided.
      if (config.token) {
        await invoke("keychain_set", {
          service: "mesoclaw",
          key: "telegram_bot_token",
          value: config.token,
        });
      }
      // Update local state.
      set((state) => ({
        channels: state.channels.map((ch) =>
          ch.name === "telegram"
            ? { ...ch, config: { type: "telegram", telegram: config } }
            : ch,
        ),
      }));
    } catch (err) {
      set({ error: String(err) });
    }
  },

  selectChannel: (name) => set({ selectedChannel: name }),

  setChannelStatus: (name, status, error = null) =>
    set((state) => ({
      channels: state.channels.map((ch) =>
        ch.name === name ? { ...ch, status, lastError: error } : ch,
      ),
    })),
}));
