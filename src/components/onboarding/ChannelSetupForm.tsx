/**
 * ChannelSetupForm - Reusable channel configuration for onboarding
 *
 * Uses the existing channel configuration components from settings
 * but in a simplified layout suitable for onboarding flow.
 */

import { forwardRef, useImperativeHandle, useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ChannelIcon } from "@/components/channels/ChannelIcon";
import { CHANNEL_REGISTRY } from "@/lib/channel-registry";
import { cn } from "@/lib/utils";
import {
  type DiscordChannelConfig,
  type MatrixChannelConfig,
  type SlackChannelConfig,
  type TelegramChannelConfig,
  useChannelStore,
} from "@/stores/channelStore";
import { Loader2 } from "@/lib/icons";

export interface ChannelSetupFormRef {
  /** Save the current channel configuration */
  save: () => Promise<void>;
  /** Check if any channel is configured */
  hasConfiguredChannel: () => boolean;
}

interface ChannelSetupFormProps {
  /** Additional class name */
  className?: string;
}

export const ChannelSetupForm = forwardRef<ChannelSetupFormRef, ChannelSetupFormProps>(
  function ChannelSetupForm({ className }, ref) {
    const {
      updateTelegramConfig,
      updateDiscordConfig,
      updateMatrixConfig,
      updateSlackConfig,
      testConnection,
      channels,
    } = useChannelStore();

    const [selectedChannelId, setSelectedChannelId] = useState<string | null>(null);
    const [isTesting, setIsTesting] = useState(false);
    const [testResult, setTestResult] = useState<"ok" | "fail" | null>(null);

    // Telegram state
    const [telegramToken, setTelegramToken] = useState("");
    const [telegramChatIds, setTelegramChatIds] = useState("");
    const [telegramTimeout, setTelegramTimeout] = useState("30");

    // Discord state
    const [discordBotToken, setDiscordBotToken] = useState("");
    const [discordGuildIds, setDiscordGuildIds] = useState("");
    const [discordChannelIds, setDiscordChannelIds] = useState("");

    // Matrix state
    const [matrixHomeserver, setMatrixHomeserver] = useState("");
    const [matrixUsername, setMatrixUsername] = useState("");
    const [matrixAccessToken, setMatrixAccessToken] = useState("");
    const [matrixRoomIds, setMatrixRoomIds] = useState("");

    // Slack state
    const [slackBotToken, setSlackBotToken] = useState("");
    const [slackAppToken, setSlackAppToken] = useState("");
    const [slackChannelIds, setSlackChannelIds] = useState("");

    function resetForm() {
      setTestResult(null);
    }

    async function handleTestConnection() {
      if (!selectedChannelId) return;

      setIsTesting(true);
      setTestResult(null);
      try {
        // Save config first before testing
        await saveChannelConfig(selectedChannelId);
        const result = await testConnection(selectedChannelId);
        setTestResult(result ? "ok" : "fail");
      } catch {
        setTestResult("fail");
      } finally {
        setIsTesting(false);
      }
    }

    async function saveChannelConfig(channelId: string) {
      if (channelId === "telegram") {
        const config: TelegramChannelConfig = {
          token: telegramToken.trim(),
          allowedChatIds: telegramChatIds.trim(),
          pollingTimeoutSecs: Number(telegramTimeout) || 30,
        };
        await updateTelegramConfig(config);
      } else if (channelId === "discord") {
        const config: DiscordChannelConfig = {
          botToken: discordBotToken.trim(),
          allowedGuildIds: discordGuildIds.trim(),
          allowedChannelIds: discordChannelIds.trim(),
        };
        await updateDiscordConfig(config);
      } else if (channelId === "matrix") {
        const config: MatrixChannelConfig = {
          homeserverUrl: matrixHomeserver.trim() || "https://matrix.org",
          username: matrixUsername.trim(),
          accessToken: matrixAccessToken.trim(),
          allowedRoomIds: matrixRoomIds.trim(),
        };
        await updateMatrixConfig(config);
      } else if (channelId === "slack") {
        const config: SlackChannelConfig = {
          botToken: slackBotToken.trim(),
          appToken: slackAppToken.trim(),
          allowedChannelIds: slackChannelIds.trim(),
        };
        await updateSlackConfig(config);
      }
    }

    async function handleSave() {
      if (selectedChannelId) {
        await saveChannelConfig(selectedChannelId);
      }
    }

    const canTest = (): boolean => {
      switch (selectedChannelId) {
        case "telegram":
          return telegramToken.trim().length > 0;
        case "discord":
          return discordBotToken.trim().length > 0;
        case "matrix":
          return matrixAccessToken.trim().length > 0 && matrixHomeserver.trim().length > 0;
        case "slack":
          return slackBotToken.trim().length > 0;
        default:
          return false;
      }
    };

    // Check if any channel has been configured
    const hasConfiguredChannel = () => {
      return channels.some((ch) => ch.status === "connected");
    };

    // Expose save function via ref
    useImperativeHandle(ref, () => ({
      save: handleSave,
      hasConfiguredChannel,
    }), [handleSave, hasConfiguredChannel]);

    return (
      <div className={cn("space-y-4", className)}>
        {/* Channel Selection Grid */}
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          {CHANNEL_REGISTRY.map((channel) => (
            <button
              key={channel.id}
              type="button"
              disabled={!channel.available}
              onClick={() => {
                if (!channel.available) return;
                setSelectedChannelId(
                  selectedChannelId === channel.id ? null : channel.id,
                );
                resetForm();
              }}
              className={cn(
                "relative flex flex-col items-start gap-1 rounded-lg border p-4 text-left transition-colors",
                channel.available
                  ? "cursor-pointer hover:bg-accent"
                  : "cursor-default opacity-60",
                selectedChannelId === channel.id
                  ? "border-primary bg-primary/5 ring-2 ring-primary"
                  : "border-border bg-card",
              )}
            >
              {channel.comingSoonLabel && (
                <span className="absolute right-3 top-3 rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-muted-foreground">
                  {channel.comingSoonLabel}
                </span>
              )}
              <div className="flex items-center gap-2">
                <ChannelIcon channel={channel.id} size={24} />
                <span className="font-semibold text-foreground">
                  {channel.displayName}
                </span>
              </div>
              <span className="text-xs text-muted-foreground">
                {channel.description}
              </span>
            </button>
          ))}
        </div>

        {/* Selected Channel Configuration */}
        {selectedChannelId && (
          <div className="space-y-4 rounded-lg border border-border bg-card p-4">
            {/* Telegram Config */}
            {selectedChannelId === "telegram" && (
              <>
                <h3 className="flex items-center gap-2 font-semibold">
                  <ChannelIcon channel="telegram" size={20} />
                  Telegram Configuration
                </h3>
                <div className="space-y-4">
                  <div className="space-y-1.5">
                    <Label htmlFor="tg-token">Bot Token</Label>
                    <Input
                      id="tg-token"
                      type="password"
                      placeholder="123456:ABC-DEF..."
                      value={telegramToken}
                      onChange={(e) => {
                        setTelegramToken(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      Obtained from @BotFather. Stored securely in the OS keyring.
                    </p>
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="tg-chat-ids">Allowed Chat IDs</Label>
                    <Input
                      id="tg-chat-ids"
                      type="text"
                      placeholder="123456789, -1001234567890"
                      value={telegramChatIds}
                      onChange={(e) => {
                        setTelegramChatIds(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      Comma-separated Telegram chat IDs allowed to interact with the bot
                    </p>
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="tg-timeout">Polling Timeout (seconds)</Label>
                    <Input
                      id="tg-timeout"
                      type="number"
                      min={5}
                      max={60}
                      value={telegramTimeout}
                      onChange={(e) => setTelegramTimeout(e.target.value)}
                      className="w-32"
                    />
                  </div>
                </div>
              </>
            )}

            {/* Discord Config */}
            {selectedChannelId === "discord" && (
              <>
                <h3 className="flex items-center gap-2 font-semibold">
                  <ChannelIcon channel="discord" size={20} />
                  Discord Configuration
                </h3>
                <div className="space-y-4">
                  <div className="space-y-1.5">
                    <Label htmlFor="dc-token">Bot Token</Label>
                    <Input
                      id="dc-token"
                      type="password"
                      placeholder="MTExMjM0NTY3ODkwMTIzNDU2.Gh1234.abc..."
                      value={discordBotToken}
                      onChange={(e) => {
                        setDiscordBotToken(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      From Discord Developer Portal. Enable Message Content Intent.
                    </p>
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="dc-guilds">Allowed Server (Guild) IDs</Label>
                    <Input
                      id="dc-guilds"
                      type="text"
                      placeholder="123456789012345678, 987654321098765432"
                      value={discordGuildIds}
                      onChange={(e) => {
                        setDiscordGuildIds(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      Comma-separated Discord server IDs. Leave empty for all servers.
                    </p>
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="dc-channels">Allowed Channel IDs</Label>
                    <Input
                      id="dc-channels"
                      type="text"
                      placeholder="111222333444555666, 666555444333222111"
                      value={discordChannelIds}
                      onChange={(e) => {
                        setDiscordChannelIds(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      Comma-separated Discord channel IDs. Leave empty for all channels.
                    </p>
                  </div>
                </div>
              </>
            )}

            {/* Matrix Config */}
            {selectedChannelId === "matrix" && (
              <>
                <h3 className="flex items-center gap-2 font-semibold">
                  <ChannelIcon channel="matrix" size={20} />
                  Matrix Configuration
                </h3>
                <div className="space-y-4">
                  <div className="space-y-1.5">
                    <Label htmlFor="mx-homeserver">Homeserver URL</Label>
                    <Input
                      id="mx-homeserver"
                      type="url"
                      placeholder="https://matrix.org"
                      value={matrixHomeserver}
                      onChange={(e) => {
                        setMatrixHomeserver(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      Full URL of your Matrix homeserver including https://
                    </p>
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="mx-username">Username (MXID)</Label>
                    <Input
                      id="mx-username"
                      type="text"
                      placeholder="@mybot:matrix.org"
                      value={matrixUsername}
                      onChange={(e) => {
                        setMatrixUsername(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      Full Matrix ID including the server part
                    </p>
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="mx-token">Access Token</Label>
                    <Input
                      id="mx-token"
                      type="password"
                      placeholder="syt_dXNlcm5hbWU_abc123..."
                      value={matrixAccessToken}
                      onChange={(e) => {
                        setMatrixAccessToken(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      From Element Settings &gt; Help &amp; About. Stored securely.
                    </p>
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="mx-rooms">Allowed Room IDs</Label>
                    <Input
                      id="mx-rooms"
                      type="text"
                      placeholder="!abc123:matrix.org, !xyz789:matrix.org"
                      value={matrixRoomIds}
                      onChange={(e) => {
                        setMatrixRoomIds(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      Comma-separated room IDs. Leave empty for all joined rooms.
                    </p>
                  </div>
                </div>
              </>
            )}

            {/* Slack Config */}
            {selectedChannelId === "slack" && (
              <>
                <h3 className="flex items-center gap-2 font-semibold">
                  <ChannelIcon channel="slack" size={20} />
                  Slack Configuration
                </h3>
                <div className="space-y-4">
                  <div className="space-y-1.5">
                    <Label htmlFor="sl-bot-token">Bot Token</Label>
                    <Input
                      id="sl-bot-token"
                      type="password"
                      placeholder="xoxb-xxxxxxxxxxxx-xxxxxxxxxxxx-xxxxxxxxxxxx"
                      value={slackBotToken}
                      onChange={(e) => {
                        setSlackBotToken(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      Bot User OAuth Token from OAuth &amp; Permissions. Starts with xoxb-
                    </p>
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="sl-app-token">App Token (Socket Mode)</Label>
                    <Input
                      id="sl-app-token"
                      type="password"
                      placeholder="xapp-1-XXXXXXXXX-0000000000000-abc..."
                      value={slackAppToken}
                      onChange={(e) => {
                        setSlackAppToken(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      App-Level Token for Socket Mode. Starts with xapp-
                    </p>
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="sl-channels">Allowed Channel IDs</Label>
                    <Input
                      id="sl-channels"
                      type="text"
                      placeholder="C01234567AB, C09876543ZZ"
                      value={slackChannelIds}
                      onChange={(e) => {
                        setSlackChannelIds(e.target.value);
                        setTestResult(null);
                      }}
                    />
                    <p className="text-xs text-muted-foreground">
                      Comma-separated Slack channel IDs. Leave empty for all channels.
                    </p>
                  </div>
                </div>
              </>
            )}

            {/* Test Connection */}
            <div className="flex items-center gap-3 pt-2">
              <Button
                variant="outline"
                size="sm"
                onClick={handleTestConnection}
                disabled={isTesting || !canTest()}
              >
                {isTesting ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Testing...
                  </>
                ) : (
                  "Test Connection"
                )}
              </Button>

              {testResult === "ok" && (
                <span className="text-sm font-medium text-green-600">
                  Connected successfully
                </span>
              )}
              {testResult === "fail" && (
                <span className="text-sm font-medium text-destructive">
                  Connection failed
                </span>
              )}
            </div>
          </div>
        )}
      </div>
    );
  }
);
