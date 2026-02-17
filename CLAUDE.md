# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Tauri AI Boilerplate** is an AI-powered database comprehension desktop application built with Tauri 2. It helps developers quickly understand unfamiliar databases through AI-powered schema analysis, relationship inference, and query understanding.

- **Frontend**: React 19 + TypeScript + Vite + TanStack Router + Zustand + Tailwind CSS 4
- **Backend**: Rust 2024 + Tauri 2 + Diesel ORM + Tokio async runtime
- **Databases**: SQLite (complete), PostgreSQL, MySQL (95% complete), MongoDB (95% complete)
- **AI**: Multi-provider LLM support (OpenAI, Anthropic, Google AI, Groq, Vercel AI Gateway, Ollama)

## Quick Start

```bash
# Development (full stack)
bun run tauri dev              # Hot reload for frontend + backend
bun run tauri:dev:fast         # Fast dev mode (skip some checks)

# Frontend only
bun run dev                    # Vite dev server with route watching

# Backend only (in src-tauri/)
cargo run                      # Run backend with frontend in dist/
cargo check                    # Quick compile check
cargo clippy                   # Lint checks

# Testing
bun run test                   # Frontend tests
cd src-tauri && cargo test --lib   # Backend unit tests (120 tests)

# Build
bun run tauri build            # Production builds (macOS, Windows, Linux)

# Code quality
bunx ultracite fix             # Auto-format with Biome
bunx ultracite check           # Lint check
```

## Architecture

This project follows a **clean architecture** pattern with clear separation between layers:

```
┌─────────────────────────────────────────────────────────────┐
│                   Frontend (React/TypeScript)               │
│  TanStack Router • Zustand Stores • UI Components           │
└─────────────────────────────────────────────────────────────┘
                          ▼ Tauri IPC
┌─────────────────────────────────────────────────────────────┐
│                  Backend (Rust/Tauri)                       │
├─────────────────────────────────────────────────────────────┤
│  Commands Layer (src-tauri/src/commands/)                   │
│  ├─ database_commands.rs   - Workspace management          │
│  ├─ schema_commands.rs     - Schema introspection          │
│  ├─ explanation_commands.rs - AI explanations              │
│  └─ skill_commands.rs      - AI skill execution            │
├─────────────────────────────────────────────────────────────┤
│  Services Layer (src-tauri/src/services/)                   │
│  ├─ workspace_service.rs   - Workspace CRUD                │
│  ├─ introspection_service.rs - Schema analysis             │
│  ├─ credential_store.rs    - OS keyring integration        │
│  └─ settings.rs            - App settings persistence       │
├─────────────────────────────────────────────────────────────┤
│  Database Layer (src-tauri/src/database/)                   │
│  ├─ providers/             - DatabaseProvider trait         │
│  │   ├─ sqlite.rs          - SQLite implementation         │
│  │   ├─ postgres.rs        - PostgreSQL implementation     │
│  │   ├─ mysql.rs           - MySQL implementation          │
│  │   └─ mongodb.rs         - MongoDB implementation        │
│  └─ models/                - Diesel ORM models             │
├─────────────────────────────────────────────────────────────┤
│  AI Layer (src-tauri/src/ai/)                               │
│  ├─ llm_provider.rs        - LLMProvider trait             │
│  ├─ providers/             - Provider implementations      │
│  ├─ agents/                - AI agents (interpretation, etc)│
│  ├─ prompts/               - Prompt templates              │
│  └─ cache.rs               - Explanation cache (LRU)       │
├─────────────────────────────────────────────────────────────┤
│  Skills System (src-tauri/src/skills/)                      │
│  ├─ loader.rs              - Skill discovery and loading   │
│  ├─ selector.rs            - LLM-based skill selection     │
│  ├─ composer.rs            - Skill composition             │
│  └─ executor.rs            - Skill execution with tools    │
└─────────────────────────────────────────────────────────────┘
```

### Key Architectural Patterns

1. **Provider Pattern**: All database types implement `DatabaseProvider` trait for consistent behavior
2. **Agent Pattern**: AI agents (schema interpretation, relationship inference, query explanation) use multi-stage prompting
3. **Skill System**: Modular AI capabilities that can be enabled/disabled and composed together
4. **Async/Await**: Tokio runtime for async I/O in backend, React hooks for async UI state

## Common Development Tasks

### Adding a New Database Provider

1. Implement `DatabaseProvider` trait in `src-tauri/src/database/providers/`
2. Add connection configuration to `database/models/workspace.rs`
3. Add provider to `database/mod.rs` enum
4. Update frontend UI components to support new database type
5. Add integration tests in provider module

### Adding a New AI Skill

Skills are markdown files with YAML frontmatter in `~/.config/<skillsConfigDirName>/skills/`:

```yaml
---
id: my-skill
name: My Skill
description: What this skill does
category: understanding
input_schema:
  type: object
  properties:
    table_name:
      type: string
      description: Name of the table
---

You are an expert database analyst. Given table: {{table_name}}

Analyze the schema and provide insights...
```

Register in `src-tauri/src/skills/registry.rs` for built-in skills.

### Adding a New Tauri Command

1. Create command in `src-tauri/src/commands/`
2. Add to `main.rs` invoke handlers:
   ```rust
   .invoke_handler(tauri::generate_handler![
       your_new_command,
   ])
   ```
3. Frontend invocation:
   ```tsx
   import { invoke } from "@tauri-apps/api/core";
   const result = await invoke<ReturnType>("your_new_command", { args });
   ```

### Running Tests

**Backend (120 unit tests)**:
```bash
cd src-tauri
cargo test --lib                              # All tests
cargo test --lib database::providers::sqlite  # Specific module
cargo test --lib -- --nocapture               # With output
```

**Frontend**:
```bash
bun run test              # Run once
bun run test:watch        # Watch mode
bun run test:coverage     # Coverage report
bun run test:ui           # Vitest UI
```

### Database Migrations

```bash
cd src-tauri
diesel migration run      # Apply migrations
diesel migration revert   # Rollback last migration
diesel migration refresh  # Rebuild DB (destructive)
```

App database location: Tauri app-local data directory (`app_local_data_dir`), typically under your OS application data path.

## Important Conventions

### Backend (Rust)

- **Command naming**: All exported commands use `*_command` suffix (e.g., `get_schema_metadata_command`)
- **Error handling**: Commands return `Result<T, String>` for consistent error propagation
- **Async patterns**: Use `async fn` with `tokio::sync` primitives
- **Security**: API keys stored in OS keyring via `keyring` crate, sensitive data zeroized with `zeroize`
- **Testing**: Unit tests in same file with `#[cfg(test)]`

See `src-tauri/CLAUDE.md` for detailed Rust standards.

### Frontend (React/TypeScript)

- **Components**: Function components with hooks (no classes)
- **State**: Zustand stores in `src/stores/`
- **Routing**: TanStack Router file-based routing in `src/routes/`
- **UI Libraries**:
  - **Base UI** (`@base-ui/react`): Accessible components (Button, Dialog, etc.)
  - **AI SDK Elements**: Chat components (Conversation, PromptInput, Artifact)
- **Styling**: Tailwind CSS 4 utility classes with `cn()` helper
- **Code quality**: Ultracite (Biome) auto-formatting via `bunx ultracite fix`

See `src/CLAUDE.md` for detailed React/TypeScript standards.

## Key Files and Directories

```
├── src/                        # Frontend React application
│   ├── routes/                 # TanStack Router file-based routes
│   ├── stores/                 # Zustand state management
│   ├── components/
│   │   ├── ui/                 # Base UI components (shadcn-style)
│   │   └── ai-elements/        # AI SDK Elements components
│   └── CLAUDE.md               # Frontend code standards
├── src-tauri/                  # Backend Rust application
│   ├── src/
│   │   ├── commands/           # Tauri IPC commands
│   │   ├── services/           # Business logic services
│   │   ├── database/           # Database providers and models
│   │   ├── ai/                 # AI integration (providers, agents, prompts)
│   │   ├── skills/             # AI skill system
│   │   ├── lib.rs              # Library entry point (app setup)
│   │   └── main.rs             # Binary entry point
│   ├── migrations/             # Diesel database migrations
│   ├── Cargo.toml              # Rust dependencies
│   ├── tauri.conf.json         # Tauri configuration
│   └── CLAUDE.md               # Backend code standards
├── docs/                       # Phase documentation
│   ├── phases/                 # Detailed phase docs (1.1-4.5)
│   ├── features/               # Feature documentation (skills, chat)
│   ├── database/               # Database connection guides
│   └── plans/                  # Implementation plans (MongoDB, i18n)
├── .claude/CLAUDE.md           # Comprehensive project standards
├── package.json                # Frontend dependencies
└── README.md                   # Project overview and features
```

## Documentation Structure

This project has **layered documentation**:

1. **This file** (`CLAUDE.md`) - High-level orientation and common tasks
2. **`.claude/CLAUDE.md`** - Comprehensive project standards and patterns
3. **`src/CLAUDE.md`** - Frontend-specific React/TypeScript standards
4. **`src-tauri/CLAUDE.md`** - Backend-specific Rust standards
5. **`docs/phases/`** - Detailed phase documentation (Phases 1.1-4.5)
6. **`README.md`** - Project overview, features, and status

## Current Status

**Complete**:
- ✅ Phase 1: Backend Infrastructure (8 phases)
- ✅ Phase 2: AI Integration (8 phases)
- ✅ Phase 3: Frontend UI (8 phases)
- ✅ Phase 4: IPC Commands (5 phases, 2 remaining)
- ✅ SSH Tunnel Support (~95% complete)
- ✅ MongoDB Integration (~95% complete)
- ✅ AI Skill System (8 built-in skills)

**In Progress**:
- Phase 4.6-4.7: LLM commands and command registration

**Planned**:
- Supabase database integration
- Local LLM support (Ollama integration complete)
- Internationalization (i18n)

## Build & Release

```bash
# Check version status
./scripts/release.sh status

# Create release (choose one)
./scripts/release.sh patch   # 0.0.1 → 0.0.2
./scripts/release.sh minor   # 0.0.2 → 0.1.0
./scripts/release.sh major   # 0.1.0 → 1.0.0
```

This syncs versions, creates a release commit, and triggers GitHub Actions for cross-platform builds.

See `docs/RELEASING.md` for code signing setup.

## Getting Help

- **Phase docs**: `docs/phases/PHASE_*.md` - Implementation details for each phase
- **Feature docs**: `docs/features/*.md` - Skill system, chat functionality
- **Database guides**: `docs/database/*.md` - Connection methods, testing, MongoDB
- **README**: Project overview and feature list
- **Issue tracking**: Track active issues and analysis in `docs/` root
