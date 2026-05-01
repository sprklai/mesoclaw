# Style Guide

## Mermaid / Markdown Compatibility

Rules for diagrams in `docs/` and plan files (Mermaid 11.x + CommonMark).

### Line breaks
Use `<br>` not `<br/>` — Mermaid 11.x Langium parser rejects self-closing `<br/>`.

### Parentheses in node labels
Use `#40;` and `#41;` for `(` and `)` inside node labels — bare parentheses trigger
"Unsupported markdown: list". Does NOT apply to subgraph titles or sequence diagram
participants — use plain text or dashes there.

### Subgraph / node ID collision
Never use the same ID for a `subgraph` and a node inside it — Mermaid treats them as the
same entity and throws "Setting X as parent of X would create a cycle". Use distinct IDs,
e.g. `subgraph "Boot"` with node `BootEntry[...]` instead of `Boot[...]`.

### Numbered lists in node labels
Never use `1.`, `2.`, etc. in node label text (including after `<br>`) — Mermaid interprets
these as Markdown ordered list items. Use plain text, letters, or dashes instead.

### Directory trees
Use Unicode box-drawing characters (`├──`, `└──`, `│`) not ASCII `+--` and `|` — the `+`
is a valid Markdown list marker and triggers "unsupported list" warnings.

### Styling (nice-to-have)
For simple diagrams, add `style` or `classDef` directives for readability. Consistent palette:

| Color | Hex | Use |
|-------|-----|-----|
| Green | `#4CAF50` | done / success |
| Orange | `#FF9800` | in-progress |
| Blue | `#2196F3` | info / neutral |
| Gray | `#9E9E9E` | not-started |
| Red | `#F44336` | error / blocked |

Prefer `classDef` for reusable styles over per-node `style` directives. Don't clutter
complex diagrams with styling.

### Layout
Use `direction TB` or `direction LR` explicitly. Group related nodes with `subgraph`.
Add invisible edges (`~~>`) only if layout is unreadable without them.

## Frontend Style

- Native `<select>` in dark mode: use `bg-background text-foreground` — never `bg-transparent`.
  See `docs/conventions.md` for full rationale.
- Svelte 5: max 1 `$effect` per component; WS for real-time data, no polling.
- Tailwind v4: use `@theme inline {}` for custom color utilities.
