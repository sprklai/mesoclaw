# Tauri AI Boilerplate - Project Code Standards

## Project Overview

A production-ready foundation for building AI-powered desktop applications. This boilerplate provides a complete stack with secure API key management, extensible AI integration, and a modern development experience.

**Tech Stack:**

- **Frontend**: React 19, TypeScript, Vite, TanStack Router, Zustand, Tailwind CSS 4
- **Backend**: Rust 2024, Tauri 2, Tokio async runtime
- **AI**: Multi-provider LLM support with streaming responses, secure keyring storage

**Directory-Specific Guidelines:**

- See `src/CLAUDE.md` for frontend React/TypeScript standards
- See `src-tauri/CLAUDE.md` for backend Rust standards

---

## Frontend: Ultracite Code Standards

This project uses **Ultracite**, a zero-config Biome preset that enforces strict code quality standards through automated formatting and linting.

## Quick Reference

- **Format code**: `bunx ultracite fix`
- **Check for issues**: `bunx ultracite check`
- **Diagnose setup**: `bunx ultracite doctor`

Biome (the underlying engine) provides extremely fast Rust-based linting and formatting. Most issues are automatically fixable.

---

## Core Principles

Write code that is **accessible, performant, type-safe, and maintainable**. Focus on clarity and explicit intent over brevity.

### Type Safety & Explicitness

- Use explicit types for function parameters and return values when they enhance clarity
- Prefer `unknown` over `any` when the type is genuinely unknown
- Use const assertions (`as const`) for immutable values and literal types
- Leverage TypeScript's type narrowing instead of type assertions
- Use meaningful variable names instead of magic numbers - extract constants with descriptive names

### Modern JavaScript/TypeScript

- Use arrow functions for callbacks and short functions
- Prefer `for...of` loops over `.forEach()` and indexed `for` loops
- Use optional chaining (`?.`) and nullish coalescing (`??`) for safer property access
- Prefer template literals over string concatenation
- Use destructuring for object and array assignments
- Use `const` by default, `let` only when reassignment is needed, never `var`

### Async & Promises

- Always `await` promises in async functions - don't forget to use the return value
- Use `async/await` syntax instead of promise chains for better readability
- Handle errors appropriately in async code with try-catch blocks
- Don't use async functions as Promise executors

### React & JSX

- Use function components over class components
- Call hooks at the top level only, never conditionally
- Specify all dependencies in hook dependency arrays correctly
- Use the `key` prop for elements in iterables (prefer unique IDs over array indices)
- Nest children between opening and closing tags instead of passing as props
- Don't define components inside other components
- Use semantic HTML and ARIA attributes for accessibility:
  - Provide meaningful alt text for images
  - Use proper heading hierarchy
  - Add labels for form inputs
  - Include keyboard event handlers alongside mouse events
  - Use semantic elements (`<button>`, `<nav>`, etc.) instead of divs with roles

### Error Handling & Debugging

- Remove `console.log`, `debugger`, and `alert` statements from production code
- Throw `Error` objects with descriptive messages, not strings or other values
- Use `try-catch` blocks meaningfully - don't catch errors just to rethrow them
- Prefer early returns over nested conditionals for error cases

### Code Organization

- Keep functions focused and under reasonable cognitive complexity limits
- Extract complex conditions into well-named boolean variables
- Use early returns to reduce nesting
- Prefer simple conditionals over nested ternary operators
- Group related code together and separate concerns

### Security

- Add `rel="noopener"` when using `target="_blank"` on links
- Avoid `dangerouslySetInnerHTML` unless absolutely necessary
- Don't use `eval()` or assign directly to `document.cookie`
- Validate and sanitize user input

### Performance

- Avoid spread syntax in accumulators within loops
- Use top-level regex literals instead of creating them in loops
- Prefer specific imports over namespace imports
- Avoid barrel files (index files that re-export everything)
- Use proper image components (e.g., Next.js `<Image>`) over `<img>` tags

### Framework-Specific Guidance

**Next.js:**

- Use Next.js `<Image>` component for images
- Use `next/head` or App Router metadata API for head elements
- Use Server Components for async data fetching instead of async Client Components

**React 19+:**

- Use ref as a prop instead of `React.forwardRef`

**Solid/Svelte/Vue/Qwik:**

- Use `class` and `for` attributes (not `className` or `htmlFor`)

---

## Testing

- Write assertions inside `it()` or `test()` blocks
- Avoid done callbacks in async tests - use async/await instead
- Don't use `.only` or `.skip` in committed code
- Keep test suites reasonably flat - avoid excessive `describe` nesting

## When Biome Can't Help

Biome's linter will catch most issues automatically. Focus your attention on:

1. **Business logic correctness** - Biome can't validate your algorithms
2. **Meaningful naming** - Use descriptive names for functions, variables, and types
3. **Architecture decisions** - Component structure, data flow, and API design
4. **Edge cases** - Handle boundary conditions and error states
5. **User experience** - Accessibility, performance, and usability considerations
6. **Documentation** - Add comments for complex logic, but prefer self-documenting code

---

Most formatting and common issues are automatically fixed by Biome. Run `bunx ultracite fix` before committing to ensure compliance.

---

## What's Included

### AI Integration Layer

**Multi-Provider Support:**
- Abstract `LLMProvider` trait for extensibility
- Built-in providers: OpenAI, Anthropic, Google AI, Groq, Vercel AI Gateway
- Streaming responses via Server-Sent Events (SSE)
- Secure API key storage in OS keyring (via `keyring` crate)

**Skills System:**
- Reusable AI prompts with parameters
- JSON-based skill definitions in `~/.tauri-ai-boilerplate/skills/`
- Dynamic skill discovery and loading
- Type-safe invocation from frontend

**Commands:**
- `list_llm_providers_command` - Get available providers
- `set_llm_provider_command` - Configure active provider
- `get_llm_provider_config_command` - Get current configuration
- `update_llm_api_key_command` - Update API key (stored in keyring)
- `list_skills_command` - Get all available skills
- `get_skill_command` - Load a specific skill
- `create_skill_command` - Create a new skill
- `execute_skill_command` - Run a skill with parameters

### Frontend Architecture

**UI Components:**
- Base UI components (shadcn-style) in `src/components/ui/`
- AI SDK Elements for chat interfaces
- Theme support (light/dark mode)
- Accessibility-first design

**State Management:**
- Zustand stores for global state
- `llmStore.ts` - AI provider configuration
- `skillsStore.ts` - Skills management

**Routing:**
- TanStack Router file-based routing
- `index.tsx` - Home page
- `settings.tsx` - App settings (AI config, skills)

### Backend Architecture

**AI Layer (`src-tauri/src/ai/`):**
- `llm_provider.rs` - Provider trait and implementations
- `providers/` - Individual provider implementations
- `skills.rs` - Skills system

**Services (`src-tauri/src/services/`):**
- `llm_service.rs` - LLM provider management
- `skills_service.rs` - Skills CRUD operations

**Commands (`src-tauri/src/commands/`):**
- `llm_commands.rs` - AI provider commands
- `skills_commands.rs` - Skills commands

## Project-Specific Standards

### Tauri IPC Commands

Commands are defined in `src-tauri/src/commands/` and invoked from the frontend using `invoke()` from `@tauri-apps/api/core`.

**Command Naming:** Use `*_command` suffix for all exported commands (e.g., `list_llm_providers_command`).

**Error Handling:** All commands return `Result<T, String>` for consistent error propagation to the frontend.

```rust
#[tauri::command]
pub async fn execute_skill_command(
    skill_name: String,
    parameters: HashMap<String, String>,
    state: State<'_, LlmService>,
) -> Result<String, String> {
    // Implementation
}
```

### AI Provider Pattern

All AI providers implement the `LLMProvider` trait defined in `src-tauri/src/ai/llm_provider.rs`. This allows for consistent behavior across OpenAI, Anthropic, Google, and other providers.

**Provider implementations:**

- `src-tauri/src/ai/providers/openai.rs` - OpenAI provider
- `src-tauri/src/ai/providers/anthropic.rs` - Anthropic provider
- `src-tauri/src/ai/providers/google_ai.rs` - Google AI provider
- `src-tauri/src/ai/providers/groq.rs` - Groq provider

**Security:** API keys are stored in the OS keyring via the `keyring` crate. Never stored in plain text.

### Skills System

Skills are JSON files in `~/.tauri-ai-boilerplate/skills/` with the following structure:

```json
{
  "name": "skill_name",
  "description": "What this skill does",
  "prompt": "System prompt with {{parameter}} placeholders",
  "parameters": [
    {
      "name": "parameter_name",
      "description": "What this parameter is for",
      "required": true
    }
  ]
}
```

**Usage:**
1. Create skills via UI or file system
2. Load skills with `list_skills_command` and `get_skill_command`
3. Execute with `execute_skill_command(skill_name, parameters)`
4. Skills automatically substitute parameters in prompts

### State Management

Frontend state is managed with Zustand stores in `src/stores/`:

- `llmStore.ts` - AI provider configuration and state
- `skillsStore.ts` - Skills management and caching

### File-Based Routing

Frontend routes are defined in `src/routes/` using TanStack Router file-based conventions:

- `index.tsx` - Home page with quick start
- `settings.tsx` - App settings (AI providers, skills management)

---

## Development Workflow

1. **Frontend changes**: Edit files in `src/`, run `bun run dev`
2. **Backend changes**: Edit files in `src-tauri/`, run `bun run tauri dev`
3. **Testing**: Run `bun run test` for frontend, `cargo test` for backend
4. **Building**: Run `bun run tauri build` for production builds

## Extending the Boilerplate

### Adding a New AI Provider

1. Create provider implementation in `src-tauri/src/ai/providers/`
2. Implement the `LLMProvider` trait
3. Add provider to `src-tauri/src/ai/llm_provider.rs` enum
4. Update frontend `llmStore.ts` with new provider name

### Creating Custom Commands

1. Define command in `src-tauri/src/commands/`
2. Add to `main.rs` invoke handlers
3. Create TypeScript types in `src/types/`
4. Invoke from frontend with `invoke<ReturnType>("command_name", { args })`

### Adding New Skills

Skills can be added via:
- UI in Settings → Skills tab
- Manually creating JSON files in `~/.tauri-ai-boilerplate/skills/`
- Programmatically via `create_skill_command`

## Key Documentation

- [README.md](../README.md) - Project overview and quick start
- [src/CLAUDE.md](../src/CLAUDE.md) - Frontend code standards
- [src-tauri/CLAUDE.md](../src-tauri/CLAUDE.md) - Backend code standards
