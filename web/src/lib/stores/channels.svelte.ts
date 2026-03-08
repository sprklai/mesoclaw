import { apiGet, apiPost, apiDelete, apiPut } from "$lib/api/client";

export interface ChannelDef {
  id: string;
  name: string;
  description: string;
  credentials: { key: string; label: string; placeholder: string }[];
}

export interface ChannelWithStatus extends ChannelDef {
  configuredKeys: Set<string>;
  connected: boolean;
  status: string;
}

export interface ChannelConfig {
  telegram_dm_policy: string;
  telegram_polling_timeout: number;
  telegram_group_mention_only: boolean;
  slack_channel_allowlist: string[];
  discord_guild_allowlist: string[];
  discord_channel_allowlist: string[];
}

const BUILTIN_CHANNELS: ChannelDef[] = [
  {
    id: "telegram",
    name: "Telegram",
    description: "Telegram Bot",
    credentials: [
      {
        key: "token",
        label: "Bot Token",
        placeholder: "Bot token from @BotFather",
      },
      {
        key: "allowed_chat_ids",
        label: "Allowed Chat IDs",
        placeholder: "Comma-separated chat IDs (empty = all)",
      },
    ],
  },
  {
    id: "slack",
    name: "Slack",
    description: "Slack Bot (Socket Mode)",
    credentials: [
      {
        key: "bot_token",
        label: "Bot Token",
        placeholder: "xoxb-... Bot User OAuth Token",
      },
      {
        key: "app_token",
        label: "App Token",
        placeholder: "xapp-... App-Level Token for Socket Mode",
      },
      {
        key: "allowed_channel_ids",
        label: "Allowed Channel IDs",
        placeholder: "Comma-separated channel IDs (empty = all)",
      },
    ],
  },
  {
    id: "discord",
    name: "Discord",
    description: "Discord Bot",
    credentials: [
      {
        key: "token",
        label: "Bot Token",
        placeholder: "Bot token from Developer Portal",
      },
      {
        key: "allowed_guild_ids",
        label: "Allowed Server IDs",
        placeholder: "Comma-separated guild IDs (empty = all)",
      },
      {
        key: "allowed_channel_ids",
        label: "Allowed Channel IDs",
        placeholder: "Comma-separated channel IDs (empty = all)",
      },
    ],
  },
  {
    id: "matrix",
    name: "Matrix",
    description: "Matrix Protocol (Bridge Hub)",
    credentials: [
      {
        key: "homeserver_url",
        label: "Homeserver URL",
        placeholder: "https://matrix.org",
      },
      {
        key: "username",
        label: "Username (MXID)",
        placeholder: "@bot:matrix.org",
      },
      {
        key: "access_token",
        label: "Access Token",
        placeholder: "Access token from Element settings",
      },
      {
        key: "allowed_room_ids",
        label: "Allowed Room IDs",
        placeholder: "Comma-separated room IDs (empty = all)",
      },
    ],
  },
];

function credKey(channelId: string, field: string): string {
  return `channel:${channelId}:${field}`;
}

function createChannelsStore() {
  let channels = $state<ChannelWithStatus[]>([]);
  let loading = $state(false);
  let channelConfig = $state<ChannelConfig>({
    telegram_dm_policy: "allowlist",
    telegram_polling_timeout: 30,
    telegram_group_mention_only: true,
    slack_channel_allowlist: [],
    discord_guild_allowlist: [],
    discord_channel_allowlist: [],
  });

  return {
    get channels() {
      return channels;
    },
    get loading() {
      return loading;
    },
    get channelConfig() {
      return channelConfig;
    },

    async load() {
      loading = true;
      try {
        // Fetch configured credential keys
        let allKeys = new Set<string>();
        try {
          const keys = await apiGet<string[]>("/credentials");
          allKeys = new Set(keys.filter((k) => k.startsWith("channel:")));
        } catch {
          allKeys = new Set();
        }

        // Fetch channel statuses from backend (if channels feature active)
        let liveStatuses: Record<string, string> = {};
        try {
          const statusList =
            await apiGet<{ name: string; status: string }[]>("/channels");
          for (const s of statusList) {
            liveStatuses[s.name] = s.status;
          }
        } catch {
          // Channels feature may not be enabled
        }

        // Fetch config for channel settings
        try {
          const config = await apiGet<Record<string, unknown>>("/config");
          channelConfig = {
            telegram_dm_policy:
              (config.telegram_dm_policy as string) ?? "allowlist",
            telegram_polling_timeout:
              (config.telegram_polling_timeout as number) ?? 30,
            telegram_group_mention_only:
              (config.telegram_group_mention_only as boolean) ?? true,
            slack_channel_allowlist:
              (config.slack_channel_allowlist as string[]) ?? [],
            discord_guild_allowlist:
              (config.discord_guild_allowlist as string[]) ?? [],
            discord_channel_allowlist:
              (config.discord_channel_allowlist as string[]) ?? [],
          };
        } catch {
          // Use defaults
        }

        // Build channel list with per-channel credential status
        channels = BUILTIN_CHANNELS.map((def) => {
          const configuredKeys = new Set<string>();
          for (const cred of def.credentials) {
            const k = credKey(def.id, cred.key);
            if (allKeys.has(k)) configuredKeys.add(cred.key);
          }
          const liveStatus = liveStatuses[def.id];
          const hasRequiredToken = configuredKeys.size > 0;
          const isConnected = liveStatus === "Connected";
          return {
            ...def,
            configuredKeys,
            connected: isConnected,
            status: liveStatus
              ? liveStatus
              : hasRequiredToken
                ? "Not connected"
                : "Not configured",
          };
        });
      } finally {
        loading = false;
      }
    },

    async setCredential(channelId: string, field: string, value: string) {
      await apiPost("/credentials", {
        key: credKey(channelId, field),
        value,
      });
      await this.load();
    },

    async removeCredential(channelId: string, field: string) {
      await apiDelete(
        `/credentials/${encodeURIComponent(credKey(channelId, field))}`,
      );
      await this.load();
    },

    async testConnection(
      channelId: string,
    ): Promise<{ healthy: boolean; error?: string; latency_ms?: number }> {
      try {
        const result = await apiPost<{
          channel: string;
          healthy: boolean;
          error?: string;
          latency_ms?: number;
        }>(`/channels/${channelId}/test`, {});
        return {
          healthy: result.healthy,
          error: result.error,
          latency_ms: result.latency_ms,
        };
      } catch (e) {
        return {
          healthy: false,
          error: e instanceof Error ? e.message : "Connection failed",
        };
      }
    },

    async getCredentialValue(
      channelId: string,
      field: string,
    ): Promise<string | null> {
      try {
        const result = await apiGet<{ key: string; value: string }>(
          `/credentials/${encodeURIComponent(credKey(channelId, field))}/value`,
        );
        return result.value;
      } catch {
        return null;
      }
    },

    async updateConfig(fields: Partial<ChannelConfig>) {
      await apiPut("/config", fields);
      Object.assign(channelConfig, fields);
    },
  };
}

export const channelsStore = createChannelsStore();
