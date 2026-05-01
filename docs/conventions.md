# Coding Conventions (Detail)

Concise 1-liners live in `CLAUDE.md`. This file holds the rationale and full specs for
verbose items.

## Credential Key Naming

Use colon-separated namespacing for all credential keys stored via `CredentialStore`:

| Category | Pattern | Examples |
|----------|---------|---------|
| AI provider API keys | `api_key:{provider_id}` | `api_key:openai`, `api_key:tavily`, `api_key:brave` |
| Channel credentials | `channel:{channel_id}:{field}` | `channel:telegram:token`, `channel:slack:bot_token` |

**Never** use underscore-separated names like `tavily_api_key` — the colon scheme is
consistent, grep-friendly, and avoids collisions between namespaces.

## Native `<select>` in Dark Mode

Always use `bg-background text-foreground` classes on `<select>` elements in Svelte components.

**Why**: The `color-scheme: dark` declaration on `.dark` in `app.css` causes the browser
to render the native dropdown popup with a dark background. Without explicit `bg-background`,
the select field itself may appear transparent or mismatched, making option text unreadable
in dark mode.

**Never** use `bg-transparent` on selects — it breaks option visibility in dark mode even
when the popup inherits `color-scheme: dark` correctly.

## No Magic Numbers

Never hardcode tunable values (weights, thresholds, limits, timeouts, ratios, intervals,
sizes, retry counts, etc.) directly in business logic.

**Pattern**:
1. Define the value as a field on `AppConfig` (or a nested config struct) in `config/schema.rs`.
2. Set a sensible default via `impl Default`.
3. Read from the config struct at runtime — never use `const` or literal values in logic.

**Why**: Users must be able to tune these via `config.toml` without recompiling. Hardcoded
values hide performance knobs and make production debugging harder.

**Examples of values that must be in config**: search scoring weights, token limits,
rate-limit windows, batch sizes, cache TTLs, connection pool sizes, retry counts.
