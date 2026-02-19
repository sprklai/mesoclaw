import { invoke } from '@tauri-apps/api/core';
import { createFileRoute } from '@tanstack/react-router';
import { useState } from 'react';
import { useChannelStore } from '../stores/channelStore';

export const Route = createFileRoute('/channels')({
  component: ChannelsPage,
});

function ChannelsPage() {
  const channels = useChannelStore((s) => s.channels);
  const messages = useChannelStore((s) => s.messages);
  const [selectedChannel, setSelectedChannel] = useState<string | null>(
    channels[0]?.name ?? null
  );
  const [replyText, setReplyText] = useState('');
  const [replyRecipient, setReplyRecipient] = useState('');
  const [sending, setSending] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);

  const channelMessages = selectedChannel ? (messages[selectedChannel] ?? []) : [];

  async function handleSend() {
    if (!selectedChannel || !replyText.trim()) return;
    setSending(true);
    setSendError(null);
    try {
      await invoke('send_channel_message_command', {
        channel: selectedChannel,
        message: replyText.trim(),
        recipient: replyRecipient.trim() || null,
      });
      setReplyText('');
    } catch (e) {
      setSendError(String(e));
    } finally {
      setSending(false);
    }
  }

  return (
    <div className="flex h-full">
      {/* Sidebar: channel list */}
      <aside className="w-52 shrink-0 border-r border-neutral-800 flex flex-col">
        <h2 className="px-4 py-3 text-xs font-semibold text-neutral-500 uppercase tracking-wider">
          Channels
        </h2>
        <ul className="flex-1 overflow-y-auto">
          {channels.map((ch) => {
            const count = (messages[ch.name] ?? []).length;
            return (
              <li key={ch.name}>
                <button
                  type="button"
                  onClick={() => setSelectedChannel(ch.name)}
                  className={`w-full text-left px-4 py-2 text-sm flex justify-between items-center transition-colors hover:bg-neutral-800 ${
                    selectedChannel === ch.name
                      ? 'bg-neutral-800 text-white'
                      : 'text-neutral-400'
                  }`}
                >
                  <span>{ch.name}</span>
                  {count > 0 && (
                    <span className="bg-blue-600 text-white text-xs rounded-full px-1.5 min-w-[1.25rem] text-center">
                      {count}
                    </span>
                  )}
                </button>
              </li>
            );
          })}
          {channels.length === 0 && (
            <li className="px-4 py-2 text-xs text-neutral-600">
              No channels connected.{' '}
              <a href="/settings" className="underline">
                Configure in Settings
              </a>
              .
            </li>
          )}
        </ul>
      </aside>

      {/* Main: message feed + composer */}
      <div className="flex-1 flex flex-col min-w-0">
        {selectedChannel ? (
          <>
            <header className="px-4 py-3 border-b border-neutral-800 text-sm font-medium text-neutral-300 shrink-0">
              #{selectedChannel}
            </header>

            <div className="flex-1 overflow-y-auto px-4 py-3 space-y-4">
              {channelMessages.length === 0 && (
                <p className="text-sm text-neutral-600">
                  No messages yet — waiting for inbound messages…
                </p>
              )}
              {channelMessages.map((msg, i) => (
                <div
                  key={`${msg.from}-${msg.timestamp}-${i}`}
                  className="flex flex-col gap-0.5"
                >
                  <div className="flex items-baseline gap-2">
                    <span className="text-xs font-medium text-neutral-400">{msg.from}</span>
                    <span className="text-xs text-neutral-600">
                      {msg.timestamp
                        ? new Date(msg.timestamp).toLocaleTimeString()
                        : ''}
                    </span>
                  </div>
                  <p className="text-sm text-neutral-200 whitespace-pre-wrap break-words">
                    {msg.content}
                  </p>
                  <button
                    type="button"
                    className="self-start text-xs text-neutral-600 hover:text-neutral-400 transition-colors"
                    onClick={() => setReplyRecipient(msg.from)}
                  >
                    Reply to {msg.from}
                  </button>
                </div>
              ))}
            </div>

            {/* Composer */}
            <div className="border-t border-neutral-800 px-4 py-3 flex flex-col gap-2 shrink-0">
              {replyRecipient && (
                <div className="flex items-center gap-2 text-xs text-neutral-500">
                  <span>To: {replyRecipient}</span>
                  <button
                    type="button"
                    className="text-neutral-600 hover:text-neutral-400 transition-colors"
                    onClick={() => setReplyRecipient('')}
                  >
                    x clear
                  </button>
                </div>
              )}
              <div className="flex gap-2">
                <textarea
                  value={replyText}
                  onChange={(e) => setReplyText(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' && !e.shiftKey) {
                      e.preventDefault();
                      void handleSend();
                    }
                  }}
                  placeholder="Type a reply… Enter sends, Shift+Enter for newline"
                  rows={2}
                  className="flex-1 bg-neutral-900 border border-neutral-700 rounded px-3 py-2 text-sm text-neutral-100 placeholder-neutral-600 resize-none focus:outline-none focus:border-neutral-500"
                />
                <button
                  type="button"
                  onClick={() => void handleSend()}
                  disabled={sending || !replyText.trim()}
                  className="px-4 py-2 bg-blue-600 text-white text-sm rounded hover:bg-blue-500 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                >
                  {sending ? 'Sending…' : 'Send'}
                </button>
              </div>
              {sendError && <p className="text-xs text-red-400">{sendError}</p>}
            </div>
          </>
        ) : (
          <div className="flex-1 flex items-center justify-center text-sm text-neutral-600">
            Select a channel
          </div>
        )}
      </div>
    </div>
  );
}
