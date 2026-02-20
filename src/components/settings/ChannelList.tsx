/**
 * ChannelList — master list of registered channels with per-channel config
 * expansion.
 *
 * Each card shows:
 * - Channel icon + name
 * - Animated status dot (green/yellow/red)
 * - Message count badge
 * - Connect / Disconnect button
 *
 * Clicking a card expands an inline config panel for that channel.
 *
 * Phase 7.2–7.4 implementation.
 */

import { memo, useEffect } from "react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import {
  type ChannelEntry,
  type ChannelStatus,
  useChannelStore,
} from "@/stores/channelStore";

import { DiscordConfig } from "./DiscordConfig";
import { MatrixConfig } from "./MatrixConfig";
import { SlackConfig } from "./SlackConfig";
import { TelegramConfig } from "./TelegramConfig";

// ─── Helpers ──────────────────────────────────────────────────────────────────

function statusDotClass(status: ChannelStatus): string {
  switch (status) {
    case "connected":
      return "bg-green-500";
    case "reconnecting":
      return "bg-yellow-400 animate-pulse";
    case "error":
      return "bg-destructive";
    default:
      return "bg-muted-foreground/40";
  }
}

function statusLabel(status: ChannelStatus): string {
  switch (status) {
    case "connected":
      return "Connected";
    case "reconnecting":
      return "Reconnecting…";
    case "error":
      return "Error";
    default:
      return "Disconnected";
  }
}

function channelIcon(name: string): string {
  switch (name) {
    case "telegram":
      return "✈";
    case "discord":
      return "🎮";
    case "matrix":
      return "🔷";
    case "slack":
      return "💬";
    case "tauri-ipc":
      return "🖥";
    case "webhook":
      return "🔗";
    default:
      return "📡";
  }
}

// ─── ChannelCard ──────────────────────────────────────────────────────────────

interface ChannelCardProps {
  entry: ChannelEntry;
  isSelected: boolean;
  onSelect: () => void;
}

const ChannelCard = memo(function ChannelCard({ entry, isSelected, onSelect }: ChannelCardProps) {
  const { connectChannel, disconnectChannel } = useChannelStore();
  const isConnected = entry.status === "connected";
  const isWorking = entry.status === "reconnecting";

  const handleToggle = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (isConnected) {
      await disconnectChannel(entry.name);
    } else {
      await connectChannel(entry.name);
    }
  };

  return (
    <div className="space-y-0">
      {/* Card row */}
      <button
        type="button"
        onClick={onSelect}
        className={cn(
          "flex w-full items-center gap-4 rounded-lg border border-border p-4 text-left transition-colors hover:bg-muted/40",
          isSelected && "border-primary/50 bg-primary/5",
        )}
      >
        {/* Status dot */}
        <span
          className={cn(
            "h-2.5 w-2.5 shrink-0 rounded-full",
            statusDotClass(entry.status),
          )}
          title={statusLabel(entry.status)}
        />

        {/* Icon + name */}
        <span className="text-xl">{channelIcon(entry.name)}</span>
        <div className="min-w-0 flex-1">
          <p className="text-sm font-semibold">{entry.displayName}</p>
          <p className="text-xs text-muted-foreground">
            {statusLabel(entry.status)}
            {entry.lastError ? ` — ${entry.lastError}` : ""}
          </p>
        </div>

        {/* Message count */}
        {entry.messageCount > 0 && (
          <Badge variant="secondary" className="shrink-0 text-xs">
            {entry.messageCount.toLocaleString()} msgs
          </Badge>
        )}

        {/* Connect / Disconnect */}
        <div
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => e.stopPropagation()}
        >
          <Button
            size="sm"
            variant={isConnected ? "outline" : "default"}
            className="shrink-0"
            onClick={handleToggle}
            disabled={isWorking || entry.name === "tauri-ipc"}
            title={
              entry.name === "tauri-ipc"
                ? "Desktop IPC is always connected"
                : undefined
            }
          >
            {isWorking ? "…" : isConnected ? "Disconnect" : "Connect"}
          </Button>
        </div>
      </button>

      {/* Inline config panel */}
      {isSelected && entry.config.type !== "tauri-ipc" && (
        <div className="rounded-b-lg border border-t-0 border-border bg-background px-4 pb-4 pt-4">
          {entry.config.type === "telegram" && (
            <TelegramConfig config={entry.config.telegram} />
          )}
          {entry.config.type === "discord" && (
            <DiscordConfig config={entry.config.discord} />
          )}
          {entry.config.type === "matrix" && (
            <MatrixConfig config={entry.config.matrix} />
          )}
          {entry.config.type === "slack" && (
            <SlackConfig config={entry.config.slack} />
          )}
          {entry.config.type === "webhook" && (
            <p className="text-sm text-muted-foreground">
              Webhook configuration coming in a future update.
            </p>
          )}
        </div>
      )}
    </div>
  );
});

// ─── ChannelList ──────────────────────────────────────────────────────────────

export function ChannelList() {
  const { channels, selectedChannel, isLoading, error, loadChannels, selectChannel } =
    useChannelStore();

  useEffect(() => {
    loadChannels();
  }, [loadChannels]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <p className="text-sm text-muted-foreground">Loading channels…</p>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div>
        <h2 className="text-base font-semibold">Channels</h2>
        <p className="text-sm text-muted-foreground">
          Connect the agent to external messaging platforms.
        </p>
      </div>

      {error && (
        <p className="rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {error}
        </p>
      )}

      <div className="space-y-2">
        {channels.map((entry) => (
          <ChannelCard
            key={entry.name}
            entry={entry}
            isSelected={selectedChannel === entry.name}
            onSelect={() =>
              selectChannel(selectedChannel === entry.name ? null : entry.name)
            }
          />
        ))}
      </div>

      <p className="text-xs text-muted-foreground">
        Click a channel to configure it. Desktop IPC is always active and
        cannot be disconnected.
      </p>
    </div>
  );
}
