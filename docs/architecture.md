# MesoClaw Architecture

## Table of Contents

- [System Architecture](#system-architecture)
- [Data Flow](#data-flow)
- [Crate Dependency Graph](#crate-dependency-graph)
- [Project Structure](#project-structure)
- [Default Paths by OS](#default-paths-by-os)
- [Feature Flag Composition](#feature-flag-composition)
- [Trait-Driven Architecture](#trait-driven-architecture)
- [Credential System](#credential-system)
- [Identity / Soul System](#identity--soul-system)
- [Skills System](#skills-system)
- [User Profile + Progressive Learning](#user-profile--progressive-learning)
- [Gateway Routes](#gateway-routes)
- [Concurrency Rules](#concurrency-rules)
- [Lessons Learned from v1](#lessons-learned-from-v1)

---

## System Architecture

```mermaid
graph LR
    subgraph Clients["Clients"]
        Desktop[Desktop] & Mobile[Mobile] & CLI[CLI] & TUI[TUI] & Daemon[Daemon]
        Web["Frontend<br>Svelte 5"]
    end

    subgraph Core["mesoclaw-core"]
        BootEntry["boot.rs<br>init_services"]

        subgraph App["Application Layer"]
            Gateway["Gateway<br>axum :18981"]
            AI["AI Engine<br>rig-core"]
            DB["Database<br>rusqlite + sqlite-vec"]
        end

        subgraph Domain["Domain Layer"]
            Identity["Identity<br>SoulLoader"]
            Skills["Skills<br>SkillRegistry"]
            UserL["User Profile<br>UserLearner"]
            Channels["Channels"]
        end

        subgraph Support["Support Layer"]
            Tools["Agent Tools"]
            Security["Security"]
            Creds2["Credentials"]
            Config["Config"]
            EventBus["EventBus"]
        end
    end

    Desktop & Mobile & CLI & TUI & Daemon --> Gateway
    Web -->|HTTP/WS| Gateway

    BootEntry --> Gateway & DB & EventBus
    Gateway --> AI & DB
    Gateway --> Identity & Skills & UserL & Channels
    AI --> Tools & Security & DB
    AI --> Identity & Skills
```

## Data Flow

```mermaid
graph TB
    subgraph Clients
        D[Desktop] & M[Mobile] & C[CLI] & T[TUI] & F[Frontend]
    end

    subgraph "Gateway :18981"
        REST["REST<br>36 routes"]
        WS["WebSocket<br>/ws/chat"]
    end

    subgraph Backend
        AIL[AI Layer] & DBL[Database]
        IDL[Identity] & SKL[Skills] & USL[User]
    end

    D & M & C & T -->|HTTP| REST
    F -->|HTTP/WS| WS
    REST & WS --> AIL
    REST --> DBL & IDL & SKL & USL
    AIL --> DBL & IDL & SKL
```

## Crate Dependency Graph

```mermaid
graph TD
    desktop[mesoclaw-desktop] --> core[mesoclaw-core]
    mobile[mesoclaw-mobile] --> core
    cli[mesoclaw-cli]
    tui[mesoclaw-tui] --> core
    daemon[mesoclaw-daemon] --> core

    core --> axum["axum<br>#40;gateway#41;"]
    core --> rusqlite["rusqlite<br>#40;database#41;"]
    core --> rigcore["rig-core<br>#40;AI#41;"]
    core --> tokio["tokio<br>#40;async runtime#41;"]
    core --> keyring["keyring<br>#40;credentials#41;"]
    core --> sysinfo["sysinfo<br>#40;system info + processes#41;"]
    core --> ignore["ignore<br>#40;file search#41;"]
    core --> diffy["diffy<br>#40;patch/diff#41;"]
    core --> lru["lru<br>#40;embedding cache#41;"]
    core --> sqlitevec["sqlite-vec<br>#40;vector search#41;"]
    core --> serdeyaml["serde_yaml<br>#40;YAML frontmatter#41;"]
    core --> dashmap["dashmap<br>#40;concurrent tool registry#41;"]

    cli --> reqwest["reqwest<br>#40;HTTP client#41;"]
    cli --> tungstenite["tokio-tungstenite<br>#40;WS client#41;"]
    cli --> clap["clap<br>#40;arg parsing#41;"]
```

## Project Structure

```
mesoclaw/
├── Cargo.toml              # Workspace root (7 members)
├── CLAUDE.md               # AI assistant instructions
├── README.md               # Project documentation
├── scripts/
│   └── build.sh            # Cross-platform build script
├── docs/
│   ├── architecture.md     # This file
│   ├── phases.md           # Implementation phases
│   └── processes.md        # Process flow diagrams
├── plans/
│   ├── phase1_core_foundation.md  # Detailed implementation plan
│   └── migration_plan.md          # v1 → v2 migration strategy
├── tests/
│   ├── phase1_core_foundation.md  # Test plan + results
│   ├── phase2_ai_integration.md   # (planned)
│   └── ...
├── crates/
│   ├── mesoclaw-core/      # Shared library (NO Tauri dependency)
│   │   ├── src/
│   │   │   ├── lib.rs      # Module exports + Result<T> alias
│   │   │   ├── error.rs    # MesoError enum (23 variants, thiserror)
│   │   │   ├── boot.rs     # init_services() -> Services -> AppState, single boot entry point
│   │   │   ├── config/     # TOML config (schema + load/save + OS paths)
│   │   │   ├── db/         # rusqlite pool + WAL + migrations + spawn_blocking
│   │   │   ├── event_bus/  # EventBus trait + TokioBroadcastBus (12 events)
│   │   │   ├── memory/     # Memory trait + SqliteMemoryStore (FTS5 + vectors) + InMemoryStore
│   │   │   ├── credential/ # CredentialStore trait + InMemoryCredentialStore
│   │   │   ├── security/   # SecurityPolicy + AutonomyLevel + rate limiter + audit log
│   │   │   ├── tools/      # Tool trait + ToolRegistry (DashMap) + 9 tools (shell, file ops, web search, sysinfo, patch, process)
│   │   │   ├── ai/         # AI agent (rig-core), providers, session manager, tool adapter
│   │   │   ├── gateway/    # axum HTTP+WS gateway (36 routes, auth middleware, error mapping, MESO_VALIDATION)
│   │   │   ├── identity/   # SoulLoader + PromptComposer + defaults (SOUL/IDENTITY/USER.md)
│   │   │   ├── skills/     # SkillRegistry + bundled/user skills (markdown + YAML frontmatter)
│   │   │   ├── user/       # UserLearner + SQLite observations + privacy controls
│   │   │   ├── channels/   # Channel trait + implementations (Phase 8)
│   │   │   └── scheduler/  # Cron + scheduled tasks, feature-gated (Phase 8)
│   │   └── tests/          # Integration tests
│   ├── mesoclaw-desktop/   # Tauri 2 shell (desktop)
│   ├── mesoclaw-mobile/    # Tauri 2 shell (iOS + Android)
│   ├── mesoclaw-cli/       # clap CLI
│   ├── mesoclaw-tui/       # ratatui TUI
│   └── mesoclaw-daemon/    # Headless daemon (full gateway server)
└── web/                    # Svelte 5 frontend (Phase 7)
    ├── src/
    └── package.json
```

## Default Paths by OS

Resolved via `directories::ProjectDirs::from("com", "sprklai", "mesoclaw")`.

Source: `crates/mesoclaw-core/src/config/mod.rs`

| OS | Config Path | Data Dir / DB Path |
|---|---|---|
| **Linux** | `~/.config/mesoclaw/config.toml` | `~/.local/share/mesoclaw/mesoclaw.db` |
| **macOS** | `~/Library/Application Support/com.sprklai.mesoclaw/config.toml` | `~/Library/Application Support/com.sprklai.mesoclaw/mesoclaw.db` |
| **Windows** | `%APPDATA%\sprklai\mesoclaw\config\config.toml` | `%APPDATA%\sprklai\mesoclaw\data\mesoclaw.db` |

Override in `config.toml`:
```toml
data_dir = "/custom/data/path"        # overrides default data directory
db_path = "/custom/path/mesoclaw.db"  # overrides database file directly
```

## Feature Flag Composition

```mermaid
graph TD
    Daemon[mesoclaw-daemon binary] --> Default[default - no flags]
    Daemon --> Channels["--features channels"]
    Daemon --> Scheduler["--features scheduler"]
    Daemon --> Dashboard["--features web-dashboard"]

    Default --> CoreGW["mesoclaw-core<br>#40;gateway feature#41;"]
    CoreGW --> Axum[axum + tower-http]

    Channels --> CoreCH[mesoclaw-core/channels]
    Scheduler --> CoreSC[mesoclaw-core/scheduler]
    Dashboard --> CoreWD[mesoclaw-core/web-dashboard]
    CoreWD --> CoreGW
```

## Trait-Driven Architecture

All major subsystems are abstracted behind traits, allowing swappable implementations for testing, migration, and scaling.

```mermaid
graph TB
    subgraph "Trait Abstractions - mesoclaw-core"
        Memory["dyn Memory<br>SQLite now<br>PostgreSQL + pgvector later"]
        CredStore["dyn CredentialStore<br>Keyring now<br>Vault / cloud KMS later"]
        Channel["dyn Channel<br>openclaw-channels"]
        EvBus["dyn EventBus<br>tokio::broadcast now<br>NATS / Redis later"]
        CompModel["Rig CompletionModel<br>built-in providers now<br>custom providers later"]
    end

    subgraph "Current Implementations"
        SQLite["SqliteMemory"]
        Keyring["KeyringStore"]
        LocalCh["LocalChannel"]
        TokioBus["TokioBroadcastBus"]
        RigProviders["OpenAI / Anthropic / etc."]
    end

    subgraph "Future Implementations"
        PgVec["PostgreSQL + pgvector"]
        Vault["HashiCorp Vault / KMS"]
        NatsCh["NATS Channel"]
        RedisBus["Redis EventBus"]
        CustomProv["Custom LLM Provider"]
    end

    Memory --> SQLite
    Memory -.-> PgVec
    CredStore --> Keyring
    CredStore -.-> Vault
    Channel --> LocalCh
    Channel -.-> NatsCh
    EvBus --> TokioBus
    EvBus -.-> RedisBus
    CompModel --> RigProviders
    CompModel -.-> CustomProv
```

All binary crates receive these traits via `AppState` (Clone + Arc\<T\>), never concrete types.

## Credential System

```mermaid
graph TB
    subgraph "Credential Module"
        Mod["mod.rs<br>CredentialStore trait<br>get / set / delete / list"]
        KR["keyring.rs<br>KeyringStore #40;production#41;<br>OS keychain integration"]
        Mem["memory.rs<br>InMemoryStore #40;tests/CI#41;<br>DashMap-backed"]
    end

    subgraph "Binary Access"
        Desktop["Desktop: direct keyring"]
        CLI["CLI: direct keyring"]
        TUI["TUI: via gateway HTTP"]
        Daemon["Daemon: direct keyring"]
        Mobile["Mobile: direct keyring"]
    end

    Mod --> KR
    Mod --> Mem
    Desktop --> KR
    CLI --> KR
    Daemon --> KR
    Mobile --> KR
    TUI -->|HTTP API| GW["Gateway"]
    GW --> KR
```

### Per-Binary Keyring Access

| Binary | Keyring Access | Notes |
|---|---|---|
| **Desktop** | Direct | Tauri 2 has full OS access |
| **Mobile** | Direct | Tauri 2 mobile has keychain access |
| **CLI** | Direct | Runs as user process |
| **TUI** | Via gateway | Connects to daemon over HTTP |
| **Daemon** | Direct | Headless, runs as service |

All credential values are wrapped with `zeroize` for secure memory cleanup.

## Identity / Soul System

Identity defines the AI assistant's personality, tone, and behavior through 3 markdown files with YAML frontmatter. All prompt content comes from `.md` files — zero hardcoded prompt strings in Rust code.

```mermaid
graph TB
    subgraph Files["Identity Files"]
        SOUL["SOUL.md"] & IDENT["IDENTITY.md"] & USER["USER.md"]
    end

    subgraph SoulLoaderSG["SoulLoader"]
        Loader["Load + parse<br>YAML frontmatter"]
        Reload["POST /identity/reload"]
    end

    subgraph ComposerSG["PromptComposer"]
        Compose["1. SOUL<br>2. IDENTITY meta<br>3. USER<br>4. Observations<br>5. Active skills<br>6. Config override"]
    end

    SOUL & IDENT & USER --> Loader
    Reload --> Loader
    Loader --> Compose
    Compose --> Agent["Rig Agent"]
```

### Identity File Format (IDENTITY.md)

```markdown
---
name: MesoClaw
version: "2.0"
description: AI-powered assistant
---

# Identity details...
```

- **Storage**: `data_dir/identity/` (configurable via `identity_dir` in config.toml)
- **Bundled defaults**: embedded via `include_str!()` at compile time, written to disk on first run
- **Reload**: manual via `POST /identity/reload` endpoint (no `notify` dependency)
- **API**: `GET /identity`, `GET /identity/{name}`, `PUT /identity/{name}`, `POST /identity/reload`

## Skills System

Skills are instructional markdown documents loaded into the agent's context. They follow the Claude Code model — pure markdown with YAML frontmatter metadata, no parameter substitution.

```mermaid
graph TB
    subgraph "Skill Sources"
        BuiltIn["Bundled skills<br>include_str! at compile time<br>system-prompt, summarize"]
        UserDir["User skills directory<br>data_dir/skills/*.md"]
    end

    subgraph "SkillRegistry"
        Load["load_all#40;#41;<br>bundled first, then user"]
        Parse["parse frontmatter<br>serde_yaml metadata"]
        Store["RwLock HashMap<br>in-memory registry"]
    end

    BuiltIn --> Load
    UserDir --> Load
    Load --> Parse
    Parse --> Store
    Store --> Compose["PromptComposer<br>enabled skills → agent context"]
```

### Skill File Format (Claude Code model)

```markdown
---
name: system-prompt
description: Generates effective system prompts for AI agents
category: meta
---

# System Prompt Generator

When creating system prompts, follow these principles:
...
```

- **No Tera/comrak**: Skills are pure markdown context documents, not parameterized templates
- **2 tiers**: Bundled (compile-time) + User (disk). User skills with same id override bundled.
- **API**: `GET /skills`, `GET /skills/{id}`, `POST /skills`, `PUT /skills/{id}`, `DELETE /skills/{id}`, `POST /skills/reload`
- **Bundled skills cannot be deleted** — only user skills support DELETE

## User Profile + Progressive Learning

MesoClaw learns user preferences over time via explicit observation API. Observations are stored in SQLite with category-based organization and confidence scoring.

```mermaid
graph TB
    subgraph "User Context"
        UserMd["USER.md<br>static user context template"]
    end

    subgraph "UserLearner - SQLite backed"
        Observe["observe#40;#41;<br>add/update observation"]
        Query["get_observations#40;#41;<br>filter by category"]
        Build["build_context#40;#41;<br>format for prompt"]
        Prune["prune_expired#40;#41;<br>TTL-based cleanup"]
    end

    subgraph "Privacy Controls"
        Toggle["learning_enabled = true/false"]
        Denied["learning_denied_categories"]
        MinConf["learning_min_confidence = 0.5"]
        TTL["learning_observation_ttl_days = 365"]
        Clear["DELETE /user/observations"]
    end

    UserMd --> Compose["PromptComposer"]
    Build --> Compose
    Compose --> Agent["Rig Agent context<br>personalized responses"]
    Toggle --> Observe
    Denied --> Observe
    MinConf --> Build
    TTL --> Prune
```

- **USER.md**: static user context template (part of identity system)
- **UserLearner**: SQLite-backed observation store with CRUD operations
- **Observations**: stored in `user_observations` table with category, key, value, confidence, timestamps
- **Privacy**: learning toggled via config, denied categories block specific observation types, TTL auto-expires old observations
- **API**: `GET /user/observations`, `POST /user/observations`, `GET /user/observations/{key}`, `DELETE /user/observations/{key}`, `DELETE /user/observations`, `GET /user/profile`

## Gateway Routes

All clients communicate via the HTTP+WebSocket gateway at `127.0.0.1:18981`. Routes are grouped by subsystem (36 implemented through Phase 4).

### Health (1 route, no auth)

| Method | Path | Description |
|---|---|---|
| GET | `/health` | Health check |

### Sessions & Chat (7 routes)

| Method | Path | Description |
|---|---|---|
| POST | `/sessions` | Create new chat session |
| GET | `/sessions` | List all sessions |
| GET | `/sessions/{id}` | Get session details |
| PUT | `/sessions/{id}` | Update session |
| DELETE | `/sessions/{id}` | Delete session |
| GET | `/sessions/{id}/messages` | Get messages for a session |
| POST | `/sessions/{id}/messages` | Send message to session |

### Chat (1 route)

| Method | Path | Description |
|---|---|---|
| POST | `/chat` | Chat with AI agent |

### Memory (5 routes)

| Method | Path | Description |
|---|---|---|
| POST | `/memory` | Create memory entry |
| GET | `/memory` | Recall/search memories |
| GET | `/memory/{key}` | Get memory by key |
| PUT | `/memory/{key}` | Update memory by key |
| DELETE | `/memory/{key}` | Delete memory by key |

### Configuration (2 routes)

| Method | Path | Description |
|---|---|---|
| GET | `/config` | Get current configuration (auth token redacted) |
| PUT | `/config` | Update configuration |

### Providers & Models (3 routes)

| Method | Path | Description |
|---|---|---|
| GET | `/providers` | List configured AI providers |
| GET | `/providers/{id}` | Get provider details |
| GET | `/models` | List available models |

### Tools (2 routes)

| Method | Path | Description |
|---|---|---|
| GET | `/tools` | List available tools |
| POST | `/tools/{name}/execute` | Execute a tool by name |

### System (1 route)

| Method | Path | Description |
|---|---|---|
| GET | `/system/info` | System information |

### WebSocket Channels (1 route)

| Path | Description |
|---|---|
| `/ws/chat` | Streaming chat responses |

### Identity (4 routes)

| Method | Path | Description |
|---|---|---|
| GET | `/identity` | List all identity files |
| GET | `/identity/{name}` | Get identity file content |
| PUT | `/identity/{name}` | Update identity file content |
| POST | `/identity/reload` | Force reload all identity files |

### Skills (6 routes)

| Method | Path | Description |
|---|---|---|
| GET | `/skills` | List all skills (optional `?category=` filter) |
| GET | `/skills/{id}` | Get full skill definition |
| POST | `/skills` | Create user skill |
| PUT | `/skills/{id}` | Update skill content |
| DELETE | `/skills/{id}` | Delete user skill (bundled cannot be deleted) |
| POST | `/skills/reload` | Force reload all skills |

### User Profile + Learning (6 routes)

| Method | Path | Description |
|---|---|---|
| GET | `/user/observations` | List observations (optional `?category=` filter) |
| POST | `/user/observations` | Add observation |
| GET | `/user/observations/{key}` | Get observation by key |
| DELETE | `/user/observations/{key}` | Delete observation by key |
| DELETE | `/user/observations` | Clear all observations |
| GET | `/user/profile` | Get computed user context string |

### Future Phases (not yet implemented)

| Group | Routes | Phase |
|---|---|---|
| Scheduler | 4 routes (feature-gated) | Phase 8 |
| Web Dashboard | 1 route (feature-gated) | Phase 6 |
| WebSocket `/ws/events`, `/ws/agents` | 2 channels | Phase 5+ |

## Concurrency Rules

These rules are enforced across the entire codebase to prevent async runtime issues.

| Rule | Rationale |
|---|---|
| No `std::sync::Mutex` in async paths | Blocks the tokio runtime; use `tokio::sync::Mutex` or `DashMap` |
| No `block_on()` anywhere | Panics inside tokio runtime; use `tokio::spawn` or `.await` |
| All SQLite ops via `spawn_blocking` | `rusqlite` is synchronous; blocking in async context starves tasks |
| All errors are `MesoError` | No `Result<T, String>`; use `thiserror` enum with typed variants |
| `AppState` is `Clone + Arc<T>` | Shared across axum handlers without lifetime issues |
| `EventBus` uses `tokio::sync::broadcast` | Lock-free fan-out to all subscribers |
| Never hold async locks across `.await` | Prevents deadlocks; acquire, use, drop before yielding |

## Lessons Learned from v1

Key architectural mistakes from MesoClaw v1 and how v2 prevents them.

| v1 Mistake | v2 Prevention |
|---|---|
| `std::sync::Mutex` in async code | `tokio::sync::Mutex` or `DashMap` exclusively |
| `block_on()` in event loop | Zero `block_on()` calls; `tokio::spawn` for sync work |
| `Result<T, String>` everywhere | `MesoError` enum with `thiserror` |
| Custom AI layer (1400 LOC) | `rig-core` (battle-tested, 18 providers) |
| 21 Zustand stores | Max 7 Svelte stores, single WS connection |
| 165 IPC commands (Tauri v1) | Gateway-only architecture (~40 HTTP routes) |
| OKLCH color functions in CSS | Pre-computed hex values only |
| useEffect soup (React) | Single `$effect` per Svelte component, reactive stores |
| 13-phase boot sequence | Single `init_services()` in `boot.rs` |