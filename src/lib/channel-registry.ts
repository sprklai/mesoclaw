/**
 * Channel type registry.
 *
 * To add a new channel type, append an entry to CHANNEL_REGISTRY.
 * The onboarding wizard renders all entries and shows coming-soon badges
 * for unavailable channels.
 */

export interface ChannelTypeInfo {
  /** Internal ID matching the backend channel name. */
  id: string;
  displayName: string;
  description: string;
  /** Emoji icon shown in the channel picker card. */
  iconEmoji: string;
  /** Whether this channel can be configured right now. */
  available: boolean;
  comingSoonLabel?: string;
}

export const CHANNEL_REGISTRY: ChannelTypeInfo[] = [
  {
    id: "telegram",
    displayName: "Telegram",
    description: "Receive and send messages via a Telegram bot",
    iconEmoji: "✈️",
    available: true,
  },
  {
    id: "discord",
    displayName: "Discord",
    description: "Connect via a Discord bot",
    iconEmoji: "🎮",
    available: false,
    comingSoonLabel: "Coming soon",
  },
  {
    id: "slack",
    displayName: "Slack",
    description: "Integrate with a Slack workspace",
    iconEmoji: "💬",
    available: false,
    comingSoonLabel: "Coming soon",
  },
  {
    id: "whatsapp",
    displayName: "WhatsApp",
    description: "Connect via WhatsApp Business API",
    iconEmoji: "📱",
    available: false,
    comingSoonLabel: "Coming soon",
  },
];
