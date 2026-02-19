import { listen } from '@tauri-apps/api/event';
import { useEffect } from 'react';
import { type ChannelIncomingMessage, useChannelStore } from '../stores/channelStore';

interface ChannelMessagePayload {
  type: string;
  channel?: string;
  from?: string;
  content?: string;
}

/**
 * Subscribes to Tauri `app-event` and routes `channel_message` payloads
 * into the channelStore message history.
 *
 * Mount once at app root (__root.tsx) for full-lifetime subscription.
 */
export function useChannelMessages(): void {
  const addMessage = useChannelStore((s) => s.addMessage);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    listen<ChannelMessagePayload>('app-event', (event) => {
      const p = event.payload;
      if (
        p.type === 'channel_message' &&
        p.channel &&
        p.from !== undefined &&
        p.content !== undefined
      ) {
        const msg: ChannelIncomingMessage = {
          channel: p.channel,
          from: p.from,
          content: p.content,
          timestamp: new Date().toISOString(),
        };
        addMessage(p.channel, msg);
      }
    })
      .then((fn) => { unlisten = fn; })
      .catch((e) => { console.error('[useChannelMessages] listen error:', e); });

    return () => { unlisten?.(); };
  }, [addMessage]);
}
