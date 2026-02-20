# MesoClaw vs OpenClaw UI/UX Gap Analysis

> **Analysis Date:** 2026-02-20 (Last Updated: 2026-02-20 - Phase 2 complete)
> **Purpose:** Identify UI/UX gaps between OpenClaw and MesoClaw to guide design improvements and feature prioritization.
> **Scope:** Visual design, interaction patterns, user flows, and experience quality—not backend functionality gaps (see `docs/gap_analysis.md` for those).

---

## Executive Summary

This analysis compares the UI/UX capabilities of MesoClaw (a Tauri 2.x desktop application) against OpenClaw (a self-hosted AI agent gateway framework). While the two products serve different primary use cases—MesoClaw as a focused desktop AI assistant and OpenClaw as a multi-channel agent orchestration platform—there are significant UI/UX patterns in OpenClaw that could enhance MesoClaw's user experience.

**Key Findings:**

1. **MesoClaw excels at desktop-native experiences** with its clean React 19 + Tailwind CSS 4 interface, proper state management via Zustand, and excellent desktop integration via Tauri 2.x.

2. **OpenClaw provides superior multi-channel orchestration** with real-time channel status, Live Canvas for visual artifacts, and companion mobile apps that provide seamless cross-device experiences.

3. **Priority gaps status (updated 2026-02-20):**
   - ✅ **P1:** Session persistence and history management in chat — **DONE** (`src-tauri/src/commands/chat_sessions.rs`)
   - ✅ **P1:** Command palette for power-user navigation — **DONE** (`src/components/CommandPalette.tsx`)
   - ✅ **P1:** Deep links for notification actions — **DONE** (`src/hooks/useDeepLinks.ts`, `mesoclaw://` scheme)
   - ✅ **P2:** Chat commands (/status, /new, /clear, /export) — **DONE** (`src/lib/chatCommands.ts`)
   - ✅ **P2:** Keyboard shortcuts system — **DONE** (`src/hooks/useGlobalShortcuts.ts`)
   - ✅ **P2:** Notification center for system events — **DONE** (`src/components/layout/NotificationCenter.tsx`)
   - ✅ **P2:** Real-time channel status indicators — **DONE** (`src/components/channels/ChannelStatusBadge.tsx`)
   - ⚠️ **P0:** Agent system — Default agent works, multi-agent CRUD pending

---

## Feature Comparison Matrix

| Feature | OpenClaw | MesoClaw | Gap Level | Notes |
|---------|----------|----------|-----------|-------|
| **Core Interface** |
| Desktop Application | ❌ Browser-only | ✅ Tauri 2.x | — | MesoClaw has native advantage |
| Mobile Companion Apps | ✅ iOS/Android | ❌ None | Medium | Planned for Phase 3 |
| Control UI/Dashboard | ✅ Vite + Lit SPA | ✅ React 19 | Low | Different stacks, similar capability |
| Dark Mode | ✅ | ✅ | — | Both support theming |
| Responsive Design | ✅ 5 breakpoints | ⚠️ 3 breakpoints | Low | Mobile-first work in progress |
| **Chat Interface** |
| Streaming Responses | ✅ SSE | ✅ Tauri Events | — | Both support streaming |
| Session Persistence | ✅ Full | ✅ SQLite backend | ~~High~~ ✅ DONE | `src-tauri/src/commands/chat_sessions.rs` |
| Chat History Browser | ✅ | ✅ ChatSessionStore | ~~High~~ ✅ DONE | Session resumption works |
| Chat Commands (/, /new) | ✅ Full set | ✅ /new, /clear, /status, /help, /export | ~~Medium~~ ✅ DONE | `src/lib/chatCommands.ts` |
| Model Selection | ✅ | ✅ Model Selector | — | Good implementation |
| Context Panel | ✅ | ✅ | — | Similar patterns |
| **Agent System** |
| Agent Configuration UI | ✅ Full CRUD | ⚠️ Default only | Medium | Multi-agent pending |
| Agent Workspace Editor | ✅ SOUL.md/AGENTS.md | ✅ Workspace files work | ~~Critical~~ ✅ DONE | `src-tauri/src/agent/commands.rs` |
| Session History Viewer | ✅ | ✅ list_sessions_command | ~~Critical~~ ✅ DONE | |
| Execution Monitor | ✅ Real-time | ✅ run_agent_command | ~~Critical~~ ✅ DONE | |
| **Multi-Channel** |
| Channel Configuration | ✅ Full | ✅ Telegram working | ~~High~~ ✅ DONE | `src-tauri/src/channels/` |
| Real-time Channel Status | ✅ WebSocket | ✅ StatusBadge component | ~~Medium~~ ✅ DONE | `src/components/channels/ChannelStatusBadge.tsx` |
| Channel Message History | ✅ Per-channel | ❌ | Medium | |
| **Visual Workspace** |
| Live Canvas (A2UI) | ✅ Artifact rendering | ❌ | Medium | Consider for Phase 2 |
| Artifact Preview | ✅ | ✅ AI SDK Elements | — | Similar capability |
| Code Block Rendering | ✅ | ✅ CodeBlock component | — | |
| **Voice Interaction** |
| Voice Wake Mode | ✅ "Hey Claw" | ❌ | Low | Platform-specific |
| Talk Mode (Push-to-talk) | ✅ | ❌ | Low | |
| **Navigation & Discovery** |
| Command Palette (Cmd+K) | ✅ | ✅ cmdk implementation | ~~High~~ ✅ DONE | `src/components/CommandPalette.tsx` |
| Global Search | ✅ | ❌ | Medium | Search across all content |
| Keyboard Shortcuts | ✅ | ✅ G+* navigation, mod+N, mod+K | ~~Medium~~ ✅ DONE | `src/hooks/useGlobalShortcuts.ts` |
| **System Integration** |
| System Tray | ✅ | ⚠️ Basic | Low | Tauri plugin available |
| Deep Links | ✅ | ✅ mesoclaw:// scheme | ~~Medium~~ ✅ DONE | `src/hooks/useDeepLinks.ts` |
| Notifications | ✅ Platform-native | ✅ tauri-plugin-notification | ~~Medium~~ ✅ DONE | |
| Notification Center | ✅ | ✅ Bell + unread badge + panel | ~~Medium~~ ✅ DONE | `src/components/layout/NotificationCenter.tsx` |
| **Security & Auth** |
| Device Pairing | ✅ QR code | ❌ | Low | Different model |
| Tailscale Integration | ✅ | ❌ | Low | Enterprise feature |

### Gap Level Legend
- **Critical** 🔴: Blocks core functionality, must fix immediately
- **High** 🟠: Significantly degrades user experience, fix soon
- **Medium** 🟡: Nice to have, improves experience
- **Low** 🟢: Future enhancement, low priority

---

## Priority-Ranked Recommendations

### P0: Critical (Blocks Core Functionality)

#### 1. ~~Agent System Backend Integration~~ ✅ DONE

**Status:** Default agent working. Multi-agent CRUD pending.

**Implementation:**
- `src-tauri/src/agent/commands.rs` - Full agent and session commands
- `src-tauri/src/agent/agent_commands.rs` - Agent loop execution
- `src/stores/agentConfigStore.ts` - Frontend store with IPC calls

**Remaining:**
- Multi-agent configuration UI (currently only default agent)

**Target State:**
- Full CRUD with SQLite persistence via Diesel ORM
- Real-time execution monitoring via Tauri events
- Session history with proper pagination

**Implementation Files:**
```
Backend:
  src-tauri/src/agent/mod.rs          - Agent module entry
  src-tauri/src/agent/commands.rs     - Tauri IPC commands
  src-tauri/src/database/models/agent.rs - Diesel models

Frontend:
  src/stores/agentConfigStore.ts      - Replace mock returns with invoke()
  src/routes/agents.tsx               - No changes needed (UI ready)
```

**Tauri 2.x Implementation Pattern:**
```rust
// src-tauri/src/agent/commands.rs
#[tauri::command]
pub async fn list_agents_command(
    pool: State<'_, DbPool>,
) -> Result<Vec<AgentConfig>, String> {
    use crate::database::schema::agents::dsl::*;

    let mut conn = pool.get().map_err(|e| e.to_string())?;
    let results = agents
        .filter(is_enabled.eq(true))
        .load::<AgentConfig>(&mut conn)
        .map_err(|e| e.to_string())?;

    Ok(results)
}
```

**Effort:** 3-5 days
**Dependencies:** Database migration for agents table

---

#### 2. ~~Session Persistence & Chat History~~ ✅ DONE

**Status:** Fully implemented with SQLite backend.

**Implementation:**
- `src-tauri/src/commands/chat_sessions.rs` - Full CRUD commands
- `src/stores/chatSessionStore.ts` - Zustand store with IPC
- Database schema in `src-tauri/migrations/`

---

### P1: High (Core User Experience)

#### 3. ~~Command Palette (Cmd+K / Ctrl+K)~~ ✅ DONE

**Status:** Fully implemented with `cmdk` library.

**Implementation:**
- `src/components/CommandPalette.tsx` - Full command palette
- `src/components/ui/command.tsx` - shadcn-style command components
- Navigation + chat actions integrated

---

#### 4. ~~Chat Commands System~~ ✅ DONE

**Status:** Implemented with 5 commands.

**Implementation:**
- `src/lib/chatCommands.ts` - Command parser and registry
- Commands: `/new`, `/clear`, `/status`, `/help`, `/export`

---

#### 4. Chat Commands System

**Current State:**
- No slash commands in chat
- All interactions via UI buttons

**Target State:**
- `/status` - Show current session/model info
- `/new` - Start new session
- `/clear` - Clear current conversation
- `/compact` - Summarize and compress history
- `/export` - Export conversation to file
- `/help` - Show available commands

**Implementation Pattern:**
```typescript
// src/lib/chat-commands.ts
const COMMANDS: Record<string, ChatCommand> = {
  "/status": {
    description: "Show current session status",
    execute: async (ctx) => {
      return `Model: ${ctx.model}\nMessages: ${ctx.messageCount}\nSession: ${ctx.sessionId}`;
    },
  },
  "/new": {
    description: "Start a new conversation",
    execute: async (ctx) => {
      await ctx.startNewSession();
      return "Started new conversation";
    },
  },
  "/clear": {
    description: "Clear current conversation",
    execute: async (ctx) => {
      ctx.clearMessages();
      return "Conversation cleared";
    },
  },
};

// In chat input handler
const handleSubmit = async (message: string) => {
  if (message.startsWith("/")) {
    const [cmd, ...args] = message.split(" ");
    const command = COMMANDS[cmd];
    if (command) {
      const result = await command.execute({ ...context, args });
      addSystemMessage(result);
    } else {
      addSystemMessage(`Unknown command: ${cmd}. Type /help for available commands.`);
    }
    return;
  }
  // Normal message handling...
};
```

**Effort:** 1 day
**Dependencies:** None

---

#### 5. Notification Action Deep Links

**Current State:**
- Basic notifications work via `notification_service.rs`
- No click actions, no deep links
- Notifications are informational only

**Target State:**
- Click notification → navigate to relevant route
- Deep link scheme: `mesoclaw://session/{id}`, `mesoclaw://agent/{id}`
- Tauri deep link plugin integration

**Implementation:**
```rust
// Register deep link scheme
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                app.deep_link().on_open_url(|event| {
                    let url = event.urls().first().unwrap();
                    // Parse mesoclaw://session/abc123
                    // Emit event to frontend for navigation
                    app.emit("deep-link", url).unwrap();
                });
            }
            Ok(())
        })
}
```

**Effort:** 1 day
**Dependencies:** `tauri-plugin-deep-link`

---

### P2: Medium (Enhanced Experience)

#### 6. Notification Center

**Current State:**
- Toast notifications via Sonner
- No persistent notification history
- No notification preferences

**Target State:**
- Bell icon in header with unread count
- Slide-out notification panel
- Notification categories with preferences
- Mark as read / Clear all actions

**UI Structure:**
```tsx
// src/components/layout/NotificationCenter.tsx
function NotificationCenter() {
  const [isOpen, setIsOpen] = useState(false);
  const { notifications, unreadCount } = useNotificationStore();

  return (
    <Popover open={isOpen} onOpenChange={setIsOpen}>
      <PopoverTrigger asChild>
        <Button variant="ghost" size="icon" className="relative">
          <Bell className="h-5 w-5" />
          {unreadCount > 0 && (
            <span className="absolute -top-1 -right-1 size-4 rounded-full bg-destructive text-xs text-destructive-foreground">
              {unreadCount}
            </span>
          )}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="end" className="w-80">
        <NotificationList notifications={notifications} />
      </PopoverContent>
    </Popover>
  );
}
```

**Effort:** 2 days
**Dependencies:** Notification store, backend persistence

---

#### 7. Real-time Channel Status Indicators

**Current State:**
- Channel configuration UI exists
- No connection status indicators
- No WebSocket monitoring

**Target State:**
- Green/Yellow/Red status indicators per channel
- Connection state in sidebar
- Reconnect button for disconnected channels
- Last message timestamp

**UI Pattern:**
```tsx
// Channel status badge
function ChannelStatusBadge({ channel }: { channel: Channel }) {
  const statusColors = {
    connected: "bg-green-500",
    connecting: "bg-yellow-500 animate-pulse",
    disconnected: "bg-red-500",
    error: "bg-red-600",
  };

  return (
    <div className="flex items-center gap-2">
      <span className={`size-2 rounded-full ${statusColors[channel.status]}`} />
      <span className="text-xs text-muted-foreground capitalize">
        {channel.status}
      </span>
    </div>
  );
}
```

**Effort:** 1-2 days
**Dependencies:** WebSocket integration for channels

---

#### 8. ~~Keyboard Shortcuts System~~ ✅ DONE

**Status:** Fully implemented with `react-hotkeys-hook`.

**Implementation:**
- `src/hooks/useGlobalShortcuts.ts` - Global shortcuts hook
- Shortcuts: mod+K (command palette), mod+N (new chat), mod+shift+C (clear)
- G+* navigation sequences (G+C, G+A, G+S, G+H, G+M, G+L)

**Remaining:**
- Add shortcuts overlay modal (Cmd+?)

---

export function useKeyboardShortcuts() {
  const navigate = useNavigate();

  useHotkeys("meta+k, ctrl+k", () => openCommandPalette());
  useHotkeys("meta+n, ctrl+n", () => navigate("/chat?new=true"));
  useHotkeys("meta+comma, ctrl+comma", () => navigate("/settings"));
  useHotkeys("meta+slash, ctrl+slash", () => openSearch());
}
```

**Effort:** 1 day
**Dependencies:** `react-hotkeys-hook` library

---

### P3: Nice-to-Have (Differentiators)

#### 9. Live Canvas (Visual Workspace)

**OpenClaw Feature:** A2UI provides a visual workspace for rendering AI-generated artifacts like diagrams, charts, and interactive components.

**MesoClaw Approach:**
- Use AI SDK Elements Artifact components
- Add iframe sandbox for safe rendering
- Support: Mermaid diagrams, Chart.js, React components

**Effort:** 5-7 days
**Priority:** Low (nice-to-have)

---

#### 10. Voice Interaction

**OpenClaw Feature:** "Hey Claw" wake word + push-to-talk mode.

**MesoClaw Approach:**
- Consider Web Speech API for TTS/STT
- Push-to-talk button in chat input
- Platform-specific: macOS uses on-device recognition

**Effort:** 3-5 days
**Priority:** Low (platform-dependent quality)

---

## Responsive Design Guidelines

### Current Breakpoint System

MesoClaw uses a 3-tier system via Tailwind CSS 4:

```css
/* Current breakpoints */
sm: 640px   /* Small tablets */
md: 768px   /* Tablets */
lg: 1024px  /* Laptops */
xl: 1280px  /* Desktops */
2xl: 1536px /* Large screens */
```

### Recommended Responsive Patterns

#### 1. Mobile-First Component Design

```tsx
// Always design for smallest screen first
<div className="
  flex flex-col           /* Mobile: vertical stack */
  md:flex-row md:gap-4    /* Tablet+: horizontal */
  lg:gap-8                /* Desktop: more spacing */
">
  <Sidebar className="hidden lg:block" />
  <MainContent className="flex-1" />
</div>
```

#### 2. Collapsible Sidebar Pattern

```tsx
// Sidebar collapses on mobile, slides over content
<Sidebar className="
  fixed inset-y-0 left-0 z-50
  w-64 transform transition-transform
  -translate-x-full md:translate-x-0
  data-[open=true]:translate-x-0
" />
```

#### 3. Touch-Friendly Targets

```tsx
// Minimum 44x44px touch targets
<Button className="h-11 w-11 md:h-10 md:w-10">
  <Icon className="h-5 w-5" />
</Button>
```

#### 4. Virtual Keyboard Handling

Already implemented in `src/routes/chat.tsx`:
```typescript
useEffect(() => {
  const vv = window.visualViewport;
  if (!vv) return;
  const handler = () => {
    const offset = window.innerHeight - vv.height;
    document.documentElement.style.setProperty("--keyboard-height", `${offset}px`);
  };
  vv.addEventListener("resize", handler);
  return () => vv.removeEventListener("resize", handler);
}, []);
```

### Safe Area Insets (iOS/Android)

When building mobile apps, add safe area handling:

```css
/* In global styles */
:root {
  --sat: env(safe-area-inset-top);
  --sar: env(safe-area-inset-right);
  --sab: env(safe-area-inset-bottom);
  --sal: env(safe-area-inset-left);
}

/* Apply to fixed elements */
.header {
  padding-top: var(--sat);
  padding-left: var(--sal);
  padding-right: var(--sar);
}

.bottom-input {
  padding-bottom: var(--sab);
}
```

---

## Mobile-First Considerations

### Tauri 2.x Mobile Development

Tauri 2.x supports iOS and Android via native plugins. Key considerations:

#### 1. Project Structure for Mobile

```
src-tauri/
├── gen/
│   ├── apple/           # Xcode project (auto-generated)
│   └── android/         # Android Studio project (auto-generated)
├── capabilities/
│   ├── default.json     # Desktop capabilities
│   ├── mobile.json      # Mobile-specific capabilities
└── tauri.conf.json      # Unified config
```

#### 2. Mobile-Specific Configuration

```json
// tauri.conf.json
{
  "app": {
    "windows": [
      {
        "title": "MesoClaw",
        "width": 800,
        "height": 600,
        "resizable": true,
        "fullscreen": false
      }
    ]
  },
  "bundle": {
    "active": true,
    "targets": ["app", "dmg", "msi", "appimage", "aab", "ipa"],
    "iOS": {
      "developmentTeam": "YOUR_TEAM_ID",
      "minimumSystemVersion": "13.0"
    },
    "android": {
      "minSdkVersion": 24
    }
  }
}
```

#### 3. Platform-Specific Code

```rust
// Conditional compilation for mobile
#[cfg(mobile)]
fn mobile_specific_setup(app: &tauri::AppHandle) {
    // Mobile-only initialization
}

#[cfg(desktop)]
fn desktop_specific_setup(app: &tauri::AppHandle) {
    // Desktop-only initialization
}
```

#### 4. Mobile Plugin Considerations

For mobile apps, you'll need platform-specific plugins:
- `tauri-plugin-biometric` - Face ID / Touch ID
- `tauri-plugin-nfc` - NFC reading (Android)
- `tauri-plugin-share` - Native share sheet
- `tauri-plugin-in-app-browser` - SFSafariViewController / Chrome Custom Tabs

---

## Technical Implementation Guidance (Tauri 2.x)

### Window State Persistence

Preserve window position and size across sessions:

```rust
// Cargo.toml
[dependencies]
tauri-plugin-window-state = "2"

// lib.rs
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Multi-Window Management

For features like detached chat windows or agent monitors:

```rust
use tauri::WebviewWindowBuilder;

#[tauri::command]
async fn open_agent_monitor(app: tauri::AppHandle, agent_id: String) -> Result<(), String> {
    let window = WebviewWindowBuilder::new(
        &app,
        format!("agent-monitor-{}", agent_id),
        tauri::WebviewUrl::App("agent-monitor.html".into()),
    )
    .title("Agent Monitor")
    .inner_size(400.0, 600.0)
    .build()
    .map_err(|e| e.to_string())?;

    window.show().map_err(|e| e.to_string())?;
    Ok(())
}
```

### State Management Best Practices

```rust
// Use tauri::State with Mutex for shared state
use std::sync::Mutex;
use tauri::State;

struct AppState {
    active_sessions: Mutex<HashMap<String, Session>>,
}

#[tauri::command]
fn get_session(
    session_id: String,
    state: State<AppState>,
) -> Result<Session, String> {
    let sessions = state.active_sessions.lock().map_err(|e| e.to_string())?;
    sessions.get(&session_id)
        .cloned()
        .ok_or_else(|| "Session not found".to_string())
}
```

### IPC Patterns: Commands + Events

For bidirectional communication (like streaming):

**Backend (Rust):**
```rust
#[tauri::command]
async fn stream_chat_command(
    request: ChatRequest,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let session_id = request.session_id.clone();

    // Emit events as they occur
    app.emit(&format!("chat-stream-{}", session_id), StreamEvent::Start)?;

    for token in generate_tokens(request) {
        app.emit(&format!("chat-stream-{}", session_id),
            StreamEvent::Token { content: token })?;
    }

    app.emit(&format!("chat-stream-{}", session_id), StreamEvent::Done)?;
    Ok(())
}
```

**Frontend (TypeScript):**
```typescript
import { listen } from "@tauri-apps/api/event";

const unlisten = await listen<StreamEvent>(
  `chat-stream-${sessionId}`,
  (event) => {
    switch (event.payload.type) {
      case "token":
        appendToken(event.payload.content);
        break;
      case "done":
        setIsStreaming(false);
        break;
    }
  }
);
```

---

## Actionable Roadmap

### Phase 1: Core Stability (Week 1-2) - ✅ COMPLETE

| Task | Priority | Effort | Status |
|------|----------|--------|--------|
| ~~Agent system backend integration~~ | P0 | 3-5 days | ✅ Default agent working |
| ~~Session persistence~~ | P0 | 2-3 days | ✅ SQLite backend |
| ~~Command palette~~ | P1 | 1-2 days | ✅ cmdk implementation |
| ~~Chat commands (/status, /new)~~ | P1 | 1 day | ✅ 5 commands |

### Phase 2: Enhanced UX (Week 3-4) - ✅ COMPLETE

| Task | Priority | Effort | Status |
|------|----------|--------|--------|
| ~~Notification center~~ | P2 | 2 days | ✅ Bell + unread badge + panel |
| ~~Keyboard shortcuts~~ | P2 | 1 day | ✅ react-hotkeys-hook |
| ~~Deep link actions~~ | P1 | 1 day | ✅ mesoclaw:// scheme wired |
| ~~Channel status indicators~~ | P2 | 1-2 days | ✅ StatusBadge component |

### Phase 3: Platform Expansion (Month 2)

| Task | Priority | Effort | Dependencies |
|------|----------|--------|--------------|
| iOS app foundation | P3 | 5-7 days | Apple Developer account |
| Android app foundation | P3 | 5-7 days | Google Play account |
| Voice input (optional) | P3 | 3-5 days | None |

### Phase 4: Advanced Features (Month 3+)

| Task | Priority | Effort | Dependencies |
|------|----------|--------|--------------|
| Live Canvas / A2UI | P3 | 5-7 days | Artifact components |
| Global search | P2 | 2-3 days | Search infrastructure |
| Session sync | P3 | 3-5 days | Cloud infrastructure |

---

## Success Metrics

Track these metrics to measure UX improvements:

| Metric | Target | Measurement |
|--------|--------|-------------|
| Session resumption rate | >80% | % of users who return to previous session |
| Command palette usage | >30% | % of navigation via Cmd+K |
| Keyboard shortcut adoption | >20% | % of actions via shortcuts |
| Chat command usage | >10% | % of actions via /commands |
| Notification action rate | >50% | % of notifications clicked |
| Mobile app rating | >4.0 | App store rating |

---

## Appendix: Related Documentation

- **Functional Gaps:** `docs/gap_analysis.md` - Backend and functional gaps
- **Implementation Plan:** `docs/implementation-plan.md` - Detailed technical plan
- **User Journey:** `docs/user-journey.md` - User flow documentation
- **App Usage Guide:** `docs/app_usage.md` - User-facing documentation
- **Tauri Configuration:** `src-tauri/tauri.conf.json` - App configuration
- **Frontend Standards:** `src/CLAUDE.md` - React/TypeScript conventions
- **Backend Standards:** `src-tauri/CLAUDE.md` - Rust conventions

---

*Document generated from analysis of OpenClaw documentation (docs.openclaw.ai, GitHub) and MesoClaw codebase exploration.*
