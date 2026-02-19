import { beforeEach, describe, expect, it, vi } from 'vitest';
import { renderHook } from '@testing-library/react';

const mockUnlisten = vi.fn();
const mockListen = vi.fn().mockResolvedValue(mockUnlisten);

vi.mock('@tauri-apps/api/event', () => ({ listen: mockListen }));

// Import AFTER mocking
const { useChannelMessages } = await import('./useChannelMessages');

describe('useChannelMessages', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('subscribes to app-event on mount', () => {
    renderHook(() => useChannelMessages());
    expect(mockListen).toHaveBeenCalledWith('app-event', expect.any(Function));
  });

  it('calls unlisten on unmount', async () => {
    const { unmount } = renderHook(() => useChannelMessages());
    // Give the promise time to resolve
    await vi.waitFor(() => expect(mockListen).toHaveBeenCalled());
    unmount();
    expect(mockUnlisten).toHaveBeenCalled();
  });
});
