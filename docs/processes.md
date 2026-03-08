# MesoClaw Process Flows

## Table of Contents

- [Chat Request Flow](#chat-request-flow)
- [Startup Sequence](#startup-sequence)
- [Default Paths by OS](#default-paths-by-os)
- [Error Handling Flow](#error-handling-flow)
- [Database Operation Flow](#database-operation-flow-async-safe)
- [WebSocket Message Flow](#websocket-message-flow)
- [Identity Loading Flow](#identity-loading-flow)
- [Skill Loading Flow](#skill-loading-flow)
- [User Learning Flow](#user-learning-flow)
- [Channel Message Flow](#channel-message-flow)
- [Channel Registration Flow](#channel-registration-flow)
- [Desktop Boot Flow](#desktop-boot-flow)
- [Credential Flow](#credential-flow)
- [Provider Management Flow](#provider-management-flow)

---

## Chat Request Flow

```mermaid
sequenceDiagram
    participant U as User (any interface)
    participant G as Gateway (axum)
    participant AI as AI Engine (rig-core)
    participant M as Memory (sqlite-vec)
    participant LLM as LLM Provider
    participant T as Tools

    U->>G: Send message (REST/WS)
    G->>M: Query relevant context (FTS5 + vectors)
    M-->>G: Context results
    G->>AI: Dispatch with context + tools
    AI->>LLM: Stream prompt

    loop Tool calling loop
        LLM-->>AI: Response (may contain tool calls)
        alt Tool call detected
            AI->>T: Execute tool (websearch, sysinfo, etc.)
            T-->>AI: Tool result
            AI->>LLM: Feed result back
        end
    end

    LLM-->>AI: Final response tokens
    AI-->>G: Stream tokens
    G-->>U: Stream to client via WS
    G->>M: Store conversation
```

## Startup Sequence

```mermaid
sequenceDiagram
    participant App as Application
    participant Cfg as Config
    participant DB as SQLite
    participant Cred as Keyring
    participant AI as AI Providers
    participant GW as Gateway

    App->>Cfg: Parse CLI args + load TOML config
    App->>App: Initialize tracing/logging
    App->>DB: Open/create database
    DB->>DB: Run pending migrations
    App->>Cred: Initialize credential store (KeyringStore / InMemoryStore)
    App->>AI: Register providers + load API keys
    App->>AI: Register 9 agent tools into ToolRegistry (DashMap)
    App->>App: Load identity (SoulLoader from data_dir/identity/)
    App->>App: Load skills (SkillRegistry from data_dir/skills/)
    App->>App: Init user learner (UserLearner from DB pool)
    opt channels feature enabled
        App->>App: Init ChannelRegistry (DashMap)
        App->>App: Register configured channels (Telegram/Slack/Discord)
    end
    App->>App: Bundle into Services struct
    App->>GW: Start axum server (127.0.0.1:18981)

    alt Desktop
        App->>App: Setup tray + resolve gateway mode
        App->>App: Boot embedded gateway or connect to external
        App->>App: Open Tauri window
    else Mobile
        App->>App: Open Tauri mobile view (in-process gateway)
    else CLI
        App->>App: Connect to daemon via HTTP/WS (MesoClient)
    else TUI
        App->>App: Render ratatui UI
    else Daemon
        App->>App: Wait for connections
    end
```

## Default Paths by OS

Resolved via `directories::ProjectDirs::from("com", "sprklai", "mesoclaw")`:

| OS | Config Path | Data Dir / DB Path |
|---|---|---|
| **Linux** | `~/.config/mesoclaw/config.toml` | `~/.local/share/mesoclaw/mesoclaw.db` |
| **macOS** | `~/Library/Application Support/com.sprklai.mesoclaw/config.toml` | `~/Library/Application Support/com.sprklai.mesoclaw/mesoclaw.db` |
| **Windows** | `%APPDATA%\sprklai\mesoclaw\config\config.toml` | `%APPDATA%\sprklai\mesoclaw\data\mesoclaw.db` |

Override via `config.toml`:
```toml
data_dir = "/custom/data/path"        # overrides default data directory
db_path = "/custom/path/mesoclaw.db"  # overrides database file directly
```

## Error Handling Flow

```mermaid
flowchart TD
    Call[Function Call] --> Result{Operation Result}
    Result -->|Ok| ReturnValue[Return value]
    Result -->|Err| Match{Match MesoError variant}
    Match -->|NotFound| NF["404 MESO_NOT_FOUND"]
    Match -->|Auth| Auth["401 MESO_AUTH_REQUIRED"]
    Match -->|PolicyDenied| PD["403 MESO_POLICY_DENIED"]
    Match -->|Serialization| Ser["400 MESO_BAD_REQUEST"]
    Match -->|Config| Cfg["422 MESO_CONFIG_ERROR"]
    Match -->|RateLimited| RL["429 MESO_RATE_LIMITED"]
    Match -->|Agent| AI["502 MESO_AGENT_ERROR"]
    Match -->|Database| DB["503 MESO_DB_ERROR"]
    Match -->|Tool / Gateway| TG["500 MESO_TOOL_ERROR /<br>MESO_GATEWAY_ERROR"]
```

## Database Operation Flow (async-safe)

```mermaid
flowchart TD
    Async[Async Context] --> Spawn["tokio::task::spawn_blocking#40;move || { ... }#41;"]
    Spawn --> SQLite["rusqlite operation<br>#40;runs on blocking thread pool,<br>NOT on async executor#41;"]
    SQLite --> Result["Result of T or MesoError"]
    Result --> Await[".await -- resumes async context"]
    Await --> Handle[Handle Result]
```

## WebSocket Message Flow

```mermaid
sequenceDiagram
    participant C as Client
    participant S as Server (Gateway)

    C->>S: WS Connect /ws/chat?token=xxx
    C->>S: { "prompt": "hello", "session_id": "optional-uuid" }
    Note over S: Validate JSON, check agent, call MesoAgent.prompt
    S-->>C: { "type": "text", "content": "Hi there!" }
    S-->>C: { "type": "done" }
    Note over C,S: Error cases
    C->>S: invalid-json
    S-->>C: { "type": "error", "error": "invalid JSON: ..." }
    C->>S: { "prompt": "hello" } (no agent configured)
    S-->>C: { "type": "error", "error": "no agent configured" }
```

## Identity Loading Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant FS as Filesystem
    participant SL as SoulLoader
    participant SY as serde_yaml
    participant PC as PromptComposer
    participant AG as Rig Agent

    App->>SL: SoulLoader::new(identity_dir)
    SL->>FS: Check for SOUL.md, IDENTITY.md, USER.md
    alt Files missing
        SL->>FS: Write bundled defaults (include_str!)
    end
    SL->>FS: Read all identity files
    FS-->>SL: Raw markdown content
    SL->>SY: Parse IDENTITY.md YAML frontmatter
    SY-->>SL: IdentityMeta (name, version, description)
    SL->>SL: Store Identity in RwLock

    Note over PC,AG: Prompt composition at chat time
    PC->>SL: Get identity files
    SL-->>PC: Identity (soul + meta + user)
    PC->>PC: Compose: SOUL + meta + USER + observations + skills + config
    PC-->>AG: Final system prompt string

    Note over SL: Manual reload via API
    Note over SL: POST /identity/reload triggers SoulLoader::reload()
```

## Skill Loading Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant SR as SkillRegistry
    participant FS as Filesystem
    participant SY as serde_yaml
    participant PC as PromptComposer
    participant AG as Rig Agent

    App->>SR: SkillRegistry::new(skills_dir)
    SR->>SR: Load bundled skills (include_str!)
    SR->>FS: Scan skills_dir/*.md
    FS-->>SR: User skill files
    SR->>SY: Parse YAML frontmatter per file
    SY-->>SR: SkillFrontmatter (name, description, category)
    SR->>SR: User skills override bundled (same id)
    SR->>SR: Store in RwLock HashMap

    Note over SR,AG: At prompt composition time
    SR->>SR: active_skills() — filter enabled skills
    SR-->>PC: Vec of (name, content) pairs
    PC->>PC: Include skill content in system prompt
    PC-->>AG: Final system prompt with skills

    Note over SR: CRUD via API
    Note over SR: POST /skills — create user skill
    Note over SR: PUT /skills/id — update content
    Note over SR: DELETE /skills/id — remove user skill
    Note over SR: POST /skills/reload — re-scan disk
```

## User Learning Flow

```mermaid
sequenceDiagram
    participant API as Gateway API
    participant UL as UserLearner
    participant DB as SQLite (user_observations)
    participant PC as PromptComposer
    participant AG as Rig Agent

    Note over API,UL: Observation management via API
    API->>UL: POST /user/observations (category, key, value, confidence)
    UL->>UL: Check learning_enabled and denied_categories
    UL->>DB: INSERT OR REPLACE into user_observations
    DB-->>UL: Stored observation

    Note over UL,DB: Query and context building
    API->>UL: GET /user/profile
    UL->>DB: Query observations where confidence >= min_confidence
    DB-->>UL: Matching observations
    UL->>UL: build_context() — format as "key: value (confidence: X)"
    UL-->>API: Context string

    Note over PC,AG: At prompt composition time
    PC->>UL: build_context()
    UL-->>PC: Formatted observations string
    PC->>PC: Include as "Known Preferences" section
    PC-->>AG: System prompt with user context

    Note over UL: Privacy controls
    Note over UL: learning_enabled = false blocks new observations
    Note over UL: learning_denied_categories blocks specific categories
    Note over UL: prune_expired() removes observations older than TTL
    Note over UL: DELETE /user/observations clears all
```

## Channel Message Flow

```mermaid
sequenceDiagram
    participant Ext as External Platform
    participant Ch as Channel (Telegram/Slack/Discord)
    participant CR as ChannelRegistry
    participant GW as Gateway
    participant AG as Rig Agent

    Note over Ch,CR: Channel lifecycle
    GW->>CR: register(channel)
    GW->>Ch: connect() via ChannelLifecycle
    Ch->>Ext: Establish connection

    Note over Ext,AG: Message handling
    Ext->>Ch: Platform message arrives
    Ch->>Ch: Normalize to ChannelMessage
    Ch->>AG: Route normalized message
    AG->>AG: Process + generate response
    AG-->>Ch: Response text
    Ch->>Ch: Format for platform (MarkdownV2 / mrkdwn / etc.)
    Ch->>Ext: send_text() via ChannelSender

    Note over GW,Ch: Health monitoring
    GW->>Ch: health_check() via ChannelLifecycle
    Ch-->>GW: ChannelStatus (Connected/Disconnected/Error)
```

## Channel Registration Flow

```mermaid
flowchart TD
    Boot([Boot with channels feature]) --> Init["Initialize ChannelRegistry<br>DashMap-backed"]
    Init --> Check{"channels_enabled<br>config list"}

    Check -->|telegram| TG["Create TelegramChannel<br>Load config: DmPolicy, polling timeout"]
    Check -->|slack| SL["Create SlackChannel<br>Load config: bot token"]
    Check -->|discord| DC["Create DiscordChannel<br>Load config: guild/channel allowlists"]

    TG --> Reg["Register in ChannelRegistry"]
    SL --> Reg
    DC --> Reg

    Reg --> Creds{"Credentials<br>available?"}
    Creds -->|Yes| Connect["connect() → platform API"]
    Creds -->|No| Wait["Status: Disconnected<br>Awaiting credentials"]

    Connect --> Ready["Status: Connected<br>Ready for messages"]
    Wait --> UI["User sets credentials<br>via Settings UI or CLI"]
    UI --> Connect
```

## Desktop Boot Flow

The desktop app uses a hybrid gateway model. By default it starts an embedded gateway; if `MESOCLAW_GATEWAY_URL` is set, it connects to an external daemon instead.

```mermaid
sequenceDiagram
    participant Main as main.rs
    participant Lib as lib.rs Builder
    participant Tray as tray.rs
    participant Cmd as commands.rs
    participant Core as mesoclaw-core
    participant GW as Gateway

    Main->>Main: Linux: set WEBKIT_DISABLE_DMABUF_RENDERER
    Main->>Lib: run#40;#41;

    Lib->>Lib: Register plugins<br>window-state, single-instance, opener
    Note over Lib: devtools plugin if feature enabled

    Lib->>Lib: setup#40;#41; hook
    Lib->>Tray: setup_tray#40;app#41;
    Tray->>Tray: Create menu: Show / Hide / Separator / Quit
    Tray->>Tray: Register tray icon with menu + click handlers

    Lib->>Cmd: boot_gateway#40;app#41;
    Cmd->>Cmd: resolve_gateway_mode#40;#41;

    alt MESOCLAW_GATEWAY_URL set
        Cmd->>Cmd: Validate URL, store external mode
    else No env var or empty
        Cmd->>Core: load_or_create_config#40;#41;
        Core-->>Cmd: AppConfig
        Cmd->>Core: init_services#40;config#41;
        Core-->>Cmd: Services
        Cmd->>GW: Start axum on host:port in background task
        Cmd->>Cmd: Store shutdown_tx in managed state
    end

    Lib->>Lib: Register IPC handlers
    Lib->>Lib: Register on_window_event: close hides to tray
    Lib->>Lib: run#40;generate_context!#41;
```

## Credential Flow

```mermaid
sequenceDiagram
    participant User as User
    participant CLI as CLI / Desktop UI
    participant KS as KeyringStore
    participant KR as OS Keyring
    participant Daemon as Daemon
    participant CS as CredentialStore
    participant AG as Rig Agent

    Note over User,KR: Setting credentials
    User->>CLI: mesoclaw key set openai <key>
    CLI->>KS: KeyringStore.set("mesoclaw.openai", key)
    KS->>KR: Store in OS keyring

    Note over User,KR: Desktop settings
    User->>CLI: Desktop Settings UI → enter key
    CLI->>KS: KeyringStore.set() → OS keyring
    KS->>KR: Store in OS keyring

    Note over Daemon,KR: Daemon boot
    Daemon->>KS: Initialize credential store
    KS->>KR: Read keys from OS keyring
    KR-->>KS: API keys

    Note over AG,CS: Runtime key access
    AG->>CS: CredentialStore.get("mesoclaw.openai")
    CS->>KS: Lookup key
    KS-->>CS: API key value
    CS-->>AG: API key

    Note over KS: All binaries share same keyring namespace (same OS user)
    Note over KS: CI/test: InMemoryStore used instead of keyring
```

## Provider Management Flow

```mermaid
sequenceDiagram
    participant User as User
    participant UI as Settings UI / CLI
    participant GW as Gateway
    participant PR as ProviderRegistry
    participant DB as SQLite (ai_providers + ai_models)
    participant KS as KeyringStore

    Note over User,DB: First boot — seed providers
    GW->>PR: ProviderRegistry::new(db_pool)
    PR->>DB: Seed 6 built-in providers if empty
    DB-->>PR: Provider configs

    Note over User,KS: Configure provider API key
    User->>UI: Enter API key for provider
    UI->>GW: POST /credentials { key: "api_key:openai", value: "sk-..." }
    GW->>KS: Store in OS keyring

    Note over User,DB: Test connection
    User->>UI: Click "Test Connection"
    UI->>GW: POST /providers/openai/test
    GW->>PR: test_connection(provider_id)
    PR->>KS: Resolve API key
    KS-->>PR: API key value
    PR->>PR: Build client + send test request
    PR-->>GW: TestResult { success, latency_ms }
    GW-->>UI: Display result + latency

    Note over User,DB: Manage models
    User->>UI: Add custom model
    UI->>GW: POST /providers/openai/models { id: "gpt-4o-mini" }
    GW->>PR: add_model(provider_id, model)
    PR->>DB: INSERT into ai_models

    Note over User,DB: Set default model
    User->>UI: Select default model
    UI->>GW: PUT /providers/default { provider_id: "openai", model_id: "gpt-4o" }
    GW->>PR: set_default_model()
    PR->>DB: Upsert _default_model row
```
