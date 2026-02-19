import { invoke } from "@tauri-apps/api/core";
import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";
import { PageHeader } from "@/components/layout/PageHeader";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useChannelStore } from "../stores/channelStore";

export const Route = createFileRoute("/channels")({
  component: ChannelsPage,
});

function ChannelsPage() {
  const channels = useChannelStore((s) => s.channels);
  const messages = useChannelStore((s) => s.messages);
  const [selectedChannel, setSelectedChannel] = useState<string | null>(
    channels[0]?.name ?? null,
  );
  const [replyText, setReplyText] = useState("");
  const [replyRecipient, setReplyRecipient] = useState("");
  const [sending, setSending] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);

  const channelMessages = selectedChannel ? (messages[selectedChannel] ?? []) : [];

  async function handleSend() {
    if (!selectedChannel || !replyText.trim()) return;
    setSending(true);
    setSendError(null);
    try {
      await invoke("send_channel_message_command", {
        channel: selectedChannel,
        message: replyText.trim(),
        recipient: replyRecipient.trim() || null,
      });
      setReplyText("");
    } catch (e) {
      setSendError(String(e));
    } finally {
      setSending(false);
    }
  }

  return (
    <div className="flex h-full flex-col">
      <PageHeader title="Channels" description="Incoming messages and replies" />

      <div className="flex min-h-0 flex-1">
        {/* Channel list */}
        <aside className="flex w-48 shrink-0 flex-col rounded-xl border border-border bg-sidebar">
          <h2 className="px-3 py-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            Channels
          </h2>
          <ul className="flex-1 overflow-y-auto px-2 pb-2">
            {channels.map((ch) => {
              const count = (messages[ch.name] ?? []).length;
              const isActive = selectedChannel === ch.name;
              return (
                <li key={ch.name}>
                  <button
                    type="button"
                    onClick={() => setSelectedChannel(ch.name)}
                    className={cn(
                      "flex w-full items-center justify-between rounded-lg px-3 py-2 text-sm transition-colors",
                      "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                      isActive
                        ? "bg-primary/10 text-primary font-medium"
                        : "text-foreground hover:bg-accent hover:text-accent-foreground",
                    )}
                  >
                    <span className="truncate">{ch.name}</span>
                    {count > 0 && (
                      <Badge variant="destructive" className="ml-2 shrink-0 text-xs">
                        {count}
                      </Badge>
                    )}
                  </button>
                </li>
              );
            })}
            {channels.length === 0 && (
              <li className="px-3 py-2 text-xs text-muted-foreground">
                No channels connected.{" "}
                <a href="/settings" className="underline hover:text-primary">
                  Configure in Settings
                </a>
                .
              </li>
            )}
          </ul>
        </aside>

        {/* Message area */}
        <div className="ml-4 flex min-w-0 flex-1 flex-col rounded-xl border border-border">
          {selectedChannel ? (
            <>
              <header className="shrink-0 border-b border-border px-4 py-3">
                <span className="text-sm font-semibold">#{selectedChannel}</span>
              </header>

              <div className="flex-1 space-y-4 overflow-y-auto px-4 py-4">
                {channelMessages.length === 0 && (
                  <p className="text-sm text-muted-foreground">
                    No messages yet — waiting for inbound messages…
                  </p>
                )}
                {channelMessages.map((msg, i) => (
                  <div
                    key={`${msg.from}-${msg.timestamp ?? i}-${i}`}
                    className="flex flex-col gap-1"
                  >
                    <div className="flex items-baseline gap-2">
                      <span className="text-xs font-semibold text-foreground">
                        {msg.from}
                      </span>
                      <span className="text-xs text-muted-foreground">
                        {msg.timestamp
                          ? new Date(msg.timestamp).toLocaleTimeString()
                          : ""}
                      </span>
                    </div>
                    <div className="rounded-xl rounded-tl-md bg-muted px-3 py-2 text-sm text-foreground">
                      <p className="whitespace-pre-wrap break-words">{msg.content}</p>
                    </div>
                    <button
                      type="button"
                      className="self-start text-xs text-muted-foreground transition-colors hover:text-primary"
                      onClick={() => setReplyRecipient(msg.from)}
                    >
                      Reply to {msg.from}
                    </button>
                  </div>
                ))}
              </div>

              {/* Composer */}
              <div className="shrink-0 border-t border-border px-4 py-3">
                {replyRecipient && (
                  <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
                    <span>To: {replyRecipient}</span>
                    <button
                      type="button"
                      className="text-muted-foreground transition-colors hover:text-destructive"
                      onClick={() => setReplyRecipient("")}
                      aria-label="Clear reply recipient"
                    >
                      ✕
                    </button>
                  </div>
                )}
                <div className="flex gap-2">
                  <textarea
                    value={replyText}
                    onChange={(e) => setReplyText(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter" && !e.shiftKey) {
                        e.preventDefault();
                        void handleSend();
                      }
                    }}
                    placeholder="Type a reply… Enter sends, Shift+Enter for newline"
                    rows={2}
                    className={cn(
                      "flex-1 resize-none rounded-xl border border-border bg-background px-3 py-2",
                      "text-sm text-foreground placeholder:text-muted-foreground",
                      "focus:border-primary/50 focus:outline-none focus:ring-2 focus:ring-ring/30",
                    )}
                  />
                  <Button
                    type="button"
                    onClick={() => void handleSend()}
                    disabled={sending || !replyText.trim()}
                  >
                    {sending ? "Sending…" : "Send"}
                  </Button>
                </div>
                {sendError && (
                  <p className="mt-2 text-xs text-destructive">{sendError}</p>
                )}
              </div>
            </>
          ) : (
            <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
              Select a channel
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
