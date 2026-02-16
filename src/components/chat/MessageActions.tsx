/**
 * MessageActions Component
 *
 * Hover action menu for chat messages, similar to ChatGPT/Claude.
 * Shows Copy and Rerun (for assistant messages) actions that appear on hover.
 */

import { useCallback, useState } from "react";

import { Button } from "@/components/ui/button";
import { Tooltip } from "@/components/ui/tooltip";
import { Check, Copy, RotateCw } from "@/lib/icons";
import { cn } from "@/lib/utils";

interface MessageActionsProps {
  /** The message content to copy */
  content: string;
  /** Whether this is an assistant message (enables rerun) */
  isAssistant: boolean;
  /** Callback when rerun is clicked */
  onRerun?: () => void;
  /** Additional CSS classes */
  className?: string;
}

/**
 * Action button with copy functionality and visual feedback.
 */
function CopyButton({
  content,
  className,
}: {
  content: string;
  className?: string;
}) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(content);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Silently fail - clipboard API may not be available
    }
  }, [content]);

  return (
    <Tooltip content={copied ? "Copied!" : "Copy"} side="top">
      <Button
        variant="ghost"
        size="sm"
        className={cn("h-7 w-7 p-0", className)}
        onClick={handleCopy}
        type="button"
        aria-label={copied ? "Copied to clipboard" : "Copy message"}
      >
        {copied ? (
          <Check className="h-3.5 w-3.5 text-green-600" aria-hidden="true" />
        ) : (
          <Copy className="h-3.5 w-3.5" aria-hidden="true" />
        )}
      </Button>
    </Tooltip>
  );
}

/**
 * Rerun button for regenerating assistant responses.
 */
function RerunButton({
  onRerun,
  className,
}: {
  onRerun: () => void;
  className?: string;
}) {
  return (
    <Tooltip content="Regenerate" side="top">
      <Button
        variant="ghost"
        size="sm"
        className={cn("h-7 w-7 p-0", className)}
        onClick={onRerun}
        type="button"
        aria-label="Regenerate response"
      >
        <RotateCw className="h-3.5 w-3.5" aria-hidden="true" />
      </Button>
    </Tooltip>
  );
}

/**
 * Message actions that appear on hover.
 * Shows Copy for all messages, Rerun for assistant messages only.
 */
export function MessageActions({
  content,
  isAssistant,
  onRerun,
  className,
}: MessageActionsProps) {
  // Don't render if no content
  if (!content?.trim()) {
    return null;
  }

  return (
    <div
      className={cn(
        "flex items-center gap-0.5 rounded-md border border-border/50 bg-background/95 px-1 py-0.5 shadow-sm backdrop-blur-sm",
        "opacity-0 transition-opacity duration-150 group-hover:opacity-100",
        "focus-within:opacity-100", // Keep visible when focused
        className
      )}
    >
      <CopyButton content={content} />
      {isAssistant && onRerun && <RerunButton onRerun={onRerun} />}
    </div>
  );
}

export { CopyButton, RerunButton };
