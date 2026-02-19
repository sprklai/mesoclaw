# Telegram Interaction Complete (Option C) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire Telegram into a full bidirectional loop — inbound messages auto-trigger the agent (bot mode) and the response is sent back to Telegram; simultaneously messages appear in a frontend inbox where operators can read and manually reply.

**Architecture:**
- **Backend**: (1) Expose `send_channel_message_command` for manual operator replies. (2) Spawn a channel-agent bridge background task that subscribes to EventBus `ChannelMessage` events, runs `AgentLoop::run()` for each external-channel message (returns `Result<String>` directly), and routes the response back via `ChannelManager.send()`.
- **Frontend**: (1) Add message state slice to `channelStore`. (2) Create `useChannelMessages` hook that subscribes to Tauri `app-event`. (3) Create `/channels` inbox route with message feed and reply composer. (4) Mount hook and add nav link.

**Tech Stack:** Rust/Tokio, Tauri 2 IPC, `tokio::sync::broadcast`, Zustand, TanStack Router, React 19, Tailwind CSS 4

**Key facts from codebase:**
- `AgentLoop::run(&system_prompt, &message)` returns `Result<String, String>` — full response text
- `AppEvent::AgentComplete { session_id, message }` — `message` IS the response text
- `AppEvent::ChannelMessage { channel, from, content }` — `from` is the Telegram chat_id as string
- `ChannelManager::send(channel_name, message, recipient)` — already implemented, just needs an IPC wrapper
- `resolve_active_provider(pool)` — public fn in `agent_commands.rs` for provider resolution

---

### Task 1: Add `send_channel_message_command` to Backend

**Files:**
- Modify: `src-tauri/src/commands/channels.rs`
- Modify: `src-tauri/src/lib.rs` (invoke handler)

**Step 1: Write failing test**

Add to `src-tauri/src/commands/channels.rs` (before `#[cfg(test)]` or at end):

```rust
#[cfg(test)]
mod send_cmd_tests {
    // Compile-time check: function exists with correct signature.
    // Real integration test requires a live ChannelManager + registered channel.
    use super::*;
    #[test]
    fn send_channel_message_command_compiles() {
        // fn(String, String, Option<String>, State<Arc<ChannelManager>>) -> impl Future
        // Verified by the compiler — if this test file compiles, the command exists.
        let _ = send_channel_message_command as fn(_, _, _, _) -> _;
    }
}
```

**Step 2: Run test to confirm it fails**

```bash
cd /home/rakesh/RD/NSRTech/Tauri/tauriclaw/src-tauri
cargo test --lib commands::channels::send_cmd_tests 2>&1 | tail -5
```
Expected: compile error — `send_channel_message_command` not found.

**Step 3: Add the command**

In `src-tauri/src/commands/channels.rs`, add after the last existing command:

```rust
/// Send a message through a named channel to a specific recipient.
///
/// `channel` — registered channel name (e.g. `"telegram"`).
/// `recipient` — channel-specific address; for Telegram this is the chat ID as a string.
///              Pass `None` to broadcast to all peers (channel-dependent).
#[tauri::command]
pub async fn send_channel_message_command(
    channel: String,
    message: String,
    recipient: Option<String>,
    channel_manager: State<'_, Arc<ChannelManager>>,
) -> Result<(), String> {
    channel_manager
        .send(&channel, &message, recipient.as_deref())
        .await
}
```

**Step 4: Register in lib.rs invoke handler**

Find `.invoke_handler(tauri::generate_handler![` in `src-tauri/src/lib.rs`.
Add `commands::channels::send_channel_message_command,` alongside the other channel commands.

**Step 5: Run test to confirm it passes**

```bash
cd /home/rakesh/RD/NSRTech/Tauri/tauriclaw/src-tauri
cargo test --lib commands::channels 2>&1 | tail -10
```
Expected: PASS.

**Step 6: Full build check**

```bash
cargo check 2>&1 | grep "^error" | head -10
```
Expected: No errors.

**Step 7: Commit**

```bash
cd /home/rakesh/RD/NSRTech/Tauri/tauriclaw
git add src-tauri/src/commands/channels.rs src-tauri/src/lib.rs
git commit -m "feat(channels): expose send_channel_message_command IPC"
```

---

### Task 2: Channel-Agent Bridge Background Task

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Context:**
`AgentLoop::run()` returns `Result<String>` — the full response text. The bridge subscribes to the EventBus, runs the agent per inbound Telegram message, and calls `channel_manager.send()` with the response. The pattern mirrors what `start_agent_session_command` does.

**Step 1: Check what's already imported in lib.rs**

```bash
grep -n "resolve_active_provider\|AgentLoop\|AgentConfig\|SessionCancelMap\|EventFilter\|EventType" \
  /home/rakesh/RD/NSRTech/Tauri/tauriclaw/src-tauri/src/lib.rs
```
Expected: Some may be missing. Add any missing items to the use declarations at the top of lib.rs:

```rust
use crate::{
    agent::{
        agent_commands::{resolve_active_provider, SessionCancelMap},
        loop_::{AgentConfig, AgentLoop},
    },
    event_bus::{AppEvent, EventBus, EventFilter, EventType},
};
```

**Step 2: Find the channel router task in lib.rs**

```bash
grep -n "channel.router\|msg_rx\|bus_router\|channel-router" \
  /home/rakesh/RD/NSRTech/Tauri/tauriclaw/src-tauri/src/lib.rs
```
Expected: Shows the router task at ~lines 231-246. The bridge goes AFTER this block.

**Step 3: Write the bridge — add AFTER the channel-router spawn block**

Locate the end of the channel-router `tauri::async_runtime::spawn(...)` call. Immediately after it, add:

```rust
// ── Channel-Agent Bridge ──────────────────────────────────────────────────────
// Subscribes to ChannelMessage events and auto-triggers the agent.
// Runs the agent loop for each inbound external-channel message and routes
// the response back via ChannelManager.send() so Telegram users get a reply.
{
    let bridge_bus      = Arc::clone(&event_bus);
    let bridge_mgr      = Arc::clone(&channel_mgr);
    let bridge_pool     = Arc::clone(&db_pool);
    let bridge_registry = Arc::clone(&tool_registry);
    let bridge_policy   = Arc::clone(&security_policy);
    let bridge_identity = identity_loader.clone();
    let bridge_cancel   = Arc::clone(&cancel_map);

    tauri::async_runtime::spawn(async move {
        use tokio::sync::broadcast::error::RecvError;
        let mut rx = bridge_bus.subscribe_filtered(
            EventFilter::new(vec![EventType::ChannelMessage])
        );

        loop {
            match rx.recv().await {
                Ok(AppEvent::ChannelMessage { channel, from, content }) => {
                    // Skip the internal Tauri IPC channel — its messages are
                    // already handled by the desktop UI; routing them through
                    // the agent a second time would create a feedback loop.
                    if channel == "tauri_ipc" {
                        continue;
                    }

                    // Clone everything needed for the per-message spawned task.
                    let bus      = Arc::clone(&bridge_bus);
                    let mgr      = Arc::clone(&bridge_mgr);
                    let pool     = Arc::clone(&bridge_pool);
                    let reg      = Arc::clone(&bridge_registry);
                    let pol      = Arc::clone(&bridge_policy);
                    let ident    = bridge_identity.clone();
                    let cmap     = Arc::clone(&bridge_cancel);
                    let chan     = channel.clone();
                    let chat_id  = from.clone();

                    tauri::async_runtime::spawn(async move {
                        let provider = match resolve_active_provider(&pool) {
                            Ok(p) => p,
                            Err(e) => {
                                log::warn!("channel-bridge [{chan}]: provider error: {e}");
                                return;
                            }
                        };

                        // Session ID follows the same pattern as desktop sessions.
                        let session_id = format!("channel:dm:{chan}:{chat_id}");
                        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
                        if let Ok(mut map) = cmap.lock() {
                            map.insert(session_id.clone(), Arc::clone(&flag));
                        }

                        let _ = bus.publish(AppEvent::AgentStarted {
                            session_id: session_id.clone(),
                        });

                        let system_prompt = ident.build_system_prompt();
                        let agent = AgentLoop::new(
                            provider,
                            reg,
                            pol,
                            Some(bus.clone()),
                            AgentConfig::default(),
                        )
                        .with_cancel_flag(Arc::clone(&flag));

                        match agent.run(&system_prompt, &content).await {
                            Ok(response) => {
                                // Emit AgentComplete so TauriBridge forwards to
                                // the desktop frontend as well.
                                let _ = bus.publish(AppEvent::AgentComplete {
                                    session_id: session_id.clone(),
                                    message: response.clone(),
                                });
                                // Send response back through the originating channel.
                                if let Err(e) = mgr.send(&chan, &response, Some(&chat_id)).await {
                                    log::warn!("channel-bridge [{chan}]: send failed: {e}");
                                }
                            }
                            Err(e) => {
                                log::warn!("channel-bridge [{chan}]: agent error: {e}");
                            }
                        }

                        if let Ok(mut map) = cmap.lock() {
                            map.remove(&session_id);
                        }
                    });
                }
                Ok(_) => {} // Non-ChannelMessage event passed through filter — discard.
                Err(RecvError::Lagged(n)) => {
                    log::warn!("channel-bridge: lagged {n} events");
                }
                Err(RecvError::Closed) => {
                    log::info!("channel-bridge: event bus closed, exiting");
                    break;
                }
            }
        }
    });
}
```

**Step 4: Compile check**

```bash
cd /home/rakesh/RD/NSRTech/Tauri/tauriclaw/src-tauri
cargo check 2>&1 | grep "^error" | head -20
```
Expected: No errors. Fix any compile errors (usually missing imports or variable names that differ from lib.rs conventions) before continuing.

**Step 5: Run all backend tests**

```bash
cargo test --lib 2>&1 | tail -10
```
Expected: All tests pass (count ≥ 372).

**Step 6: Commit**

```bash
cd /home/rakesh/RD/NSRTech/Tauri/tauriclaw
git add src-tauri/src/lib.rs
git commit -m "feat(channels): add channel-agent bridge — inbound messages auto-trigger agent and reply is routed back"
```

---

### Task 3: Add Message State to channelStore

**Files:**
- Modify: `src/stores/channelStore.ts`
- Create: `src/stores/channelStore.messages.test.ts`

**Step 1: Write failing tests**

Create `src/stores/channelStore.messages.test.ts`:

```typescript
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
```

**Step 2: Run tests to confirm they fail**

```bash
cd /home/rakesh/RD/NSRTech/Tauri/tauriclaw
bun run test src/stores/channelStore.messages.test.ts 2>&1 | tail -10
```
Expected: FAIL — `messages`, `addMessage`, `clearMessages` not found.

**Step 3: Add the type and state to channelStore.ts**

Read `src/stores/channelStore.ts` first to understand the existing state shape. Then:

1. Add type before the store definition:
```typescript
export interface ChannelIncomingMessage {
  channel: string;
  from: string;
  content: string;
  timestamp: string;
}
```

2. Add to state interface/type:
```typescript
messages: Record<string, ChannelIncomingMessage[]>;
```

3. Add to initial state:
```typescript
messages: {},
```

4. Add actions inside the store `set` callback:
```typescript
addMessage: (channel: string, msg: ChannelIncomingMessage) =>
  set((state) => ({
    messages: {
      ...state.messages,
      [channel]: [...(state.messages[channel] ?? []), msg],
    },
  })),

clearMessages: (channel: string) =>
  set((state) => ({
    messages: { ...state.messages, [channel]: [] },
  })),
```

**Step 4: Run tests to confirm they pass**

```bash
bun run test src/stores/channelStore.messages.test.ts 2>&1 | tail -10
```
Expected: PASS (4/4).

**Step 5: TypeScript check**

```bash
bunx tsc --noEmit 2>&1 | head -10
```
Expected: No errors.

**Step 6: Commit**

```bash
git add src/stores/channelStore.ts src/stores/channelStore.messages.test.ts
git commit -m "feat(stores): add message state, addMessage, clearMessages to channelStore"
```

---

### Task 4: Create `useChannelMessages` Hook

**Files:**
- Create: `src/hooks/useChannelMessages.ts`
- Create: `src/hooks/useChannelMessages.test.ts`

**Context:**
Backend emits `AppEvent` serialized as `{ type: "channel_message", channel, from, content }` on the Tauri `"app-event"` channel. The hook subscribes and feeds messages into the channelStore.

**Step 1: Verify the AppEvent type in gateway-client.ts**

```bash
grep -A8 "channel_message\|ChannelMessage" \
  /home/rakesh/RD/NSRTech/Tauri/tauriclaw/src/lib/gateway-client.ts | head -20
```
Expected: Confirms field names (`channel`, `from`, `content`). If the type definition differs, adjust the hook accordingly.

**Step 2: Write failing tests**

Create `src/hooks/useChannelMessages.test.ts`:

```typescript
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
```

**Step 3: Run tests to confirm they fail**

```bash
bun run test src/hooks/useChannelMessages.test.ts 2>&1 | tail -10
```
Expected: FAIL — module not found.

**Step 4: Create the hook**

Create `src/hooks/useChannelMessages.ts`:

```typescript
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
 * Mount once at app root (App.tsx or __root.tsx) for full-lifetime subscription.
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
```

**Step 5: Run tests to confirm they pass**

```bash
bun run test src/hooks/useChannelMessages.test.ts 2>&1 | tail -10
```
Expected: PASS (2/2).

**Step 6: TypeScript check**

```bash
bunx tsc --noEmit 2>&1 | head -10
```
Expected: No errors.

**Step 7: Commit**

```bash
git add src/hooks/useChannelMessages.ts src/hooks/useChannelMessages.test.ts
git commit -m "feat(hooks): add useChannelMessages — routes app-event channel_message into store"
```

---

### Task 5: Create `/channels` Inbox Route

**Files:**
- Create: `src/routes/channels.tsx`

**Context:**
TanStack Router file-based routing — creating `src/routes/channels.tsx` registers the `/channels` route automatically. The route reads from channelStore and calls `send_channel_message_command` for manual replies.

**Step 1: Check existing route patterns**

```bash
head -20 /home/rakesh/RD/NSRTech/Tauri/tauriclaw/src/routes/settings.tsx
```
Expected: See how `createFileRoute` is used and what imports are typical. Match the existing pattern exactly.

**Step 2: Create the route**

Create `src/routes/channels.tsx`:

```tsx
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
                    ↩ Reply to {msg.from}
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
                    ✕ clear
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
                      handleSend();
                    }
                  }}
                  placeholder="Type a reply… Enter sends, Shift+Enter for newline"
                  rows={2}
                  className="flex-1 bg-neutral-900 border border-neutral-700 rounded px-3 py-2 text-sm text-neutral-100 placeholder-neutral-600 resize-none focus:outline-none focus:border-neutral-500"
                />
                <button
                  type="button"
                  onClick={handleSend}
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
```

**Step 3: TypeScript check**

```bash
cd /home/rakesh/RD/NSRTech/Tauri/tauriclaw
bunx tsc --noEmit 2>&1 | head -20
```
Expected: No errors. If TanStack Router reports the route isn't registered, check if a route tree file needs regeneration:
```bash
bunx @tanstack/router-cli generate
```

**Step 4: Commit**

```bash
git add src/routes/channels.tsx
git commit -m "feat(ui): add /channels inbox route with message feed and reply composer"
```

---

### Task 6: Mount Hook + Add Navigation Link

**Files:**
- Modify: `src/App.tsx` OR `src/routes/__root.tsx` (whichever is the root component)
- Modify: navigation/sidebar component

**Step 1: Find the root component**

```bash
# Check for root route file (TanStack Router convention)
ls /home/rakesh/RD/NSRTech/Tauri/tauriclaw/src/routes/__root.tsx 2>/dev/null && echo "found __root"
# Check App.tsx
grep -n "useEffect\|useMobileSwipe\|function App" \
  /home/rakesh/RD/NSRTech/Tauri/tauriclaw/src/App.tsx 2>/dev/null | head -10
```
Expected: Either `__root.tsx` or `App.tsx` is the root. Mount the hook in whichever renders first.

**Step 2: Mount `useChannelMessages` in root**

In the root component file, add:
```typescript
import { useChannelMessages } from './hooks/useChannelMessages';
// OR (if in routes/): import { useChannelMessages } from '../hooks/useChannelMessages';
```

Inside the root component function:
```typescript
useChannelMessages();
```

Place it alongside other top-level hooks (like `useMobileSwipe`, `useVirtualKeyboard` etc.).

**Step 3: Find the navigation component**

```bash
grep -rn "href.*settings\|to.*settings\|Settings.*Link\|Link.*Settings" \
  /home/rakesh/RD/NSRTech/Tauri/tauriclaw/src/ --include="*.tsx" | grep -v test | head -10
```
Expected: Find the nav/sidebar component. Note its import/link pattern.

**Step 4: Add Channels nav link**

In the navigation component, add a link matching the style of existing links:
```tsx
<Link to="/channels">Channels</Link>
```
Or if it's an `<a>` tag pattern: `<a href="/channels">Channels</a>`.

**Step 5: Full test suite**

```bash
cd /home/rakesh/RD/NSRTech/Tauri/tauriclaw

# Frontend
bun run test 2>&1 | tail -15

# Backend
cd src-tauri && cargo test --lib 2>&1 | tail -10
```
Expected: All tests pass.

**Step 6: Commit**

```bash
cd /home/rakesh/RD/NSRTech/Tauri/tauriclaw
git add src/App.tsx src/routes/__root.tsx  # whichever was modified
git add src/  # navigation component
git commit -m "feat(ui): mount useChannelMessages hook and add Channels nav link"
```

---

## End-to-End Verification

After all 6 tasks complete:

1. **Start the app**: `bun run tauri dev`
2. **Configure Telegram**: Settings → Channels → Telegram → enter bot token + your Telegram chat ID → Save → Connect (green dot)
3. **Open the Channels inbox**: Click "Channels" in the nav → see your connected channel in the sidebar
4. **Send a Telegram message** to the bot — within 30s (polling interval):
   - Message appears in the inbox ✅ (frontend inbox mode)
   - Bot auto-responds in Telegram ✅ (bot mode)
5. **Manual reply**: Type in the composer + Enter → message appears in your Telegram chat ✅

**Troubleshooting:**
```bash
# Agent bridge not responding — check logs
RUST_LOG=info cargo run 2>&1 | grep "channel-bridge"

# Messages not appearing in UI — check browser console
# Look for: [useChannelMessages] errors or events

# Send command failing — verify channel is connected
# invoke('list_channels_command') should show the channel as active
```

**Debugging the bridge specifically:**
```bash
# Temporarily add extra logging to the bridge in lib.rs:
log::info!("channel-bridge: received message from {from} on {channel}: {content}");
log::info!("channel-bridge: agent response: {response}");
```
