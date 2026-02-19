import { beforeEach, describe, expect, it } from 'vitest';
import { useChannelStore } from './channelStore';

describe('channelStore – message state', () => {
  beforeEach(() => {
    useChannelStore.setState({ messages: {} });
  });

  it('starts with empty messages', () => {
    expect(useChannelStore.getState().messages).toEqual({});
  });

  it('addMessage stores message under channel key', () => {
    const { addMessage } = useChannelStore.getState();
    addMessage('telegram', {
      channel: 'telegram',
      from: '123456',
      content: 'Hello',
      timestamp: '2026-02-18T00:00:00Z',
    });
    expect(useChannelStore.getState().messages['telegram']).toHaveLength(1);
    expect(useChannelStore.getState().messages['telegram'][0].content).toBe('Hello');
  });

  it('addMessage appends to existing channel messages', () => {
    const { addMessage } = useChannelStore.getState();
    const base = { channel: 'telegram', from: '123', timestamp: '' };
    addMessage('telegram', { ...base, content: 'First' });
    addMessage('telegram', { ...base, content: 'Second' });
    expect(useChannelStore.getState().messages['telegram']).toHaveLength(2);
  });

  it('clearMessages empties a channel', () => {
    const { addMessage, clearMessages } = useChannelStore.getState();
    addMessage('telegram', { channel: 'telegram', from: '1', content: 'hi', timestamp: '' });
    clearMessages('telegram');
    expect(useChannelStore.getState().messages['telegram']).toHaveLength(0);
  });
});
