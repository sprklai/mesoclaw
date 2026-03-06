# MesoClaw Process Flows

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
    App->>AI: Register agent tools
    App->>App: Load persona (SoulLoader from ~/.mesoclaw/personas/*.md)
    App->>App: Load skills (SkillRegistry)
    App->>App: Load user profile (~/.mesoclaw/user.toml)
    App->>App: Bundle into Services struct
    App->>GW: Start axum server (127.0.0.1:18981)

    alt Desktop
        App->>App: Open Tauri window
    else Mobile
        App->>App: Open Tauri mobile view (in-process gateway)
    else CLI
        App->>App: Enter REPL loop
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
    Match -->|DatabaseError| DB[Log + return 500]
    Match -->|ConfigError| Cfg[Log + return 400]
    Match -->|AiError| AI[Log + retry or return 503]
    Match -->|AuthError| Auth[Return 401]
    Match -->|NotFound| NF[Return 404]
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

    C->>S: WS Connect /ws/chat
    C->>S: { type: "chat", content: "hello" }
    Note over S: Query memory + build prompt + call LLM
    S-->>C: { type: "token", content: "Hi" }
    S-->>C: { type: "token", content: " there" }
    S-->>C: { type: "tool_call", name: "websearch" }
    S-->>C: { type: "tool_result", result: "..." }
    S-->>C: { type: "done" }
```

## Persona Loading Flow

```mermaid
sequenceDiagram
    participant App as Application
    participant FS as Filesystem
    participant SL as SoulLoader
    participant CM as comrak
    participant SY as serde_yaml
    participant TR as Tera
    participant AG as Rig Agent
    participant NW as notify (watcher)

    App->>FS: Read ~/.mesoclaw/personas/*.md
    FS-->>SL: Raw markdown file(s)
    SL->>CM: Extract YAML frontmatter
    CM-->>SL: Frontmatter string + body
    SL->>SY: Deserialize frontmatter metadata
    SY-->>SL: PersonaMetadata struct
    SL->>TR: Render {{variables}} in body
    TR-->>SL: Rendered preamble text
    SL->>AG: Inject via agent.preamble()
    AG-->>App: Agent ready with persona

    Note over NW,SL: Hot-reload loop
    NW->>NW: Watch ~/.mesoclaw/personas/ for changes
    NW->>SL: File change detected
    SL->>CM: Re-parse changed file
    CM-->>SL: Updated frontmatter + body
    SL->>SY: Deserialize updated metadata
    SY-->>SL: Updated PersonaMetadata
    SL->>TR: Re-render body
    TR-->>SL: Updated preamble
    SL->>AG: Update active preamble
```

## Skill Invocation Flow

```mermaid
sequenceDiagram
    participant U as User
    participant UI as Frontend / CLI
    participant SR as SkillRegistry
    participant FS as Filesystem
    participant TR as Tera
    participant AG as Rig Agent
    participant LLM as LLM Provider

    U->>UI: Select skill
    UI->>SR: Request skill definition
    SR->>FS: Load skill .md file
    FS-->>SR: Raw skill template
    SR-->>UI: Skill metadata + parameter schema
    UI->>U: Prompt for parameter values
    U->>UI: Provide parameter values
    UI->>SR: Invoke skill with parameters
    SR->>TR: Render template with parameters
    TR-->>SR: Rendered skill prompt
    SR->>AG: Prepend rendered prompt to user message
    AG->>LLM: Send combined prompt
    LLM-->>AG: Response tokens
    AG-->>UI: Stream response
    UI-->>U: Display result
```

## User Learning Flow

```mermaid
sequenceDiagram
    participant EB as EventBus
    participant UL as UserLearner
    participant DB as SQLite (memory)
    participant FS as ~/.mesoclaw/user.toml
    participant AG as Rig Agent

    Note over EB,UL: Observation collection
    EB->>UL: AppEvent::MessageSent
    UL->>UL: Extract preferences, patterns, corrections
    UL->>DB: Store observation (memory_type = "user_observation", tags)
    EB->>UL: AppEvent::SessionEnded
    UL->>UL: Summarize session observations
    UL->>DB: Store summary observation

    Note over UL,FS: Periodic consolidation
    UL->>DB: Query recent observations
    DB-->>UL: Observation entries
    UL->>UL: Summarize into user profile
    UL->>FS: Write updated ~/.mesoclaw/user.toml

    Note over AG,DB: Session start recall
    AG->>DB: Recall relevant observations for context
    DB-->>AG: Matching observations
    AG->>AG: Inject observations into agent context

    Note over UL: Privacy controls
    Note over UL: Learning can be disabled via config
    Note over UL: Observations can be viewed / deleted / reset
```

## Channel Message Flow

```mermaid
sequenceDiagram
    participant Ext as External Platform (Telegram, Discord, etc.)
    participant CA as Channel Adapter
    participant CI as ChannelInbound
    participant AG as Rig Agent
    participant CO as ChannelOutbound
    participant Ext2 as External Platform

    Ext->>CA: Raw platform message arrives
    CA->>CI: Pass raw message
    CI->>CI: normalize() → standardized Message
    CI->>AG: Route normalized message
    AG->>AG: Process message + generate response
    AG-->>CO: Response text
    CO->>CO: Format for platform
    CO->>Ext2: send_text() → platform-specific delivery
    Ext2-->>CO: Delivery confirmation
    CO->>CO: acknowledge(msg_id) → mark as handled
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
