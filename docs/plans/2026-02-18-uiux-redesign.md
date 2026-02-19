# UI/UX Redesign — Warm Minimal Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Redesign MesoClaw's UI holistically using the approved "Warm Minimal" direction — dashboard home, VS Code–style settings nav, proper Channels inbox, per-page headers replacing the global topbar, and an xl+ context panel.

**Architecture:** Thin CSS token layer for the parametric accent; five focused page/component rewrites; new `ContextPanel` + `SettingsNav` components; all existing Tauri IPC calls and stores untouched. Every change is an additive restyle or drop-in replacement.

**Tech Stack:** React 19, TypeScript, Tailwind CSS 4 (OKLCH tokens), shadcn/ui component style, AI SDK Elements, TanStack Router, Zustand. Run `bun run test` for frontend, `bunx ultracite fix` for formatting.

**Design doc:** `docs/uiux.md`

**Pre-flight check:**
```bash
# Verify all tests pass before starting
bun run test
# Expected: 30 tests pass
```

---

## Discovery Notes (read before starting)

- CSS tokens: `src/styles/globals.css` (OKLCH, hue 85 = warm amber — already warm!)
- Extra tokens: `src/styles/design-tokens.css`
- Sidebar already has `Tooltip` on collapsed icons — no work needed there
- Sidebar already uses `--sidebar-*` CSS variables
- `GatewayStatus` currently lives in `src/routes/__root.tsx` topbar
- Settings route: `src/routes/settings.tsx` uses shadcn `<Tabs>` with 9 triggers
- Channels route: `src/routes/channels.tsx` uses raw `neutral-*` Tailwind classes (outside design system)
- Right panel in `__root.tsx` renders as `<aside className="hidden ... xl:flex xl:w-80">`

---

## Task 1: Add Parametric Accent to CSS Tokens

**Goal:** Make swapping the accent color a one-variable change.

**Files:**
- Modify: `src/styles/globals.css`

### Step 1: Read the current `:root` block

```bash
head -80 src/styles/globals.css
```

### Step 2: Add `--accent-hue` variable at the TOP of `:root`

In `src/styles/globals.css`, in the `:root` block, **before** `--primary`, add:

```css
/* ── Parametric accent — change only this to swap the entire accent palette ── */
--accent-hue: 85;   /* 85 = warm amber. Try 145 (sage), 250 (indigo), 0 (red) */
```

Then update all hardcoded `85` hue references in `:root` to use `var(--accent-hue)`:

```css
/* Replace every oklch(...  85) in :root with oklch(... var(--accent-hue)) */
--primary:              oklch(0.75 0.18 var(--accent-hue));
--primary-foreground:   oklch(0.2 0.04 var(--accent-hue));
--secondary:            oklch(0.96 0.02 var(--accent-hue));
--secondary-foreground: oklch(0.35 0.04 var(--accent-hue));
--muted:                oklch(0.96 0.015 var(--accent-hue));
--muted-foreground:     oklch(0.5 0.02 var(--accent-hue));
--accent:               oklch(0.92 0.08 var(--accent-hue));
--accent-foreground:    oklch(0.35 0.08 var(--accent-hue));
--border:               oklch(0.91 0.025 var(--accent-hue));
--input:                oklch(0.91 0.025 var(--accent-hue));
--ring:                 oklch(0.75 0.18 var(--accent-hue));
--sidebar:              oklch(0.98 0.01 var(--accent-hue));
--sidebar-foreground:   oklch(0.2 0.02 var(--accent-hue));
--sidebar-primary:      oklch(0.75 0.18 var(--accent-hue));
--sidebar-primary-foreground: oklch(0.2 0.04 var(--accent-hue));
--sidebar-accent:       oklch(0.92 0.08 var(--accent-hue));
--sidebar-accent-foreground:  oklch(0.35 0.08 var(--accent-hue));
--sidebar-border:       oklch(0.91 0.025 var(--accent-hue));
--sidebar-ring:         oklch(0.75 0.18 var(--accent-hue));
--background:           oklch(0.995 0.005 var(--accent-hue));
--foreground:           oklch(0.2 0.02 var(--accent-hue));
--card:                 oklch(1 0 0);
--card-foreground:      oklch(0.2 0.02 var(--accent-hue));
--popover:              oklch(1 0 0);
--popover-foreground:   oklch(0.2 0.02 var(--accent-hue));
```

Do the same for `.dark` block — add `--accent-hue: 85;` at the top and replace all `85` hue values.

> **Note:** `oklch()` CSS functions do NOT support `var()` for individual channel values in all browsers. **Use the `@property` approach instead:**

```css
/* At the very top of :root, BEFORE other vars: */
@property --accent-hue {
  syntax: "<number>";
  inherits: true;
  initial-value: 85;
}
```

> If `@property` + `var()` in `oklch()` doesn't work in Tauri's WebView (Chromium-based, should work), fall back to defining a comment `/* ACCENT HUE: change 85 below */` above each group and accept multi-value editing. Test in dev first.

### Step 3: Run the app and visually verify colors look the same

```bash
bun run dev
```

Open the app. Colors should look identical to before — you changed the structure, not the values.

### Step 4: Commit

```bash
git add src/styles/globals.css
git commit -m "feat(tokens): add parametric --accent-hue for one-variable accent swapping"
```

---

## Task 2: Create `PageHeader` Component

**Goal:** A consistent page-level header used by all pages to replace the global topbar and ad-hoc per-page headers.

**Files:**
- Create: `src/components/layout/PageHeader.tsx`
- Create: `src/components/layout/__tests__/PageHeader.test.tsx`

### Step 1: Write the failing test

```bash
# Create the test file
mkdir -p src/components/layout/__tests__
```

Create `src/components/layout/__tests__/PageHeader.test.tsx`:

```tsx
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { PageHeader } from "../PageHeader";

describe("PageHeader", () => {
  it("renders title", () => {
    render(<PageHeader title="AI Chat" />);
    expect(screen.getByRole("heading", { name: "AI Chat" })).toBeInTheDocument();
  });

  it("renders description when provided", () => {
    render(<PageHeader title="Settings" description="Configure your AI" />);
    expect(screen.getByText("Configure your AI")).toBeInTheDocument();
  });

  it("renders children as actions", () => {
    render(
      <PageHeader title="Chat">
        <button type="button">New Chat</button>
      </PageHeader>
    );
    expect(screen.getByRole("button", { name: "New Chat" })).toBeInTheDocument();
  });

  it("renders without description gracefully", () => {
    const { container } = render(<PageHeader title="Memory" />);
    expect(container.querySelector("p")).not.toBeInTheDocument();
  });
});
```

### Step 2: Run test to verify it fails

```bash
bun run test src/components/layout/__tests__/PageHeader.test.tsx
```

Expected: FAIL — `Cannot find module '../PageHeader'`

### Step 3: Implement `PageHeader`

Create `src/components/layout/PageHeader.tsx`:

```tsx
import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

interface PageHeaderProps {
  title: string;
  description?: string;
  children?: ReactNode;
  className?: string;
}

export function PageHeader({ title, description, children, className }: PageHeaderProps) {
  return (
    <div className={cn("flex items-start justify-between gap-4 pb-6", className)}>
      <div className="space-y-1">
        <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
        {description && (
          <p className="text-sm text-muted-foreground">{description}</p>
        )}
      </div>
      {children && (
        <div className="flex shrink-0 items-center gap-2">{children}</div>
      )}
    </div>
  );
}
```

### Step 4: Run test to verify it passes

```bash
bun run test src/components/layout/__tests__/PageHeader.test.tsx
```

Expected: PASS — 4 tests

### Step 5: Commit

```bash
git add src/components/layout/PageHeader.tsx src/components/layout/__tests__/PageHeader.test.tsx
git commit -m "feat(layout): add PageHeader component for consistent page-level headings"
```

---

## Task 3: Create `ContextPanel` and `contextPanelStore`

**Goal:** A right-panel component (xl+) that pages can populate with contextual content.

**Files:**
- Create: `src/stores/contextPanelStore.ts`
- Create: `src/components/layout/ContextPanel.tsx`
- Create: `src/stores/__tests__/contextPanelStore.test.ts`

### Step 1: Write the failing store test

Create `src/stores/__tests__/contextPanelStore.test.ts`:

```ts
import { describe, expect, it, beforeEach } from "vitest";
import { act } from "react";
import { useContextPanelStore } from "../contextPanelStore";

describe("contextPanelStore", () => {
  beforeEach(() => {
    act(() => {
      useContextPanelStore.getState().clearContent();
    });
  });

  it("starts with null content", () => {
    expect(useContextPanelStore.getState().content).toBeNull();
  });

  it("setContent stores a ReactNode", () => {
    const node = <div>hello</div>;
    act(() => {
      useContextPanelStore.getState().setContent(node);
    });
    expect(useContextPanelStore.getState().content).toBe(node);
  });

  it("clearContent resets to null", () => {
    act(() => {
      useContextPanelStore.getState().setContent(<div>hello</div>);
      useContextPanelStore.getState().clearContent();
    });
    expect(useContextPanelStore.getState().content).toBeNull();
  });
});
```

### Step 2: Run to verify it fails

```bash
bun run test src/stores/__tests__/contextPanelStore.test.ts
```

Expected: FAIL — module not found

### Step 3: Implement `contextPanelStore`

Create `src/stores/contextPanelStore.ts`:

```ts
import type { ReactNode } from "react";
import { create } from "zustand";

interface ContextPanelState {
  content: ReactNode | null;
  setContent: (content: ReactNode) => void;
  clearContent: () => void;
}

export const useContextPanelStore = create<ContextPanelState>((set) => ({
  content: null,
  setContent: (content) => set({ content }),
  clearContent: () => set({ content: null }),
}));
```

### Step 4: Run to verify store tests pass

```bash
bun run test src/stores/__tests__/contextPanelStore.test.ts
```

Expected: PASS — 3 tests

### Step 5: Implement `ContextPanel` component

Create `src/components/layout/ContextPanel.tsx`:

```tsx
import { Brain, Cpu, Wifi, WifiOff } from "@/lib/icons";
import { useContextPanelStore } from "@/stores/contextPanelStore";
import { useGatewayStore } from "@/stores/gatewayStore";
import { useLLMStore } from "@/stores/llm";
import { cn } from "@/lib/utils";

export function ContextPanel() {
  const content = useContextPanelStore((s) => s.content);
  const { config } = useLLMStore();
  const isConnected = useGatewayStore((s) => s.isConnected);

  return (
    <div className="flex h-full flex-col gap-4 overflow-y-auto p-4">
      {/* Page-provided content takes priority */}
      {content ? (
        <div>{content}</div>
      ) : (
        <DefaultContextContent config={config} isConnected={isConnected} />
      )}
    </div>
  );
}

interface DefaultContentProps {
  config: { providerId?: string; modelId?: string } | null;
  isConnected: boolean;
}

function DefaultContextContent({ config, isConnected }: DefaultContentProps) {
  return (
    <div className="space-y-4">
      <div>
        <h2 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Active Model
        </h2>
        <div className="rounded-lg border border-border bg-card p-3">
          <div className="flex items-center gap-2">
            <Cpu className="size-4 text-primary" aria-hidden />
            <div className="min-w-0">
              <p className="truncate text-sm font-medium">
                {config?.modelId ?? "No model selected"}
              </p>
              <p className="truncate text-xs text-muted-foreground">
                {config?.providerId ?? "Configure in Settings"}
              </p>
            </div>
          </div>
        </div>
      </div>

      <div>
        <h2 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Gateway
        </h2>
        <div className="rounded-lg border border-border bg-card p-3">
          <div className="flex items-center gap-2">
            {isConnected ? (
              <Wifi className="size-4 text-green-600" aria-hidden />
            ) : (
              <WifiOff className="size-4 text-muted-foreground" aria-hidden />
            )}
            <span className="text-sm font-medium">
              {isConnected ? "Connected" : "Disconnected"}
            </span>
          </div>
        </div>
      </div>

      <div>
        <h2 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Memory
        </h2>
        <div className="rounded-lg border border-border bg-card p-3">
          <div className="flex items-center gap-2">
            <Brain className="size-4 text-primary" aria-hidden />
            <span className="text-sm text-muted-foreground">
              Available in Memory tab
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
```

### Step 6: Run all tests to confirm no regressions

```bash
bun run test
```

Expected: All existing tests pass + 3 new store tests pass.

### Step 7: Commit

```bash
git add src/stores/contextPanelStore.ts src/stores/__tests__/contextPanelStore.test.ts src/components/layout/ContextPanel.tsx
git commit -m "feat(layout): add ContextPanel and contextPanelStore for xl+ right panel content"
```

---

## Task 4: Remove Global Topbar; Add ContextPanel to Root

**Goal:** Remove the 56px topbar from `__root.tsx`, move `GatewayStatus` to the sidebar footer, and mount `ContextPanel` in the xl right slot.

**Files:**
- Modify: `src/routes/__root.tsx`
- Modify: `src/components/ui/sidebar.tsx`
- Modify: `src/constants/sidebar.ts`

### Step 1: Check `sidebar.ts` constants

```bash
cat src/constants/sidebar.ts
```

This shows the `SIDEBAR_BOTTOM_ITEMS` (currently just Settings).

### Step 2: Modify `__root.tsx`

In `src/routes/__root.tsx`, make these targeted changes:

**Remove** the entire topbar `<div>` block:
```tsx
{/* Remove this entire block: */}
<div
  data-tauri-drag-region
  className="flex h-14 shrink-0 items-center justify-between border-b border-border px-4"
>
  ...
</div>
```

**Change** the main content div padding (remove `pt-6` since no topbar offset needed):
```tsx
{/* Before: */}
<div className="flex-1 overflow-auto p-4 pb-20 pt-6 md:p-6 md:pb-6">

{/* After: */}
<div className="flex-1 overflow-auto p-4 pb-20 md:p-6 md:pb-6">
```

**Replace** the empty `<aside>` with `ContextPanel`:
```tsx
{/* Before: */}
<aside className="hidden border-l border-border xl:flex xl:w-80 xl:flex-col">
  {/* Future: contextual panel content */}
</aside>

{/* After: */}
<aside className="hidden border-l border-border xl:flex xl:w-80 xl:flex-col">
  <ContextPanel />
</aside>
```

**Add** the import at the top:
```tsx
import { ContextPanel } from "@/components/layout/ContextPanel";
```

**Remove** the `GatewayStatus` import from `__root.tsx` (it moves to sidebar).

### Step 3: Move `GatewayStatus` to sidebar footer

In `src/components/ui/sidebar.tsx`, add a status row at the bottom above the `SIDEBAR_BOTTOM_ITEMS`:

```tsx
// Add import:
import { GatewayStatus } from "@/components/ui/gateway-status";

// In the <aside> block, between SidebarNav (main) and SidebarNav (bottom):
{expanded && (
  <div className="border-t border-sidebar-border px-3 py-2">
    <GatewayStatus />
  </div>
)}
```

### Step 4: Add drag region back (Tauri needs it for frameless window)

Since the topbar was the only `data-tauri-drag-region`, the sidebar header already has none. Add drag region to the sidebar header:

In `src/components/ui/sidebar/sidebar-header.tsx`, add `data-tauri-drag-region` to the outer `<div>`:

```tsx
<div
  data-tauri-drag-region
  className={cn(
    "flex h-14 items-center gap-2 border-b border-sidebar-border px-3",
    ...
  )}
>
```

### Step 5: Run the app and verify

```bash
bun run dev
```

- No topbar visible
- Sidebar shows GatewayStatus above Settings
- Right panel shows ContextPanel on xl screens
- Window drag still works (drag from sidebar header)

### Step 6: Run tests

```bash
bun run test
```

Expected: All 33 tests pass.

### Step 7: Commit

```bash
git add src/routes/__root.tsx src/components/ui/sidebar.tsx src/components/ui/sidebar/sidebar-header.tsx
git commit -m "feat(layout): remove global topbar, mount ContextPanel in xl panel, move GatewayStatus to sidebar"
```

---

## Task 5: Redesign Home Page as Dashboard

**Goal:** Replace the boilerplate placeholder with a real dashboard: agent greeting, quick actions, system status.

**Files:**
- Modify: `src/routes/index.tsx`

### Step 1: Read the current file

```bash
cat src/routes/index.tsx
```

### Step 2: Rewrite `src/routes/index.tsx`

```tsx
import { createFileRoute, Link } from "@tanstack/react-router";
import { Brain, MessageSquare, Settings, Sparkles, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";
import { PageHeader } from "@/components/layout/PageHeader";
import { GatewayStatus } from "@/components/ui/gateway-status";
import { useIdentityStore } from "@/stores/identityStore";
import { useLLMStore } from "@/stores/llm";
import { cn } from "@/lib/utils";

export const Route = createFileRoute("/")(({ component: HomePage }));

function HomePage() {
  const { t } = useTranslation("common");
  const agentName = useIdentityStore((s) => s.agentName ?? "Agent");
  const { config } = useLLMStore();

  const hour = new Date().getHours();
  const greeting =
    hour < 12 ? "Good morning" : hour < 17 ? "Good afternoon" : "Good evening";

  const quickActions = [
    { icon: Sparkles, label: "New Chat", href: "/chat", description: "Start a conversation" },
    { icon: Brain, label: "Memory", href: "/memory", description: "Search agent memory" },
    { icon: MessageSquare, label: "Channels", href: "/channels", description: "View Telegram inbox" },
    { icon: Settings, label: "Settings", href: "/settings", description: "Configure providers" },
  ] as const;

  return (
    <div className="mx-auto w-full max-w-4xl space-y-8">
      {/* ── Greeting ── */}
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">
            {greeting}, {agentName}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {new Date().toLocaleDateString("en-US", {
              weekday: "long",
              month: "long",
              day: "numeric",
            })}
          </p>
        </div>
        <GatewayStatus />
      </div>

      {/* ── Quick actions grid ── */}
      <section aria-labelledby="quick-actions-heading">
        <h2
          id="quick-actions-heading"
          className="mb-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground"
        >
          Quick Actions
        </h2>
        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
          {quickActions.map(({ icon: Icon, label, href, description }) => (
            <Link
              key={href}
              to={href}
              className={cn(
                "group flex flex-col gap-3 rounded-xl border border-border bg-card p-4",
                "transition-all hover:border-primary/40 hover:shadow-sm",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              )}
            >
              <div className="flex size-9 items-center justify-center rounded-lg bg-primary/10">
                <Icon className="size-4 text-primary" aria-hidden />
              </div>
              <div>
                <p className="text-sm font-medium leading-none">{label}</p>
                <p className="mt-1 text-xs text-muted-foreground">{description}</p>
              </div>
            </Link>
          ))}
        </div>
      </section>

      {/* ── System status ── */}
      <section aria-labelledby="status-heading">
        <h2
          id="status-heading"
          className="mb-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground"
        >
          System Status
        </h2>
        <div className="rounded-xl border border-border bg-card p-4">
          <div className="flex flex-wrap gap-6">
            <StatusRow
              label="AI Provider"
              value={config?.providerId ?? "Not configured"}
              ok={!!config?.providerId}
            />
            <StatusRow
              label="Model"
              value={config?.modelId ?? "Not selected"}
              ok={!!config?.modelId}
            />
          </div>
        </div>
      </section>
    </div>
  );
}

function StatusRow({ label, value, ok }: { label: string; value: string; ok: boolean }) {
  return (
    <div className="flex items-center gap-2">
      <div
        className={cn(
          "size-2 rounded-full",
          ok ? "bg-green-500" : "bg-muted-foreground"
        )}
        aria-hidden
      />
      <span className="text-sm text-muted-foreground">{label}:</span>
      <span className="text-sm font-medium">{value}</span>
    </div>
  );
}
```

> **Note:** If `useIdentityStore` doesn't have `agentName` as a direct field, check `src/stores/identityStore.ts` for the correct selector — it may be `s.files[0]?.name` or similar. Adjust the selector accordingly.

### Step 3: Run the app and verify

```bash
bun run dev
```

Navigate to `/`. Should see: greeting, 4 quick action cards, system status row.

### Step 4: Run tests

```bash
bun run test
```

Expected: All tests pass.

### Step 5: Commit

```bash
git add src/routes/index.tsx
git commit -m "feat(home): replace placeholder with dashboard — greeting, quick actions, system status"
```

---

## Task 6: Create `SettingsNav` Component

**Goal:** VS Code–style vertical left-nav for settings that collapses to a `Select` on mobile.

**Files:**
- Create: `src/components/settings/SettingsNav.tsx`
- Create: `src/components/settings/__tests__/SettingsNav.test.tsx`

### Step 1: Write the failing test

Create `src/components/settings/__tests__/SettingsNav.test.tsx`:

```tsx
import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SettingsNav } from "../SettingsNav";

const sections = [
  { id: "ai", label: "AI Provider" },
  { id: "skills", label: "Skills" },
  { id: "app", label: "App Settings" },
];

describe("SettingsNav", () => {
  it("renders all section labels", () => {
    render(
      <SettingsNav sections={sections} activeSection="ai" onSectionChange={vi.fn()} />
    );
    expect(screen.getByText("AI Provider")).toBeInTheDocument();
    expect(screen.getByText("Skills")).toBeInTheDocument();
    expect(screen.getByText("App Settings")).toBeInTheDocument();
  });

  it("marks active section with aria-current", () => {
    render(
      <SettingsNav sections={sections} activeSection="skills" onSectionChange={vi.fn()} />
    );
    expect(screen.getByRole("button", { name: "Skills" })).toHaveAttribute(
      "aria-current",
      "page"
    );
  });

  it("calls onSectionChange when a section is clicked", () => {
    const onChange = vi.fn();
    render(
      <SettingsNav sections={sections} activeSection="ai" onSectionChange={onChange} />
    );
    fireEvent.click(screen.getByRole("button", { name: "Skills" }));
    expect(onChange).toHaveBeenCalledWith("skills");
  });
});
```

### Step 2: Run to verify it fails

```bash
bun run test src/components/settings/__tests__/SettingsNav.test.tsx
```

Expected: FAIL — module not found

### Step 3: Implement `SettingsNav`

Create `src/components/settings/SettingsNav.tsx`:

```tsx
import { cn } from "@/lib/utils";

export interface SettingsSection {
  id: string;
  label: string;
  description?: string;
}

interface SettingsNavProps {
  sections: SettingsSection[];
  activeSection: string;
  onSectionChange: (id: string) => void;
}

export function SettingsNav({ sections, activeSection, onSectionChange }: SettingsNavProps) {
  return (
    <>
      {/* Desktop: vertical nav list */}
      <nav
        aria-label="Settings sections"
        className="hidden flex-col gap-0.5 md:flex"
      >
        {sections.map((section) => {
          const isActive = section.id === activeSection;
          return (
            <button
              key={section.id}
              type="button"
              onClick={() => onSectionChange(section.id)}
              aria-current={isActive ? "page" : undefined}
              className={cn(
                "w-full rounded-lg px-3 py-2 text-left text-sm font-medium transition-colors",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                isActive
                  ? "bg-primary/10 text-primary"
                  : "text-foreground hover:bg-accent hover:text-accent-foreground"
              )}
            >
              {section.label}
            </button>
          );
        })}
      </nav>

      {/* Mobile: select dropdown */}
      <div className="md:hidden">
        <label htmlFor="settings-section-select" className="sr-only">
          Settings section
        </label>
        <select
          id="settings-section-select"
          value={activeSection}
          onChange={(e) => onSectionChange(e.target.value)}
          className={cn(
            "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm",
            "focus:outline-none focus:ring-2 focus:ring-ring"
          )}
        >
          {sections.map((section) => (
            <option key={section.id} value={section.id}>
              {section.label}
            </option>
          ))}
        </select>
      </div>
    </>
  );
}
```

### Step 4: Run tests

```bash
bun run test src/components/settings/__tests__/SettingsNav.test.tsx
```

Expected: PASS — 3 tests

### Step 5: Run all tests

```bash
bun run test
```

Expected: All pass.

### Step 6: Commit

```bash
git add src/components/settings/SettingsNav.tsx src/components/settings/__tests__/SettingsNav.test.tsx
git commit -m "feat(settings): add SettingsNav — VS Code-style vertical nav with mobile Select fallback"
```

---

## Task 7: Refactor Settings Page to Use `SettingsNav`

**Goal:** Replace the 9 horizontal `<Tabs>` in settings with the new `SettingsNav` left-nav layout.

**Files:**
- Modify: `src/routes/settings.tsx`

### Step 1: Read the current file in full

```bash
cat src/routes/settings.tsx
```

### Step 2: Rewrite `src/routes/settings.tsx`

```tsx
import { createFileRoute } from "@tanstack/react-router";
import { useState } from "react";

import { AdvancedSettingsTab } from "@/components/settings/AdvancedSettingsTab";
import { AISettingsTab } from "@/components/settings/AISettingsTab";
import { AppSettingsTab } from "@/components/settings/AppSettingsTab";
import { ChannelList } from "@/components/settings/ChannelList";
import { IdentityEditor } from "@/components/settings/IdentityEditor";
import { JobList } from "@/components/settings/JobList";
import { MobileSettings } from "@/components/settings/MobileSettings";
import { ModuleList } from "@/components/settings/ModuleList";
import { SettingsNav, type SettingsSection } from "@/components/settings/SettingsNav";
import { SkillsSettingsTab } from "@/components/settings/SkillsSettingsTab";
import { PageHeader } from "@/components/layout/PageHeader";
import { useHandleSettings } from "@/hooks/use-handle-settings";

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
});

const SETTINGS_SECTIONS: SettingsSection[] = [
  { id: "ai", label: "AI Provider", description: "Providers, models, API keys" },
  { id: "skills", label: "Skills", description: "Prompt templates" },
  { id: "app", label: "App Settings", description: "Theme, autostart, notifications" },
  { id: "identity", label: "Identity", description: "Agent name and personality" },
  { id: "scheduler", label: "Scheduler", description: "Scheduled jobs" },
  { id: "modules", label: "Modules", description: "Sidecar modules" },
  { id: "channels", label: "Channels", description: "Telegram and other channels" },
  { id: "mobile", label: "Mobile", description: "Mobile-specific settings" },
  { id: "advanced", label: "Advanced", description: "Developer and advanced options" },
];

function SettingsPage() {
  const [activeSection, setActiveSection] = useState("ai");

  const {
    theme,
    settings,
    autostartEnabled,
    isLoading,
    isSaving,
    handleUpdateSetting,
    handleAutostartChange,
    handleTrayVisibilityChange,
    handleNotificationChange,
  } = useHandleSettings();

  if (isLoading) {
    return (
      <div className="flex items-center justify-center p-12">
        <p className="text-muted-foreground">Loading settings…</p>
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="flex items-center justify-center p-12">
        <p className="text-destructive">Failed to load settings</p>
      </div>
    );
  }

  const activeLabel =
    SETTINGS_SECTIONS.find((s) => s.id === activeSection)?.label ?? "Settings";

  return (
    <div className="mx-auto w-full max-w-5xl">
      <PageHeader title="Settings" description="Configure your AI providers and application" />

      <div className="flex gap-8">
        {/* Left nav — 200px on desktop, full-width select on mobile */}
        <div className="w-full shrink-0 md:w-48">
          <SettingsNav
            sections={SETTINGS_SECTIONS}
            activeSection={activeSection}
            onSectionChange={setActiveSection}
          />
        </div>

        {/* Content area */}
        <div className="min-w-0 flex-1">
          <h2 className="mb-6 text-lg font-semibold">{activeLabel}</h2>

          {activeSection === "ai" && <AISettingsTab />}
          {activeSection === "skills" && <SkillsSettingsTab />}
          {activeSection === "app" && (
            <AppSettingsTab
              theme={theme}
              settings={settings}
              isSaving={isSaving}
              onUpdateSetting={handleUpdateSetting}
              onAutostartChange={handleAutostartChange}
              onTrayVisibilityChange={handleTrayVisibilityChange}
              onNotificationChange={handleNotificationChange}
              autostartEnabled={autostartEnabled}
            />
          )}
          {activeSection === "identity" && <IdentityEditor />}
          {activeSection === "scheduler" && <JobList />}
          {activeSection === "modules" && <ModuleList />}
          {activeSection === "channels" && <ChannelList />}
          {activeSection === "mobile" && <MobileSettings />}
          {activeSection === "advanced" && <AdvancedSettingsTab />}
        </div>
      </div>
    </div>
  );
}
```

### Step 3: Handle the `search` param (deep-linking from home page)

The home page uses `<Link to="/settings" search={{ tab: "ai" }}>`. The new settings page uses `activeSection` state. Update the route to read from search params:

```tsx
// In the route definition:
export const Route = createFileRoute("/settings")({
  validateSearch: (search) => ({
    tab: (search.tab as string) ?? "ai",
  }),
  component: SettingsPage,
});

// In the component, replace useState with:
const { tab } = Route.useSearch();
const [activeSection, setActiveSection] = useState(tab ?? "ai");
```

> **Note:** Check `src/routes/index.tsx` — any `<Link to="/settings" search={{ tab: "..." }}>` links use the `tab` param. The new page reads it on mount via `Route.useSearch()`.

### Step 4: Run app and verify

```bash
bun run dev
```

Navigate to `/settings`. Should see: left nav (desktop) or select (mobile), section content on right.

### Step 5: Commit

```bash
git add src/routes/settings.tsx
git commit -m "refactor(settings): replace 9-tab overflow with VS Code-style SettingsNav layout"
```

---

## Task 8: Rebuild Channels Page with Design Tokens

**Goal:** Replace every raw `neutral-*` class with design system tokens. Preserve all `invoke` calls, store bindings, and UX logic exactly.

**Files:**
- Modify: `src/routes/channels.tsx`

### Step 1: Read the current channels page

```bash
cat src/routes/channels.tsx
```

### Step 2: Rewrite `src/routes/channels.tsx`

Replace the entire file. The logic is **identical** — only classNames change:

```tsx
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
        <div className="flex min-w-0 flex-1 flex-col rounded-xl border border-border ml-4">
          {selectedChannel ? (
            <>
              <header className="shrink-0 border-b border-border px-4 py-3">
                <span className="text-sm font-semibold">#{selectedChannel}</span>
              </header>

              {/* Messages */}
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
```

### Step 3: Run tests

```bash
bun run test
```

Expected: All pass. (No behavioral change, only classNames.)

### Step 4: Run app and verify

```bash
bun run dev
```

Navigate to `/channels`. Should look consistent with the rest of the app — warm background, design-token borders, styled channel list, readable messages.

### Step 5: Commit

```bash
git add src/routes/channels.tsx
git commit -m "refactor(channels): rebuild with design system tokens — remove raw neutral-* classes"
```

---

## Task 9: Polish Chat Page — Warm Bubble Styling

**Goal:** Apply warm token styling to chat message bubbles and `PromptInput` wrapper. No behavioral changes.

**Files:**
- Modify: `src/routes/chat.tsx`

### Step 1: Find the Message rendering section

In `src/routes/chat.tsx`, find the `messages.map(...)` block and the `PromptInput` block.

### Step 2: Update message bubble classNames

Currently messages are rendered as:
```tsx
<Message key={message.id} from={message.role}>
  <MessageContent>
    <MessageResponse>...</MessageResponse>
  </MessageContent>
</Message>
```

Add `PageHeader` to the chat page and improve the empty state:

At the top of the `return` block in `ChatPage`, add:
```tsx
import { PageHeader } from "@/components/layout/PageHeader";
```

And above the `<Conversation>`, add:
```tsx
<PageHeader title="AI Chat" description={`${selectedModelData.name}`} />
```

For message bubbles — check if `Message` component accepts `className`. If it does, apply:
- User messages: `className="[&>div]:bg-primary/10 [&>div]:rounded-2xl [&>div]:rounded-br-sm"`
- Assistant messages: `className="[&>div]:bg-card [&>div]:border [&>div]:border-border [&>div]:rounded-2xl [&>div]:rounded-bl-sm [&>div]:shadow-sm"`

> **Note:** The `Message` component from AI SDK Elements may apply its own container styles. Check `src/components/ai-elements/message.tsx` to understand the DOM structure before applying wrappers.

### Step 3: Style the `PromptInput` wrapper

Find the `<div className="w-full px-4 pb-4">` wrapper around `PromptInput` and update:
```tsx
<div className="w-full px-4 pb-4">
  <div className="rounded-2xl border-2 border-border shadow-sm focus-within:border-primary/40 focus-within:shadow-md transition-all">
    <PromptInput ...>
```

### Step 4: Style suggestion pills

Find `<Suggestion ...>` components and add `className="rounded-full"` if the component supports it.

### Step 5: Run tests

```bash
bun run test
```

Expected: All pass.

### Step 6: Commit

```bash
git add src/routes/chat.tsx
git commit -m "style(chat): apply warm bubble tokens and rounded PromptInput wrapper"
```

---

## Task 10: Polish Memory Page

**Goal:** Pill-shaped tabs, Card-wrapped timeline. Light polish.

**Files:**
- Modify: `src/routes/memory.tsx`

### Step 1: Read the current file

```bash
cat src/routes/memory.tsx
```

### Step 2: Apply changes

In `src/routes/memory.tsx`:

1. Add `PageHeader` import and usage:
```tsx
import { PageHeader } from "@/components/layout/PageHeader";

// Inside MemoryPage(), before the Tabs:
<PageHeader
  title="Memory"
  description="Search the agent's semantic memory or browse daily journals."
/>
```

2. Remove the existing `<div>` heading block (it already has title + description).

3. Update `TabsList` to pill style:
```tsx
<TabsList className="shrink-0 rounded-full bg-muted p-1">
  <TabsTrigger value="search" className="rounded-full">Search</TabsTrigger>
  <TabsTrigger value="timeline" className="rounded-full">Daily Timeline</TabsTrigger>
</TabsList>
```

### Step 3: Commit

```bash
git add src/routes/memory.tsx
git commit -m "style(memory): add PageHeader and pill-shaped tabs"
```

---

## Task 11: Polish Mobile Nav

**Goal:** Add `rounded-t-2xl` + shadow lift to mobile nav container.

**Files:**
- Modify: `src/components/layout/MobileNav.tsx`

### Step 1: Update the `<nav>` className in `MobileNav.tsx`

Find:
```tsx
className={cn(
  "fixed bottom-0 left-0 right-0 z-50 md:hidden",
  "border-t border-border bg-background",
)}
```

Replace with:
```tsx
className={cn(
  "fixed bottom-0 left-0 right-0 z-50 md:hidden",
  "border-t border-border bg-background/95 backdrop-blur-sm",
  "shadow-[0_-4px_20px_rgba(0,0,0,0.06)]",
)}
```

### Step 2: Commit

```bash
git add src/components/layout/MobileNav.tsx
git commit -m "style(mobile-nav): add backdrop blur and lift shadow to bottom nav bar"
```

---

## Task 12: Theme Mode Switcher — Light / Dark / System

**Goal:** Add a compact 3-mode theme toggle to the sidebar footer so users can switch themes without digging into App Settings. The underlying `useTheme` store and `ThemeProvider` already work — this only adds a visible UI.

**Background:** `src/stores/theme.ts` has `setTheme("light" | "dark" | "system")`. `ThemeProvider` already listens for OS changes when `system` is selected. `THEME_OPTIONS` in `src/constants/settings.ts` exports all three values.

**Files:**
- Create: `src/components/ui/theme-toggle.tsx`
- Modify: `src/components/ui/sidebar.tsx`

### Step 1: Create `ThemeToggle` component

Create `src/components/ui/theme-toggle.tsx`:

```tsx
import { Monitor, Moon, Sun } from "lucide-react";
import { Tooltip } from "@/components/ui/tooltip";
import { useTheme } from "@/stores/theme";
import { cn } from "@/lib/utils";
import type { Theme } from "@/lib/tauri/settings/types";

const MODES: { value: Theme; label: string; icon: typeof Sun }[] = [
  { value: "light", label: "Light", icon: Sun },
  { value: "dark", label: "Dark", icon: Moon },
  { value: "system", label: "System (OS)", icon: Monitor },
];

interface ThemeToggleProps {
  /** Show icon-only (collapsed sidebar) or icons + labels */
  compact?: boolean;
}

export function ThemeToggle({ compact = false }: ThemeToggleProps) {
  const { theme, setTheme } = useTheme();

  if (compact) {
    // Cycle through modes on single click when collapsed
    const currentIndex = MODES.findIndex((m) => m.value === theme);
    const next = MODES[(currentIndex + 1) % MODES.length];
    const Current = MODES[currentIndex]?.icon ?? Sun;

    return (
      <Tooltip content={`Theme: ${MODES[currentIndex]?.label ?? "System"} — click to change`}>
        <button
          type="button"
          onClick={() => setTheme(next.value)}
          className={cn(
            "flex min-h-[44px] min-w-[44px] items-center justify-center rounded-lg",
            "text-sidebar-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
            "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
          )}
          aria-label={`Current theme: ${MODES[currentIndex]?.label}. Click to switch to ${next.label}`}
        >
          <Current className="size-4" aria-hidden />
        </button>
      </Tooltip>
    );
  }

  // Expanded: 3-button segmented control
  return (
    <div
      className="flex rounded-lg border border-sidebar-border bg-sidebar p-0.5"
      role="group"
      aria-label="Theme mode"
    >
      {MODES.map(({ value, label, icon: Icon }) => (
        <Tooltip key={value} content={label}>
          <button
            type="button"
            onClick={() => setTheme(value)}
            aria-label={label}
            aria-pressed={theme === value}
            className={cn(
              "flex flex-1 items-center justify-center rounded-md py-1.5 text-xs transition-colors",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              theme === value
                ? "bg-background text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground",
            )}
          >
            <Icon className="size-3.5" aria-hidden />
          </button>
        </Tooltip>
      ))}
    </div>
  );
}
```

### Step 2: Add `ThemeToggle` to sidebar footer

In `src/components/ui/sidebar.tsx`, add `ThemeToggle` to the sidebar footer area:

```tsx
// Add import:
import { ThemeToggle } from "@/components/ui/theme-toggle";

// In the <aside> block, after GatewayStatus and before SidebarNav (bottom):
<div className="border-t border-sidebar-border px-3 py-2 space-y-2">
  {expanded && (
    <div className="px-1">
      <ThemeToggle compact={false} />
    </div>
  )}
  {!expanded && (
    <div className="flex justify-center">
      <ThemeToggle compact={true} />
    </div>
  )}
  {expanded && (
    <div className="px-1">
      <GatewayStatus />
    </div>
  )}
</div>
```

### Step 3: Run the app and test all three modes

```bash
bun run dev
```

1. Click **Sun** → page switches to light mode
2. Click **Moon** → page switches to dark mode
3. Click **Monitor** → page follows OS preference
4. When collapsed, single icon cycles through modes on click

### Step 4: Verify ThemeProvider still reacts to OS changes

Set mode to **System**, then change OS theme (Settings > Appearance) — the app should follow without a reload.

### Step 5: Run tests

```bash
bun run test
```

Expected: All pass.

### Step 6: Commit

```bash
git add src/components/ui/theme-toggle.tsx src/components/ui/sidebar.tsx
git commit -m "feat(theme): add ThemeToggle to sidebar footer — light/dark/system modes"
```

---

## Task 13: Format, Test, and Final Verification

**Goal:** Clean up all modified files, run full test suite, verify app visually.

### Step 1: Auto-format all changed files

```bash
bunx ultracite fix
```

Expected: No output (or auto-fixed formatting). If Biome flags issues, fix them.

### Step 2: Run backend tests (ensure no regressions)

```bash
cd src-tauri && cargo test --lib
```

Expected: All backend tests pass (419 tests from last run).

### Step 3: Run frontend tests

```bash
bun run test
```

Expected: All tests pass. Count should be ≥ 33 (30 original + 3 new contextPanelStore + 4 PageHeader + 3 SettingsNav = 40 total).

### Step 4: Run cargo check on backend (no changes but verify no broken imports)

```bash
cd src-tauri && cargo check
```

Expected: Clean (one pre-existing `build.codegen-units` warning in `.cargo/config.toml` is OK).

### Step 5: Manual smoke test checklist

```
[ ] Home page: greeting shows, 4 quick action cards, status row
[ ] Home page → click "New Chat": navigates to /chat
[ ] Home page → click "Settings": navigates to /settings with AI Provider section active
[ ] Chat page: no topbar visible, PageHeader shows model name, messages render
[ ] Chat page: send a message (if API key configured) — verify streaming still works
[ ] Settings: left nav visible on desktop, select visible on mobile (<768px)
[ ] Settings: click each of the 9 sections — content renders correctly
[ ] Channels: channel list styled correctly, messages visible, send reply works
[ ] Memory: pill tabs, both Search and Timeline tabs work
[ ] Sidebar: GatewayStatus visible above Settings in footer
[ ] Sidebar collapsed (toggle): all icons show tooltips on hover
[ ] xl+ screen (>1280px): ContextPanel shows in right panel
[ ] Dark mode: toggle dark mode, verify all pages look correct
[ ] Mobile (<768px): MobileNav visible, shadow lift visible, all 4 nav items work
```

### Step 6: Final commit

```bash
git add -u
bunx ultracite fix
git add -u
git commit -m "chore: final format pass — UI/UX warm minimal redesign complete"
```

---

## Execution Order Summary

```
Task 1  → Parametric accent tokens            (10 min)
Task 2  → PageHeader component + tests        (15 min)
Task 3  → ContextPanel + contextPanelStore    (20 min)
Task 4  → Remove topbar, mount ContextPanel   (20 min)
Task 5  → Dashboard home page                 (25 min)
Task 6  → SettingsNav component + tests       (20 min)
Task 7  → Settings page refactor              (20 min)
Task 8  → Channels rebuild                    (25 min)
Task 9  → Chat page polish                    (15 min)
Task 10 → Memory page polish                  (10 min)
Task 11 → MobileNav polish                    (5 min)
Task 12 → Theme toggle — Light/Dark/System    (20 min)
Task 13 → Format, test, verify                (20 min)
─────────────────────────────────────────────
Total estimate:                               ~4 hours
```

All tasks are **independently committable**. If any task is blocked, skip it and come back — there are no hard sequential dependencies except Task 4 depends on Task 3.
