# MesoClaw UI/UX Redesign — Design Document

> **Direction**: Warm Minimal (Direction A)
> **Approved**: 2026-02-18
> **Scope**: Full holistic redesign across all routes, preserving all functionality

---

## 1. Functionality Matrix (Preserved)

Every feature in the current UI is preserved. The matrix below cross-references current implementations with their redesigned counterparts.

| Feature | Current Location | Redesigned Location | Components |
|---|---|---|---|
| AI Chat (streaming) | `/chat` | `/chat` (refined) | AI SDK Elements (unchanged) |
| Model selector | Chat footer pill | Chat footer pill (refined) | `ModelSelectorDialog` (unchanged) |
| Provider management | `/settings` → AI Provider tab | `/settings` → AI Provider section | `SettingsNav` + existing tab content |
| API key management | AI Provider card | AI Provider section | Existing `ProviderCard` (unchanged) |
| Skills management | `/settings` → Skills tab | `/settings` → Skills section | Existing `SkillsSettingsTab` |
| App settings | `/settings` → App Settings tab | `/settings` → App Settings section | Existing `AppSettingsTab` |
| Identity editor | `/settings` → Identity tab | `/settings` → Identity section | Existing `IdentityEditor` |
| Scheduler / jobs | `/settings` → Scheduler tab | `/settings` → Scheduler section | Existing `JobList` |
| Modules | `/settings` → Modules tab | `/settings` → Modules section | Existing `ModuleList` |
| Channel configuration | `/settings` → Channels tab | `/settings` → Channels section | Existing `ChannelList` |
| Mobile settings | `/settings` → Mobile tab | `/settings` → Mobile section | Existing `MobileSettings` |
| Advanced settings | `/settings` → Advanced tab | `/settings` → Advanced section | Existing `AdvancedSettingsTab` |
| Telegram inbox | `/channels` | `/channels` (redesigned) | `ChannelMessage`, `Badge`, `Card` |
| Message send/reply | Channels composer | Channels composer (PromptInput style) | `PromptInput`-style composer |
| Memory search | `/memory` → Search tab | `/memory` → Search tab (refined) | Existing `MemorySearch` |
| Daily timeline | `/memory` → Timeline tab | `/memory` → Timeline tab (refined) | `Card`-wrapped `DailyTimeline` |
| Gateway status | Topbar | Sidebar footer | `GatewayStatus` (relocated) |
| Sidebar collapse | Sidebar toggle | Sidebar toggle + icon `Tooltip` | `Tooltip` added |
| Mobile navigation | Fixed bottom bar | Fixed bottom bar (refined) | `MobileNav` (styled) |
| Swipe gestures | Root layout | Root layout (unchanged) | `useMobileSwipe` (unchanged) |
| Virtual keyboard | Root layout | Root layout (unchanged) | `useVirtualKeyboard` (unchanged) |
| Channel messages hook | Root layout | Root layout (unchanged) | `useChannelMessages` (unchanged) |
| Theme (light/dark) | `ThemeProvider` | `ThemeProvider` (unchanged) | Design tokens updated |

---

## 2. Design Language System

### 2.1 Color Tokens

The color system is **parametric** — swapping the entire accent requires changing a single `--color-accent` HSL value in `globals.css`. All derived values (`--primary`, `--primary-foreground`, `--ring`) reference that base.

```css
/* globals.css — Light theme */
:root {
  /* Backgrounds */
  --background:          249 247 245;   /* #F9F7F5 warm off-white */
  --card:                255 255 255;   /* #FFFFFF pure white cards */
  --sidebar-background:  245 242 238;   /* #F5F2EE warm sidebar */
  --popover:             255 255 255;

  /* Borders */
  --border:              232 228 222;   /* #E8E4DE warm gray */
  --input:               232 228 222;

  /* Muted */
  --muted:               240 236 230;   /* #F0ECE6 */
  --muted-foreground:    121 116 110;   /* #79746E warm medium gray */

  /* Accent — parametric: change this one value to swap the whole palette */
  --color-accent-h: 38;                  /* Amber hue */
  --color-accent-s: 92%;
  --color-accent-l: 50%;

  /* Derived from accent */
  --primary:             var(--color-accent-h) var(--color-accent-s) var(--color-accent-l);
  --primary-foreground:  0 0% 100%;
  --ring:                var(--color-accent-h) var(--color-accent-s) var(--color-accent-l);

  /* Semantic */
  --destructive:         0 84% 60%;
  --success:             142 71% 45%;
  --warning:             var(--color-accent-h) 96% 56%;

  /* Typography */
  --foreground:          28 25 23;       /* #1C1917 warm near-black */
  --secondary:           68 64 60;       /* #444038 */

  /* Radius */
  --radius:              0.75rem;        /* 12px — rounded-xl base */
}

/* Dark theme */
.dark {
  --background:          15 14 13;       /* zinc-950 warm */
  --card:                24 23 21;       /* zinc-900 warm */
  --sidebar-background:  20 19 17;
  --border:              39 38 35;
  --muted:               32 31 28;
  --muted-foreground:    161 155 148;
  --foreground:          250 249 246;
  /* accent stays the same — amber works on dark too */
}
```

**To swap accent** (e.g., to sage green):
```css
--color-accent-h: 130;
--color-accent-s: 25%;
--color-accent-l: 40%;
```

### 2.2 Typography

| Use | Class | Size | Weight |
|---|---|---|---|
| Page title | `text-2xl font-semibold` | 24px | 600 |
| Section heading | `text-lg font-semibold` | 18px | 600 |
| Card title | `text-base font-medium` | 16px | 500 |
| Body copy | `text-sm` | 14px | 400 |
| Meta / timestamps | `text-xs text-muted-foreground` | 12px | 400 |
| Navigation labels | `text-sm font-medium` | 14px | 500 |

Font stack: `Geist Sans` (already in project) → `system-ui` → `sans-serif`.

### 2.3 Spacing & Radius

| Token | Value | Use |
|---|---|---|
| `rounded-2xl` | 16px | Cards, message bubbles, mobile nav container |
| `rounded-xl` | 12px | Input fields, modals |
| `rounded-lg` | 8px | Buttons, badges |
| `rounded-full` | 9999px | Avatars, status dots, pill tabs |
| `gap-4` / `p-4` | 16px | Standard intra-component spacing |
| `gap-6` / `p-6` | 24px | Section spacing |
| `gap-8` | 32px | Page-level spacing |

### 2.4 Elevation

| Level | Class | Use |
|---|---|---|
| Flat | `border border-border` | Default cards, inputs |
| Raised | `shadow-sm border border-border` | Interactive cards |
| Floating | `shadow-md` | Dropdowns, popovers |
| Overlay | `shadow-xl` | Modals, dialogs |

---

## 3. Layout Shell

### 3.1 Desktop (md+)

```
┌─ sidebar 240px (or 64px collapsed) ─┬──── main ────┬─── context panel 300px (xl+) ─┐
│ Logo + app name                      │              │                                │
│ ────────────────                     │  Page        │  <ContextPanel>                │
│ [Home]         (active: pill)        │  content     │  Current model                 │
│ [AI Chat]                            │              │  Memory snippets               │
│ [Memory]                             │              │  Session info                  │
│ [Channels]   [badge: unread count]   │              │                                │
│                                      │              │                                │
│ ─── bottom ──────────────────────── │              │                                │
│ [GatewayStatus]                      │              │                                │
│ [Settings]                           │              │                                │
└──────────────────────────────────────┴──────────────┴────────────────────────────────┘
```

**Changes from current**:
- No global topbar — each page renders its own `PageHeader` inside `<main>`
- Sidebar background: `bg-sidebar` (warm `#F5F2EE`)
- Active nav item: `bg-primary/10 text-primary rounded-lg` pill (not border-left)
- Collapsed state (w-16): every icon wrapped in `<Tooltip side="right">`
- `GatewayStatus` moves from topbar → sidebar footer above Settings link
- Unread badge on Channels: `<Badge variant="destructive">` count

### 3.2 Mobile

- Fixed bottom nav keeps structure but adds `shadow-[0_-4px_20px_rgba(0,0,0,0.06)]` lift
- Nav container gets `rounded-t-2xl` top corners
- Active item: icon scales to `scale-110` + amber text (current behavior kept)
- Safe area bottom padding via `env(safe-area-inset-bottom)` (current — unchanged)

### 3.3 Right Context Panel (xl+)

New `ContextPanel` component. Each page can populate it by calling:
```ts
useContextPanel({ content: <MyContextContent /> });
```
Default content when no page provides it:
- Current active model + provider
- Gateway connection status
- Last 3 memory entries (from `memoryStore`)
- Session start time

---

## 4. Page Redesigns

### 4.1 Home (`/`) — Dashboard

Replace the boilerplate placeholder with a real dashboard.

```
┌─────────────────────────────────────────────────────────┐
│ Good morning, [agent name from identityStore]           │  PageHeader
│ [timestamp]                         [gateway status]   │
├──────────────────────┬──────────────────────────────────┤
│ Recent Conversations │  Quick Actions                   │  2-col grid (md+)
│                      │                                  │
│ [chat entry card]    │  [New Chat →]                    │
│ [chat entry card]    │  [Open Memory →]                 │
│ [chat entry card]    │  [Channels →]                    │
│ [View all →]         │  [Settings →]                    │
│                      │                                  │
├──────────────────────┴──────────────────────────────────┤
│ System Status: Provider ✓  Memory ✓  Scheduler ✓        │  Status row
└─────────────────────────────────────────────────────────┘
```

**Components**:
- `DashboardCard` — new reusable `Card` with icon, title, description, arrow
- `AgentGreeting` — reads `identityStore` for agent name
- Recent chats: reads `channelStore.messages` + any future chat history store
- Quick actions: `Button variant="outline"` links with Lucide icons
- Status row: small `Badge` indicators per service

### 4.2 Chat (`/chat`) — Style refinement

**No behavioral changes** — only visual polish.

```
┌─────────────────────────────────┐
│ AI Chat            [model pill] │  PageHeader (new, replaces topbar)
├─────────────────────────────────┤
│                                 │
│  [assistant bubble — bg-card]   │  Left-aligned, border, shadow-sm
│                                 │
│           [user bubble]         │  Right-aligned, bg-primary/10
│                                 │
├─────────────────────────────────┤
│ [suggestions row]               │  Only on empty state
│ ┌─────────────────────────────┐ │
│ │ Type your message...        │ │  PromptInput — rounded-2xl
│ │ [model] [attach]    [send]  │ │
│ └─────────────────────────────┘ │
└─────────────────────────────────┘
```

**Token changes**:
- User message: `bg-primary/10 text-foreground rounded-2xl rounded-br-md`
- Assistant message: `bg-card border border-border rounded-2xl rounded-bl-md shadow-sm`
- `PromptInput` wrapper: `rounded-2xl border-2 border-border focus-within:border-primary/50`
- Suggestion pills: `rounded-full bg-muted hover:bg-primary/10 text-sm`

### 4.3 Settings (`/settings`) — VS Code navigation

Replace 9 horizontal tabs with a vertical `SettingsNav` component.

**Desktop layout**:
```
┌─── settings-nav 220px ──┬──── content area ──────────┐
│ Settings                │                             │
│                         │  [Section heading]          │
│ AI Provider         →   │                             │
│ Skills                  │  <Tab content renders here> │
│ App Settings            │                             │
│ Identity                │                             │
│ Scheduler               │                             │
│ Modules                 │                             │
│ Channels (config)       │                             │
│ Mobile                  │                             │
│ Advanced                │                             │
└─────────────────────────┴─────────────────────────────┘
```

**Mobile layout**: The `SettingsNav` collapses to a `Select` dropdown at the top of the page, with the section content below.

**Components**:
- `SettingsNav` — new component; nav items with `SidebarNav`-style active states
- Mobile `Select` fallback uses existing `Select` component from `src/components/ui/select.tsx`
- All existing tab content components (`AISettingsTab`, `SkillsSettingsTab`, etc.) are **unchanged** — only the nav wrapper changes

### 4.4 Channels (`/channels`) — Proper inbox

The entire page is rebuilt using design system tokens while preserving all `invoke` calls and store bindings.

```
┌─── channel list 200px ──┬──── message area ──────────┐
│ CHANNELS                │ #telegram                   │  PageHeader
│                         │─────────────────────────────│
│ telegram     [3]        │ [sender] [timestamp]        │
│ discord               │ Message text here...        │
│                         │ [Reply to sender]           │
│ ─────────────────────── │                             │
│ [+ Add Channel]         │ [sender] [timestamp]        │
│                         │ Another message...          │
│                         │─────────────────────────────│
│                         │ To: [sender name] [x]       │
│                         │ ┌─────────────────────────┐ │
│                         │ │ Type a reply...         │ │
│                         │ │                  [Send] │ │
│                         │ └─────────────────────────┘ │
└─────────────────────────┴─────────────────────────────┘
```

**Token changes**:
- Channel list: `bg-sidebar` + `SidebarNav`-style buttons, `Badge` for unread count
- Incoming messages: `bg-muted rounded-2xl rounded-tl-md p-3`
- Outgoing messages: `bg-primary/10 rounded-2xl rounded-tr-md p-3` (right-aligned)
- Composer: `rounded-xl border-2 border-border focus-within:border-primary/50`
- Send button: `Button variant="default"` (replaces raw `bg-blue-600`)

### 4.5 Memory (`/memory`) — Light polish

Minimal changes — mostly cosmetic.

- Tab list → `rounded-full` pill tabs (`TabsList rounded-full bg-muted p-1`)
- `MemorySearch` input → `rounded-xl` styling
- Timeline entries → wrapped in `Card` with `CardContent` padding
- Page title updated with consistent `PageHeader` component

---

## 5. New Components to Build

| Component | Location | Purpose |
|---|---|---|
| `PageHeader` | `src/components/layout/PageHeader.tsx` | Replaces per-page ad-hoc headers with consistent h1 + description |
| `ContextPanel` | `src/components/layout/ContextPanel.tsx` | Right-panel shell (xl+) with slot content |
| `ThemeToggle` | `src/components/ui/theme-toggle.tsx` | 3-mode Light/Dark/System switcher in sidebar footer; compact (cycle) in collapsed state |
| `contextPanelStore` | `src/stores/contextPanelStore.ts` | Zustand store so pages can push content to right panel |
| `SettingsNav` | `src/components/settings/SettingsNav.tsx` | VS Code–style vertical nav for settings; mobile `Select` fallback |
| `DashboardCard` | `src/components/ui/dashboard-card.tsx` | Reusable card for home dashboard quick actions |
| `AgentGreeting` | `src/components/AgentGreeting.tsx` | Personalized greeting from `identityStore` |
| `ChannelMessage` | `src/components/channels/ChannelMessage.tsx` | Consistent message bubble with proper tokens |

### Components to modify

| Component | File | Change |
|---|---|---|
| `Sidebar` | `src/components/ui/sidebar.tsx` | Add `Tooltip` for collapsed icons; warm bg token; move GatewayStatus |
| `MobileNav` | `src/components/layout/MobileNav.tsx` | Add `rounded-t-2xl shadow-[...]` lift |
| `__root.tsx` | `src/routes/__root.tsx` | Remove topbar block; add `ContextPanel` to right slot |
| `/` | `src/routes/index.tsx` | Full rewrite → Dashboard |
| `/settings` | `src/routes/settings.tsx` | Replace `Tabs` with `SettingsNav` |
| `/channels` | `src/routes/channels.tsx` | Full rewrite using design tokens |
| `/memory` | `src/routes/memory.tsx` | Minor polish |
| `globals.css` | `src/index.css` (or equivalent) | Add new warm token values |

---

## 6. Accessibility & Responsive Design

| Concern | Solution |
|---|---|
| Collapsed sidebar icon legibility | `<Tooltip side="right">` on all nav icons |
| Settings tab overflow on mobile | `SettingsNav` collapses to `<Select>` |
| Touch targets (WCAG 2.5.8) | All nav items min `44×44px` (current MobileNav already compliant) |
| Color contrast | Amber `#D97706` on `#F9F7F5` = 3.8:1 (AA large text); darken to `#B45309` for small text |
| Focus rings | `focus-visible:ring-2 focus-visible:ring-primary/50` on all interactives |
| Screen reader labels | `aria-label` on icon-only buttons (sidebar toggle, send button) |
| Keyboard navigation | `SettingsNav` items use `<button>` not `<div>` |

---

## 7. Implementation Tasks

Tasks are ordered by dependency. Each task is independently completable.

### Phase 1 — Foundation (no visible changes)

| # | Task | File(s) | Effort |
|---|---|---|---|
| 1.1 | Add warm color tokens to `globals.css` | `src/index.css` | S |
| 1.2 | Add `--sidebar-background` CSS var; apply to `Sidebar` | `src/components/ui/sidebar.tsx` | S |
| 1.3 | Create `PageHeader` component | `src/components/layout/PageHeader.tsx` | S |
| 1.4 | Create `contextPanelStore` (Zustand) | `src/stores/contextPanelStore.ts` | S |
| 1.5 | Create `ContextPanel` shell component | `src/components/layout/ContextPanel.tsx` | M |
| 1.6 | Create `DashboardCard` component | `src/components/ui/dashboard-card.tsx` | S |
| 1.7 | Create `AgentGreeting` component | `src/components/AgentGreeting.tsx` | S |

### Phase 2 — Layout shell

| # | Task | File(s) | Effort |
|---|---|---|---|
| 2.1 | Remove global topbar from `__root.tsx`; add `ContextPanel` to xl slot | `src/routes/__root.tsx` | S |
| 2.2 | Add `Tooltip` to collapsed sidebar icons; move `GatewayStatus` to sidebar footer | `src/components/ui/sidebar.tsx`, `sidebar-nav.tsx` | M |
| 2.3 | Apply warm bg token + pill active states to sidebar | `src/components/ui/sidebar/sidebar-nav-item.tsx` | S |
| 2.4 | Style `MobileNav`: add `rounded-t-2xl shadow` lift; amber active state | `src/components/layout/MobileNav.tsx` | S |
| 2.5 | Add unread-count `Badge` to Channels nav item | `src/components/ui/sidebar.tsx`, `MobileNav.tsx` | M |

### Phase 3 — Pages

| # | Task | File(s) | Effort |
|---|---|---|---|
| 3.1 | Rewrite home page as Dashboard | `src/routes/index.tsx` | M |
| 3.2 | Create `SettingsNav` component with mobile `Select` fallback | `src/components/settings/SettingsNav.tsx` | M |
| 3.3 | Refactor settings page to use `SettingsNav` | `src/routes/settings.tsx` | S |
| 3.4 | Rebuild `/channels` with design tokens + `ChannelMessage` component | `src/routes/channels.tsx`, `src/components/channels/` | L |
| 3.5 | Polish chat page: warm message bubbles + `PromptInput` styling | `src/routes/chat.tsx` | S |
| 3.6 | Polish memory page: pill tabs + `Card` timeline entries | `src/routes/memory.tsx` | S |

### Phase 4 — Context panel content

| # | Task | File(s) | Effort |
|---|---|---|---|
| 4.1 | Default `ContextPanel` content: model info + gateway status | `src/components/layout/ContextPanel.tsx` | M |
| 4.2 | Chat page pushes session info + model to `ContextPanel` | `src/routes/chat.tsx` | S |
| 4.3 | Memory page pushes recent entries to `ContextPanel` | `src/routes/memory.tsx` | S |

### Phase 5 — Polish & verification

| # | Task | File(s) | Effort |
|---|---|---|---|
| 5.1 | Run `bunx ultracite fix` across all modified files | All modified | S |
| 5.2 | Test all routes on mobile (< 768px) | Manual | M |
| 5.3 | Test collapsed sidebar tooltips | Manual | S |
| 5.4 | Test settings nav Select fallback on mobile | Manual | S |
| 5.5 | Verify all existing functionality preserved (channel send, API key save, model select) | Manual | M |
| 5.6 | Accessibility: run axe or Lighthouse on each page | Manual | M |
| 5.7 | Dark mode verification across all pages | Manual | S |

**Effort key**: S = Small (< 1hr), M = Medium (1-3hr), L = Large (3-6hr)

---

## 8. Component Library References

### AI SDK Elements (already used)

| Component | Used in | Notes |
|---|---|---|
| `Conversation` / `ConversationContent` | Chat | No change needed |
| `Message` / `MessageContent` / `MessageResponse` | Chat | Apply warm bubble tokens via className |
| `PromptInput` + sub-components | Chat | Apply `rounded-2xl` + warm border focus |
| `ModelSelectorDialog` | Chat | No change needed |
| `Suggestions` / `Suggestion` | Chat empty state | Apply `rounded-full` pill styling |
| `Reasoning` | (available) | Use when extended thinking is added |
| `Loader` | (available) | Add streaming indicator |

### shadcn/ui (to add)

| Component | Use | Install |
|---|---|---|
| `Tooltip` | Collapsed sidebar icon labels | Already in `src/components/ui/tooltip.tsx` ✓ |
| `Badge` | Unread counts, status indicators | Already in `src/components/ui/badge.tsx` ✓ |
| `Card` / `CardHeader` / `CardContent` | Dashboard + channels | Needs `npx shadcn@latest add card` |
| `Select` | Settings nav mobile fallback | Already in `src/components/ui/select.tsx` ✓ |
| `Avatar` | Channel sender icons | Needs `npx shadcn@latest add avatar` |

---

## 9. Design Rationale Summary

1. **Warm Minimal over dark-first** — The app's target users include non-developers (team leads, privacy-conscious users). Warm neutral whites are less intimidating than terminal-dark UIs.

2. **Parametric accent over hard-coded color** — A single CSS HSL triplet controls the entire accent palette. The user specifically requested this; it also future-proofs white-labeling.

3. **Remove global topbar** — Tauri desktop apps don't have browser chrome. Giving that 56px back to content (especially on small laptop screens) materially improves information density.

4. **VS Code settings nav over tabs** — 9 tabs overflow at any reasonable width on mobile. A vertical nav with a mobile `Select` fallback is a well-established pattern that scales to any number of sections.

5. **Channels as proper inbox** — The current channels page is the most broken surface in the app. It uses `neutral-*` raw classes outside the design system entirely. Rebuilding it with tokens makes it consistent and maintainable.

6. **Context panel over empty slot** — The xl right panel was intentionally reserved. Rather than let it render as blank space, populating it with contextually relevant data (model info, memory snippets) makes wide-screen layouts meaningfully better.

7. **AI SDK Elements kept** — The existing chat implementation is well-built and uses the upstream library correctly. The redesign applies warm token overrides via `className` props rather than replacing the component tree.
