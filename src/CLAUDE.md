# Frontend Code Standards (src/)

This directory contains the React frontend for the application, built with TypeScript, Vite, TanStack Router, and Tailwind CSS 4.

## Tech Stack

- **React 19** - Latest React with function components
- **TypeScript 5** - Type-safe JavaScript
- **Vite** - Fast build tool and dev server
- **TanStack Router** - File-based routing with type safety
- **Zustand** - Lightweight state management
- **Tailwind CSS 4** - Utility-first CSS framework
- **Base UI (@base-ui/react)** - Unstyled React components (accessibility-focused)
- **AI SDK Elements** - Always useAI-native components from Vercel AI SDK
- **CodeMirror 6** - SQL editor with syntax highlighting
- **ReactFlow** - Interactive graph visualization

## Quick Reference

```bash
# Development
bun run dev                # Start dev server with route watching
bun run watch-routes       # Watch routes for changes

# Build & Check
bun run build              # Generate routes, type-check, and build
bun run check              # Type-check only
bun run lint               # Ultracite/Biome lint check
bun run format             # Auto-fix with Ultracite/Biome

# Testing
bun run test               # Run tests
bun run test:watch         # Watch mode
bun run test:coverage      # With coverage
bun run test:ui            # Vitest UI
```

## UI Component Libraries

### Base UI (shadcn-style Components)

The project uses **Base UI** (`@base-ui/react`) as the foundation for unstyled, accessible components. These are wrapped with Tailwind CSS styling in `src/components/ui/` following the shadcn/ui pattern.

**Available Components:**

- `Button` - Button with variants (default, ghost, outline, destructive, link)
- `Input` - Text input with focus states
- `Textarea` - Multi-line text input
- `Select` - Dropdown select with search
- `Switch` - Toggle switch
- `Tooltip` - Hover tooltip
- `Dialog` - Modal dialogs
- `AlertDialog` - Alert dialogs with actions
- `Dropdown Menu` - Context menus and dropdowns
- `Context Menu` - Right-click context menus
- `Tabs` - Tabbed content
- `Table` - Data tables with sorting
- `Badge` - Status badges and labels
- `Label` - Form labels
- `Hover Card` - Hover-triggered cards
- `Collapsible` - Expand/collapse content
- `Sidebar` - Collapsible sidebar components

**Usage Pattern:**

```tsx
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export function MyForm() {
  return (
    <form>
      <Input placeholder="Enter text..." />
      <Button variant="default">Submit</Button>
      <Button variant="ghost">Cancel</Button>
      <Button variant="outline" size="sm">
        Small
      </Button>
    </form>
  );
}
```

**Component Variants:**
Components use `class-variance-authority` (cva) for type-safe variants:

```tsx
// Button variants
<Button variant="default">Primary</Button>
<Button variant="ghost">Ghost</Button>
<Button variant="outline">Outline</Button>
<Button variant="destructive">Delete</Button>
<Button variant="link">Link</Button>

// Button sizes
<Button size="sm">Small</Button>
<Button size="default">Default</Button>
<Button size="lg">Large</Button>
<Button size="icon">Icon</Button>
```

### AI SDK Elements

AI SDK Elements provides pre-built components for AI-native applications. These are adapted for Tauri + React in `src/components/ai-elements/`.

**Documentation:** https://ai-sdk.dev/elements

**Available Components:**

#### Conversation Components

- `Conversation` - Scrollable message container with auto-scroll
- `ConversationContent` - Container for messages with scroll-on-update
- `ConversationEmptyState` - Empty state when no messages exist
- `ConversationScrollButton` - Scroll-to-bottom button

```tsx
import {
  Conversation,
  ConversationContent,
  ConversationEmptyState,
} from "@/components/ai-elements";

export function ChatInterface() {
  return (
    <Conversation className="h-full">
      {messages.length === 0 ? (
        <ConversationEmptyState
          title="Start a conversation"
          description="Ask anything about your database"
          icon={<BotIcon />}
        />
      ) : (
        <ConversationContent>
          {messages.map((msg) => (
            <Message key={msg.id} {...msg} />
          ))}
        </ConversationContent>
      )}
      <ConversationScrollButton />
    </Conversation>
  );
}
```

#### PromptInput Components

- `PromptInput` - Auto-resizing textarea with submit button
- `PromptInputTextarea` - Textarea for custom compositions
- `PromptInputSubmit` - Submit button with loading states

```tsx
import { PromptInput } from "@/components/ai-elements";

export function ChatInput() {
  const [value, setValue] = useState("");

  return (
    <PromptInput
      value={value}
      onChange={setValue}
      onSubmit={(message) => sendMessage(message.text)}
      placeholder="Ask about your database..."
      isLoading={isStreaming}
      minHeight="56px"
      maxHeight="200px"
    />
  );
}
```

#### CodeBlock Components

- `CodeBlock` - Syntax-highlighted code display
- `CodeBlockCopyButton` - Copy-to-clipboard button

```tsx
import { CodeBlock, CodeBlockCopyButton } from "@/components/ai-elements";

export function SQLDisplay({ sql }: { sql: string }) {
  return (
    <CodeBlock language="sql" code={sql}>
      <CodeBlockCopyButton />
    </CodeBlock>
  );
}
```

#### Artifact Components

- `Artifact` - Container for AI-generated content
- `ArtifactHeader` - Header with title and description
- `ArtifactTitle` - Title element
- `ArtifactDescription` - Description element
- `ArtifactActions` - Container for action buttons
- `ArtifactAction` - Individual action button
- `ArtifactClose` - Close button
- `ArtifactContent` - Main content area

```tsx
import {
  Artifact,
  ArtifactHeader,
  ArtifactTitle,
  ArtifactDescription,
  ArtifactActions,
  ArtifactAction,
  ArtifactContent,
} from "@/components/ai-elements";

export function GeneratedContent() {
  return (
    <Artifact>
      <ArtifactHeader>
        <ArtifactTitle>Generated Query</ArtifactTitle>
        <ArtifactDescription>AI-generated SQL query</ArtifactDescription>
      </ArtifactHeader>
      <ArtifactActions>
        <ArtifactAction onClick={runQuery}>Run</ArtifactAction>
        <ArtifactAction onClick={copyQuery}>Copy</ArtifactAction>
      </ArtifactActions>
      <ArtifactContent>
        <pre>{generatedSQL}</pre>
      </ArtifactContent>
    </Artifact>
  );
}
```

**Import Pattern:**

```tsx
// Import all components from the index
import {
  Conversation,
  PromptInput,
  CodeBlock,
  Artifact,
} from "@/components/ai-elements";

// Or import specific components
import { PromptInput } from "@/components/ai-elements/prompt-input";
```

## Code Style

### TypeScript & React

- Use **function components** with hooks (no class components)
- Use **explicit types** for props and return values
- Prefer **type inference** when types are obvious
- Use `const` assertions (`as const`) for immutable literals
- Avoid `any` - use `unknown` for genuinely unknown types

```tsx
// Good
interface TableProps {
  tableName: string;
  columns: Column[];
  onColumnClick?: (column: Column) => void;
}

export function TableList({ tableName, columns, onColumnClick }: TableProps) {
  // ...
}
```

### TanStack Router

- Routes are defined in `src/routes/` with file-based naming
- Use `createFileRoute` for typed route parameters
- Implement route loaders for data fetching
- Use typed navigation with `useNavigate()`

```tsx
// routes/workspace.$id.table.$table.tsx
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/workspace/$id/table/$table")({
  component: TableDetailPage,
});
```

### State Management (Zustand)

- Stores are defined in `src/stores/`
- Use TypeScript with typed selectors
- Keep stores focused and composable
- Use actions for state mutations

```tsx
// stores/schemaStore.ts
interface SchemaStore {
  tables: Map<string, Table>;
  getTable: (id: string) => Table | undefined;
  setTables: (tables: Table[]) => void;
}

export const useSchemaStore = create<SchemaStore>((set, get) => ({
  tables: new Map(),
  getTable: (id) => get().tables.get(id),
  setTables: (tables) => set({ tables: new Map(tables.map((t) => [t.id, t])) }),
}));
```

### Component Patterns

- Co-locate related components and hooks
- Extract reusable logic into custom hooks
- Use compound component pattern for complex UIs
- Keep components small and focused
- Use AI SDK Elements for chat interfaces
- Use Base UI components for forms and dialogs

```tsx
// Good: Focused component using UI libraries
export function SchemaTree({ tables, onTableSelect }: SchemaTreeProps) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const { query } = useSchemaSearch();

  return (
    <Sidebar>
      {filteredTables.map((table) => (
        <SidebarMenuItem key={table.id}>{table.name}</SidebarMenuItem>
      ))}
    </Sidebar>
  );
}
```

### Tailwind CSS 4

- Use utility classes over custom CSS
- Prefer semantic variants (`hover:`, `focus:`, `aria-`)
- Use `@layer` directives for custom styles
- Avoid arbitrary values when standard utilities exist
- Use `cn()` utility from `@/lib/utils` for conditional classes

```tsx
import { cn } from "@/lib/utils"

// Good
<button className="px-4 py-2 bg-blue-500 hover:bg-blue-600 rounded-md focus:ring-2 focus:ring-blue-300">
  Connect
</button>

// With cn utility for conditional classes
<div className={cn(
  "base-classes",
  isActive && "active-classes",
  isDisabled && "disabled-classes"
)}>
```

### Error Handling

- Handle errors at component boundaries
- Use Error Boundaries for React errors
- Display user-friendly error messages
- Log errors for debugging (dev mode only)

```tsx
// Good
try {
  await createWorkspace(connection);
} catch (error) {
  if (error instanceof WorkspaceError) {
    toast.error(error.message);
  } else {
    toast.error("Failed to create workspace");
  }
}
```

### Tauri IPC Commands

- Import commands from `@tauri-apps/api/commands` (auto-generated)
- Use type-safe command invocations
- Handle errors appropriately
- Show loading states during async operations

```tsx
import { invoke } from "@tauri-apps/api/core";

const tables = await invoke<Table[]>("get_schema_metadata_command", {
  workspaceId: id,
});
```

## File Organization

```
src/
├── assets/          # Static assets (images, fonts)
├── components/      # Reusable UI components
│   ├── ui/          # Base UI components (shadcn-style)
│   └── ai-elements/ # AI SDK Elements components
├── constants/       # App-wide constants
├── hooks/           # Custom React hooks
├── lib/             # Utility functions (cn, etc.)
├── routes/          # TanStack Router file-based routes
├── stores/          # Zustand state stores
├── styles/          # Global styles and Tailwind config
├── types/           # Shared TypeScript types
├── App.tsx          # Root app component
└── main.tsx         # Entry point
```

## Accessibility

- Base UI components provide built-in accessibility (ARIA, keyboard navigation)
- Use semantic HTML (`<button>`, `<nav>`, `<main>`)
- Provide meaningful `alt` text for images
- Use proper heading hierarchy
- Add labels to form inputs
- Support keyboard navigation
- Include ARIA attributes where needed
- Test with screen readers

## Performance

- Use `React.memo()` for expensive components
- Implement virtualization for long lists
- Lazy load routes and components
- Optimize re-renders with proper dependencies
- Use `useCallback`/`useMemo` judiciously
- AI SDK Elements handle auto-scrolling efficiently

## Testing

- Write tests alongside components (`__tests__/` directories)
- Test user behavior, not implementation details
- Use Vitest for unit testing
- Mock Tauri commands for frontend tests
- Test AI component interactions (prompts, submissions)

## Best Practices

### Combining Base UI and AI Elements

```tsx
import { Dialog } from "@/components/ui/dialog";
import { PromptInput } from "@/components/ai-elements";
import { Button } from "@/components/ui/button";

export function ChatDialog() {
  return (
    <Dialog open={isOpen} onOpenChange={setIsOpen}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>Database Chat</DialogTitle>
        </DialogHeader>
        <Conversation className="h-96">
          <ConversationContent>
            {messages.map((msg) => (
              <Message key={msg.id} {...msg} />
            ))}
          </ConversationContent>
        </Conversation>
        <PromptInput
          value={input}
          onChange={setInput}
          onSubmit={handleSubmit}
          isLoading={isStreaming}
        />
      </DialogContent>
    </Dialog>
  );
}
```

### Theming

- Use CSS variables for theme colors (`--primary`, `--background`, etc.)
- Components use semantic color tokens (`bg-primary`, `text-muted-foreground`)
- Supports light/dark mode via class-based theming
