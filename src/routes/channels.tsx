import { invoke } from "@tauri-apps/api/core";
import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Conversation,
  ConversationContent,
  ConversationScrollButton,
  Message,
  MessageContent,
  MessageResponse,
  PromptInput,
  PromptInputBody,
  PromptInputFooter,
  PromptInputSubmit,
  PromptInputTextarea,
} from "@/components/ai-elements";
import { PageHeader } from "@/components/layout/PageHeader";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import { useChannelStore } from "../stores/channelStore";
import { useContextPanelStore } from "@/stores/contextPanelStore";

export const Route = createFileRoute("/channels")({
  component: ChannelsPage,
});

function ChannelsContextPanel({
  selectedChannel,
  channelMessages,
  replyRecipient,
}: {
  selectedChannel: string | null;
  channelMessages: { from: string; content: string; timestamp: string }[];
  replyRecipient: string;
}) {
  const senders = [...new Set(channelMessages.map((m) => m.from))];

  return (
    <div className="space-y-4 p-4">
      {selectedChannel ? (
        <>
          <div>
            <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
              Channel
            </p>
            <p className="text-sm font-medium">#{selectedChannel}</p>
            <p className="mt-1 text-xs text-muted-foreground">
              {channelMessages.length} message{channelMessages.length !== 1 ? "s" : ""}
            </p>
          </div>

          {senders.length > 0 && (
            <div>
              <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Senders
              </p>
              <div className="space-y-1">
                {senders.slice(0, 5).map((sender) => (
                  <p key={sender} className="text-xs text-foreground">
                    {sender}
                  </p>
                ))}
              </div>
            </div>
          )}

          {replyRecipient && (
            <div>
              <p className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Replying To
              </p>
              <p className="text-xs text-primary">{replyRecipient}</p>
            </div>
          )}
        </>
      ) : (
        <p className="text-sm text-muted-foreground">Select a channel to see details.</p>
      )}
    </div>
  );
}

function ChannelsPage() {
  const { t } = useTranslation("channels");
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

  useEffect(() => {
    useContextPanelStore.getState().setContent(
      <ChannelsContextPanel
        selectedChannel={selectedChannel}
        channelMessages={channelMessages}
        replyRecipient={replyRecipient}
      />,
    );
    return () => useContextPanelStore.getState().clearContent();
  }, [selectedChannel, channelMessages, replyRecipient]);

  async function handleSend(text: string) {
    if (!selectedChannel || !text.trim()) return;
    setSending(true);
    setSendError(null);
    try {
      await invoke("send_channel_message_command", {
        channel: selectedChannel,
        message: text.trim(),
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
      <PageHeader title={t("title")} description={t("description")} />

      <div className="flex min-h-0 flex-1 flex-col md:flex-row">
        {/* Channel list */}
        <aside className="flex shrink-0 flex-col rounded-xl border border-border bg-sidebar md:w-48">
          <h2 className="px-3 py-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {t("sidebar.heading")}
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
                {t("sidebar.noChannels")}{" "}
                <a href="/settings" className="underline hover:text-primary">
                  {t("sidebar.configureLink")}
                </a>
                .
              </li>
            )}
          </ul>
        </aside>

        {/* Message area */}
        <div className="mt-2 flex min-w-0 flex-1 flex-col rounded-xl border border-border md:ml-4 md:mt-0">
          {selectedChannel ? (
            <>
              <header className="shrink-0 border-b border-border px-4 py-3">
                <span className="text-sm font-semibold">#{selectedChannel}</span>
              </header>

              <Conversation>
                <ConversationContent>
                  {channelMessages.length === 0 && (
                    <p className="text-sm text-muted-foreground p-4">
                      {t("messages.empty")}
                    </p>
                  )}
                  {channelMessages.map((msg, i) => (
                    <div key={`${msg.from}-${msg.timestamp ?? i}-${i}`}>
                      <div className="mb-1 flex items-baseline gap-2 px-1">
                        <span className="text-xs font-semibold text-foreground">
                          {msg.from}
                        </span>
                        <span className="text-xs text-muted-foreground">
                          {msg.timestamp
                            ? new Date(msg.timestamp).toLocaleTimeString()
                            : ""}
                        </span>
                      </div>
                      <Message from="assistant">
                        <MessageContent>
                          <MessageResponse>{msg.content}</MessageResponse>
                        </MessageContent>
                      </Message>
                      <button
                        type="button"
                        className="mt-1 self-start text-xs text-muted-foreground transition-colors hover:text-primary"
                        onClick={() => setReplyRecipient(msg.from)}
                      >
                        {t("messages.replyTo", { name: msg.from })}
                      </button>
                    </div>
                  ))}
                </ConversationContent>
                <ConversationScrollButton />
              </Conversation>

              {/* Composer */}
              <div className="shrink-0 border-t border-border px-4 py-3">
                {replyRecipient && (
                  <div className="mb-2 flex items-center gap-2 text-xs text-muted-foreground">
                    <span>{t("composer.recipientLabel", { recipient: replyRecipient })}</span>
                    <button
                      type="button"
                      className="transition-colors hover:text-destructive"
                      onClick={() => setReplyRecipient("")}
                      aria-label={t("composer.clearRecipient")}
                    >
                      ✕
                    </button>
                  </div>
                )}
                <PromptInput
                  value={replyText}
                  onChange={setReplyText}
                  onSubmit={(msg) => void handleSend(msg.text)}
                  isLoading={sending}
                >
                  <PromptInputBody>
                    <PromptInputTextarea
                      placeholder={t("composer.placeholder")}
                    />
                  </PromptInputBody>
                  <PromptInputFooter>
                    <PromptInputSubmit
                      status={sending ? "loading" : "ready"}
                      disabled={!replyText.trim()}
                    />
                  </PromptInputFooter>
                </PromptInput>
                {sendError && (
                  <p className="mt-2 text-xs text-destructive">{sendError}</p>
                )}
              </div>
            </>
          ) : (
            <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
              {t("selectPrompt")}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
