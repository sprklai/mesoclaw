import { SocialIcon } from "react-social-icons";
import { cn } from "@/lib/utils";

/** Channel type to social icon network mapping. */
const CHANNEL_TO_NETWORK: Record<string, string> = {
  telegram: "telegram",
  discord: "discord",
  matrix: "matrix",
  slack: "slack",
  webhook: "email",
  "tauri-ipc": "mailto",
};

interface ChannelIconProps {
  /** Channel type name (e.g., "telegram", "discord"). */
  channel: string;
  /** Size in pixels. Default: 24. */
  size?: number;
  /** Additional CSS classes. */
  className?: string;
}

/**
 * Renders a social media icon for a given channel type.
 * Falls back to a generic icon for unknown channels.
 */
export function ChannelIcon({ channel, size = 24, className }: ChannelIconProps) {
  const network = CHANNEL_TO_NETWORK[channel] ?? "mailto";

  return (
    <SocialIcon
      network={network}
      style={{ width: size, height: size }}
      className={cn("shrink-0", className)}
      fgColor="currentColor"
      bgColor="transparent"
    />
  );
}
