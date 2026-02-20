/**
 * Zustand store for the Channel Management UI (Phase 7.2–7.4).
 *
 * Tracks the lifecycle status, configuration, and message counts for each
 * registered channel (Telegram, Discord, Matrix, Slack, etc.).
 *
 * Channel IPC commands are wired to the real ChannelManager backend.
 */

import { invoke } from "@tauri-apps/api/core";
import { create } from "zustand";

/** Keychain service identifier for all channel secrets. */
const CHANNEL_KEYCHAIN_SVC = "com.sprklai.mesoclaw";

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

/** Discord-specific connection configuration. */
export interface DiscordChannelConfig {
  /** Bot token from the Discord Developer Portal. */
  botToken: string;
  /** Comma-separated Discord guild (server) IDs. Empty = all guilds allowed. */
  allowedGuildIds: string;
  /** Comma-separated Discord channel IDs. Empty = all channels allowed. */
  allowedChannelIds: string;
}

/** Matrix-specific connection configuration. */
export interface MatrixChannelConfig {
  /** Full homeserver URL (e.g. `https://matrix.org`). */
  homeserverUrl: string;
  /** Bot MXID (e.g. `@mybot:matrix.org`). */
  username: string;
  /** Access token from the Matrix login API. */
  accessToken: string;
  /** Comma-separated room IDs. Empty = all joined rooms allowed. */
  allowedRoomIds: string;
}

/** Slack-specific connection configuration. */
export interface SlackChannelConfig {
  /** Bot User OAuth Token (`xoxb-…`). */
  botToken: string;
  /** App-Level Token for Socket Mode (`xapp-…`). */
  appToken: string;
  /** Comma-separated Slack channel IDs. Empty = all channels allowed. */
  allowedChannelIds: string;
}

/** Discriminated union for per-channel configuration. */
export type ChannelConfig =
  | { type: "telegram"; telegram: TelegramChannelConfig }
  | { type: "discord"; discord: DiscordChannelConfig }
  | { type: "matrix"; matrix: MatrixChannelConfig }
  | { type: "slack"; slack: SlackChannelConfig }
  | { type: "webhook" }
  | { type: "tauri-ipc" };

/** Live status and metadata for a registered channel. */
export interface ChannelEntry {
  /** Unique channel identifier (e.g. "telegram", "discord"). */
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
  {
    name: "discord",
    displayName: "Discord",
    status: "disconnected",
    messageCount: 0,
    lastError: null,
    config: {
      type: "discord",
      discord: {
        botToken: "",
        allowedGuildIds: "",
        allowedChannelIds: "",
      },
    },
  },
  {
    name: "matrix",
    displayName: "Matrix",
    status: "disconnected",
    messageCount: 0,
    lastError: null,
    config: {
      type: "matrix",
      matrix: {
        homeserverUrl: "",
        username: "",
        accessToken: "",
        allowedRoomIds: "",
      },
    },
  },
  {
    name: "slack",
    displayName: "Slack",
    status: "disconnected",
    messageCount: 0,
    lastError: null,
    config: {
      type: "slack",
      slack: {
        botToken: "",
        appToken: "",
        allowedChannelIds: "",
      },
    },
  },
];

// ─── Message types ────────────────────────────────────────────────────────────

export interface ChannelIncomingMessage {
  channel: string;
  from: string;
  content: string;
  timestamp: string;
}

// ─── Store ────────────────────────────────────────────────────────────────────

interface ChannelStore {
  channels: ChannelEntry[];
  selectedChannel: string | null;
  isLoading: boolean;
  error: string | null;
  /** Inbound messages keyed by channel name. */
  messages: Record<string, ChannelIncomingMessage[]>;

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
  /** Update the Discord configuration and persist it. */
  updateDiscordConfig: (config: DiscordChannelConfig) => Promise<void>;
  /** Update the Matrix configuration and persist it. */
  updateMatrixConfig: (config: MatrixChannelConfig) => Promise<void>;
  /** Update the Slack configuration and persist it. */
  updateSlackConfig: (config: SlackChannelConfig) => Promise<void>;
  /** Select a channel to show its config panel. */
  selectChannel: (name: string | null) => void;
  /** Internal: update the status of a named channel. */
  setChannelStatus: (name: string, status: ChannelStatus, error?: string | null) => void;
  /** Append a message to the given channel's message history. */
  addMessage: (channel: string, msg: ChannelIncomingMessage) => void;
  /** Clear all messages for the given channel. */
  clearMessages: (channel: string) => void;
}

export const useChannelStore = create<ChannelStore>((set, get) => ({
  channels: DEFAULT_CHANNELS,
  selectedChannel: null,
  isLoading: false,
  error: null,
  messages: {},

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

      // Restore Telegram config.
      try {
        const token = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:telegram:token" });
        const allowedChatIds = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:telegram:allowed_chat_ids" }).catch(() => "");
        const timeoutStr = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:telegram:polling_timeout_secs" }).catch(() => "30");
        const pollingTimeoutSecs = Number(timeoutStr) || 30;
        set((state) => ({
          channels: state.channels.map((ch) =>
            ch.name === "telegram"
              ? { ...ch, config: { type: "telegram", telegram: { token, allowedChatIds, pollingTimeoutSecs } } }
              : ch,
          ),
        }));
      } catch {
        // No saved Telegram config yet — keep defaults.
      }

      // Restore Discord config.
      try {
        const botToken = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:discord:token" });
        const allowedGuildIds = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:discord:allowed_guild_ids" }).catch(() => "");
        const allowedChannelIds = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:discord:allowed_channel_ids" }).catch(() => "");
        set((state) => ({
          channels: state.channels.map((ch) =>
            ch.name === "discord"
              ? { ...ch, config: { type: "discord", discord: { botToken, allowedGuildIds, allowedChannelIds } } }
              : ch,
          ),
        }));
      } catch {
        // No saved Discord config yet — keep defaults.
      }

      // Restore Matrix config.
      try {
        const homeserverUrl = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:matrix:homeserver_url" });
        const username = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:matrix:username" }).catch(() => "");
        const accessToken = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:matrix:access_token" });
        const allowedRoomIds = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:matrix:allowed_room_ids" }).catch(() => "");
        set((state) => ({
          channels: state.channels.map((ch) =>
            ch.name === "matrix"
              ? { ...ch, config: { type: "matrix", matrix: { homeserverUrl, username, accessToken, allowedRoomIds } } }
              : ch,
          ),
        }));
      } catch {
        // No saved Matrix config yet — keep defaults.
      }

      // Restore Slack config.
      try {
        const botToken = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:slack:bot_token" });
        const appToken = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:slack:app_token" }).catch(() => "");
        const allowedChannelIds = await invoke<string>("keychain_get", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:slack:allowed_channel_ids" }).catch(() => "");
        set((state) => ({
          channels: state.channels.map((ch) =>
            ch.name === "slack"
              ? { ...ch, config: { type: "slack", slack: { botToken, appToken, allowedChannelIds } } }
              : ch,
          ),
        }));
      } catch {
        // No saved Slack config yet — keep defaults.
      }
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  connectChannel: async (name) => {
    get().setChannelStatus(name, "reconnecting");
    try {
      const result = await invoke<{ name: string; connected: boolean; error: string | null }>(
        "start_channel_command",
        { name },
      );
      get().setChannelStatus(
        name,
        result.connected ? "connected" : "error",
        result.connected ? null : "health check failed after connecting",
      );
    } catch (err) {
      get().setChannelStatus(name, "error", String(err));
    }
  },

  disconnectChannel: async (name) => {
    try {
      await invoke("disconnect_channel_command", { name });
      get().setChannelStatus(name, "disconnected");
    } catch (err) {
      get().setChannelStatus(name, "error", String(err));
    }
  },

  testConnection: async (name) => {
    try {
      await invoke("test_channel_connection_command", { name });
      return true;
    } catch {
      return false;
    }
  },

  updateTelegramConfig: async (config) => {
    try {
      if (config.token) {
        await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:telegram:token", value: config.token });
      }
      await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:telegram:allowed_chat_ids", value: config.allowedChatIds });
      await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:telegram:polling_timeout_secs", value: String(config.pollingTimeoutSecs) });
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

  updateDiscordConfig: async (config) => {
    try {
      if (config.botToken) {
        await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:discord:token", value: config.botToken });
      }
      await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:discord:allowed_guild_ids", value: config.allowedGuildIds });
      await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:discord:allowed_channel_ids", value: config.allowedChannelIds });
      set((state) => ({
        channels: state.channels.map((ch) =>
          ch.name === "discord"
            ? { ...ch, config: { type: "discord", discord: config } }
            : ch,
        ),
      }));
    } catch (err) {
      set({ error: String(err) });
    }
  },

  updateMatrixConfig: async (config) => {
    try {
      if (config.homeserverUrl) {
        await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:matrix:homeserver_url", value: config.homeserverUrl });
      }
      if (config.username) {
        await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:matrix:username", value: config.username });
      }
      if (config.accessToken) {
        await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:matrix:access_token", value: config.accessToken });
      }
      await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:matrix:allowed_room_ids", value: config.allowedRoomIds });
      set((state) => ({
        channels: state.channels.map((ch) =>
          ch.name === "matrix"
            ? { ...ch, config: { type: "matrix", matrix: config } }
            : ch,
        ),
      }));
    } catch (err) {
      set({ error: String(err) });
    }
  },

  updateSlackConfig: async (config) => {
    try {
      if (config.botToken) {
        await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:slack:bot_token", value: config.botToken });
      }
      if (config.appToken) {
        await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:slack:app_token", value: config.appToken });
      }
      await invoke("keychain_set", { service: CHANNEL_KEYCHAIN_SVC, key: "channel:slack:allowed_channel_ids", value: config.allowedChannelIds });
      set((state) => ({
        channels: state.channels.map((ch) =>
          ch.name === "slack"
            ? { ...ch, config: { type: "slack", slack: config } }
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

  addMessage: (channel, msg) =>
    set((state) => ({
      messages: {
        ...state.messages,
        [channel]: [...(state.messages[channel] ?? []), msg],
      },
    })),

  clearMessages: (channel) =>
    set((state) => ({
      messages: { ...state.messages, [channel]: [] },
    })),
}));
