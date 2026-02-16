# Comparative Analysis: OpenClaw, PicoClaw, IronClaw, ZeroClaw

> Architectural patterns, feature implementations, and design decisions across the "claw" ecosystem, with recommendations for TauriClaw.

---

## 1. Project Overview Comparison

| Dimension | OpenClaw | PicoClaw | IronClaw | ZeroClaw |
|-----------|----------|----------|----------|----------|
| **Language** | TypeScript (Node.js) | Go | Rust | Rust |
| **Repository** | github.com/openclaw/openclaw | github.com/sipeed/picoclaw | github.com/nearai/ironclaw | github.com/openagen/zeroclaw |
| **Purpose** | Full-featured personal AI assistant | Ultra-lightweight AI agent for edge | NEAR AI agentic worker framework | Lightweight security-first AI agent |
| **Binary Size** | ~28 MB + Node.js (~390 MB) | Single binary, tiny | Single binary | ~3.4 MB |
| **RAM Usage** | >1 GB | <10 MB | Moderate (PostgreSQL dep) | <5 MB |
| **Startup** | >500s on slow hardware | <1s on 0.6 GHz | Fast | <10ms |
| **Target** | Desktop/server | Edge devices ($10 SBCs) | NEAR AI marketplace | Resource-constrained + secure |
| **Maturity** | Most mature, 30+ plugins | Newest, minimal | Partial parity with OpenClaw | Growing rapidly |
| **Test Count** | 70% coverage target | Minimal | Growing | 1,017 tests |

---

## 2. Architecture Patterns

### OpenClaw (TypeScript) - Hub-and-Spoke

- **Gateway pattern**: Central WebSocket control plane at `ws://127.0.0.1:18789`
- **Plugin architecture**: 30+ extension plugins, 40+ skills
- **Adapter pattern**: Each messaging platform is a channel adapter plugin
- **Massive dependency tree**: Express 5, Playwright, Grammy, Baileys, etc.
- **UI**: Lit (Web Components) for Control UI
- **Mobile**: Swift (iOS/macOS), Kotlin (Android)
- **Config**: JSON5 with schema validation (Zod + TypeBox)
- **Memory**: LanceDB vector storage, markdown-based, hybrid BM25 + vector search

**Key Design Decisions:**
- Plugin SDK at `src/plugin-sdk/` for extensibility
- Channel plugins implement standardized adapters (auth, messaging, outbound, status, directory, threading)
- WebSocket-based async communication for real-time control
- Background task management with proper lockfile handling

**Strengths:**
- Feature-complete, most integrations (15+ messaging platforms)
- Extensive plugin and skills ecosystem
- Production-grade deployment options (Docker, Fly.io, Render, systemd)

**Weaknesses:**
- Heavy runtime requirement (Node.js >= 22)
- Enormous dependency footprint (~60+ npm packages)
- High memory consumption (>1 GB)

---

### PicoClaw (Go) - Minimal Core

- **Interface-driven**: `LLMProvider` interface with clean Go structs
- **JSON config with env override**: Struct tags for both JSON and environment variables
- **Thread-safe config**: `sync.RWMutex` for concurrent access
- **Single binary, cross-arch**: RISC-V, ARM64, x86-64

**Provider Support (12+):**
- OpenAI, Anthropic, OpenRouter, Groq, Zhipu, VLLM, Gemini, Nvidia, Moonshot, ShengSuanYun, DeepSeek, GitHub Copilot

**Channel Support (10+):**
- WhatsApp, Telegram, Discord, Slack, Feishu, QQ, DingTalk, LINE, OneBot, MaixCam

**Key Design Decisions:**
- Go interfaces over inheritance for provider/channel abstraction
- Environment variable override pattern (`PICOCLAW_PROVIDERS_{{.Name}}_API_KEY`)
- Workspace-restricted filesystem access by default
- Heartbeat system for periodic health checks
- Web search tools (Brave, DuckDuckGo) as built-in capabilities

**Strengths:**
- 99% less memory than OpenClaw (10 MB vs 1 GB+)
- 1% of the codebase (95% agent-generated via recursive engineering)
- Sub-second startup on 0.6 GHz single-core processors
- True cross-platform single binary

**Weaknesses:**
- Fewer features, less extensible plugin system
- No formal extension/plugin mechanism
- Limited documentation compared to others

---

### IronClaw (Rust) - WASM-First Extensibility

- **WASM sandbox**: Novel approach - tools/channels run in WASM (not Docker)
- **Layered architecture**: User Interaction → Agent Loop → Safety Layer + Self-Repair
- **Trait-based**: `Channel`, `LlmProvider`, `Tool`, `Workspace` traits
- **PostgreSQL + pgvector**: Production-grade memory with hybrid search (BM25 + vector, RRF algorithm)
- **Ratatui TUI**: Native terminal UI with approval overlays
- **NEAR AI focus**: Primary provider with OAuth session-based auth

**Architecture (from lib.rs):**
```
User Interaction Layer (CLI, Slack, Telegram, HTTP)
        ↓
Main Agent Loop
  ├── Message Router
  ├── LLM Reasoning
  ├── Action Executor
  ├── Safety Layer (input sanitizer, injection defense)
  └── Self-Repair (stuck job detection, tool fixer)
```

**Module Structure:**
- `agent` - Core agent loop
- `channels` - Multi-channel interaction (CLI, Slack, Telegram, HTTP)
- `tools` - Pluggable tool system with WASM sandbox
- `safety` - Input sanitization, prompt injection defense
- `workspace` - Memory and file management
- `estimation` - Continuous learning from historical data
- `evaluation` - Quality assessment
- `extensions` - WASM extension system

**Prelude pattern for clean imports:**
```rust
pub mod prelude {
    pub use crate::channels::{Channel, IncomingMessage, MessageStream};
    pub use crate::config::Config;
    pub use crate::context::{JobContext, JobState};
    pub use crate::error::{Error, Result};
    pub use crate::llm::LlmProvider;
    pub use crate::safety::{SanitizedOutput, Sanitizer};
    pub use crate::tools::{Tool, ToolOutput, ToolRegistry};
    pub use crate::workspace::{MemoryDocument, Workspace};
}
```

**Feature Parity with OpenClaw:**
- Core (done): TUI, HTTP webhook, WASM sandbox, hybrid memory, prompt injection defense, heartbeat, sessions, context compaction, model selection
- P1 (needed): Slack, Telegram, WhatsApp channels, multi-provider failover, gateway, hooks
- P2 (planned): Cron jobs, web UI, media handling, Ollama support
- P3 (future): Discord, Signal, Matrix, skills system, plugin registry

**Intentional Deviations:**
1. Rust vs TypeScript for native performance
2. WASM sandbox vs Docker for lighter weight
3. PostgreSQL vs SQLite for production deployments
4. NEAR AI as primary provider
5. No mobile/desktop apps (server-side focus)
6. WASM channels as novel extension mechanism

**Strengths:**
- WASM sandboxing is innovative and lighter than Docker
- Self-repair capability (stuck job detection, tool fixer)
- Clean Rust architecture with good separation of concerns
- Comprehensive feature parity tracking

**Weaknesses:**
- Many features still unimplemented
- PostgreSQL dependency adds deployment weight
- Smaller community than OpenClaw

---

### ZeroClaw (Rust) - Security-First, Zero Overhead

- **Trait-based abstraction**: Every subsystem (`Provider`, `Channel`, `Memory`, `Tool`) is a swappable trait
- **Factory pattern**: Config-driven instantiation of all components
- **Resilience wrapper**: `ReliableProvider` wraps any provider with retry + fallback
- **Multi-layered security**: 6 layers of defense
- **SQLite-only memory**: Custom hybrid search (vector + BM25) with no external DB
- **Atomic config updates**: Write-to-temp → fsync → backup → atomic rename

**Trait Definitions:**

```rust
// Provider trait (22+ implementations)
#[async_trait]
pub trait Provider: Send + Sync {
    async fn chat_with_system(...) -> anyhow::Result<String>;
    async fn chat_with_history(...) -> anyhow::Result<String>;
    async fn warmup(&self) -> anyhow::Result<()>;
}

// Channel trait (9+ implementations)
#[async_trait]
pub trait Channel: Send + Sync {
    fn name(&self) -> &str;
    async fn send(&self, message: &str, recipient: &str) -> anyhow::Result<()>;
    async fn listen(&self, tx: mpsc::Sender<ChannelMessage>) -> anyhow::Result<()>;
    async fn health_check(&self) -> bool;
    async fn start_typing(&self, recipient: &str) -> anyhow::Result<()>;
}

// Memory trait
#[async_trait]
pub trait Memory: Send + Sync {
    fn name(&self) -> &str;
    async fn store(&self, key: &str, content: &str, category: MemoryCategory) -> anyhow::Result<()>;
    async fn recall(&self, query: &str, limit: usize) -> anyhow::Result<Vec<MemoryEntry>>;
    async fn forget(&self, key: &str) -> anyhow::Result<bool>;
}

// Tool trait
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult>;
}
```

**Security Architecture (6 Layers):**

| Layer | Purpose | Implementation |
|-------|---------|----------------|
| 1. Gateway | Network | Binds to 127.0.0.1, 6-digit pairing code, bearer token auth |
| 2. Command Validation | Execution | 3 autonomy levels, command allowlist, injection protection |
| 3. Filesystem | Access Control | Workspace-only, 14 system dirs blocked, symlink escape detection |
| 4. Runtime | Sandboxing | Docker with read-only rootfs, network isolation, memory limits |
| 5. Secrets | Encryption | ChaCha20-Poly1305 encrypted API keys |
| 6. Rate Limiting | Abuse Prevention | Sliding window, 20 actions/hour default |

**Command Risk Classification:**
- **Low**: Read-only (ls, cat, grep, git status)
- **Medium**: State-changing (git commit, npm install, touch, mkdir)
- **High**: Destructive (rm, sudo, curl, wget, chmod, shutdown)

**Performance Benchmarks (vs OpenClaw):**

| Metric | OpenClaw (TS) | ZeroClaw (Rust) | Improvement |
|--------|---------------|-----------------|-------------|
| Binary Size | ~28MB + Node.js | 3.4 MB | 92% smaller |
| RAM Usage | > 1GB | < 5MB | 99% less |
| Startup (0.8GHz) | > 500s | < 10ms | 50,000x faster |
| Hardware Cost | Mac mini $599 | $10 SBC | 98% cheaper |

**Release Profile Optimization:**
```toml
[profile.release]
opt-level = "z"       # Optimize for size
lto = true            # Link-time optimization
codegen-units = 1     # Better optimization
strip = true          # Remove symbols
panic = "abort"       # No unwind tables
```

**Memory Hybrid Search Algorithm:**
- Vector similarity: Cosine distance on embeddings stored as BLOBs in SQLite
- Keyword search: FTS5 with BM25 scoring
- Weighted merge: default 0.7 vector + 0.3 keyword
- LRU embedding cache: 10,000 entries default

**Configuration-Driven Architecture (TOML):**
```toml
[memory]
backend = "sqlite"              # Swap to "markdown" or "none"
embedding_provider = "openai"   # Swap to "custom:URL" or "none"

[runtime]
kind = "native"                 # Swap to "docker" for sandboxing

[tunnel]
provider = "none"               # Swap to "cloudflare", "tailscale", "ngrok"
```

**Strengths:**
- Best security architecture of all four projects
- Smallest resource footprint (3.4 MB, <5 MB RAM)
- 1,017 tests (130+ security-specific)
- Every subsystem is independently swappable
- No external database dependencies

**Weaknesses:**
- Rust learning curve for contributors
- Newer project, smaller community
- Rapid iteration may introduce instability

---

## 3. Key Design Decisions Compared

| Decision | OpenClaw | PicoClaw | IronClaw | ZeroClaw |
|----------|----------|----------|----------|----------|
| **Config format** | JSON5 | JSON + env vars | .env | TOML + env vars |
| **Extension model** | JS plugins | None (monolithic) | WASM modules | Trait implementations |
| **Memory backend** | SQLite-vec, LanceDB | Minimal | PostgreSQL + pgvector | SQLite (custom vectors) |
| **Sandboxing** | Docker | None | WASM | Docker + Landlock |
| **Tool calling** | OpenAI format | OpenAI format | OpenAI format | XML + OpenAI dual |
| **Security model** | Token auth + pairing | Allowlists | WASM capability | 6-layer defense |
| **LLM providers** | 5+ | 12+ | NEAR AI primary | 22+ |
| **Channels** | 15+ | 10+ | 4 (+ WASM) | 9+ |

---

## 4. Dependency Footprint Analysis

| Project | Core Deps | External DBs | Runtime Required | Deployment |
|---------|-----------|-------------|-----------------|------------|
| OpenClaw | ~60+ npm packages | SQLite | Node.js >= 22 | Complex |
| PicoClaw | ~15 Go modules | None | None (static binary) | Single binary |
| IronClaw | ~30 Rust crates | PostgreSQL | None (static binary) | Binary + Postgres |
| ZeroClaw | ~60 Rust crates | None (SQLite embedded) | None (static binary) | Single binary |

---

## 5. Recommendations for TauriClaw

### Recommended Architecture: "ZeroClaw patterns + PicoClaw minimalism + Tauri shell"

### 5.1 Suggested File Structure

```
tauriclaw/
├── src/                          # Frontend (React/TypeScript)
│   ├── routes/                   # TanStack Router file-based routes
│   ├── stores/                   # Zustand stores
│   │   ├── agent-store.ts        # Agent loop state
│   │   ├── provider-store.ts     # LLM provider config
│   │   ├── channel-store.ts      # Channel status
│   │   └── memory-store.ts       # Memory/search state
│   ├── components/
│   │   ├── ui/                   # Base UI primitives
│   │   ├── chat/                 # Conversation components
│   │   └── settings/             # Provider/channel config
│   └── lib/
│       └── invoke.ts             # Typed Tauri IPC wrappers
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs                # Module tree + prelude (IronClaw pattern)
│   │   ├── commands/             # Tauri IPC commands
│   │   │   ├── agent_commands.rs
│   │   │   ├── provider_commands.rs
│   │   │   ├── channel_commands.rs
│   │   │   └── memory_commands.rs
│   │   ├── agent/                # Core agent loop (ZeroClaw pattern)
│   │   │   ├── mod.rs
│   │   │   ├── loop_.rs          # Multi-turn conversation
│   │   │   └── tool_parser.rs    # XML + OpenAI dual format
│   │   ├── providers/            # LLM provider trait (ZeroClaw pattern)
│   │   │   ├── traits.rs         # Provider trait definition
│   │   │   ├── openai.rs
│   │   │   ├── anthropic.rs
│   │   │   ├── ollama.rs
│   │   │   ├── router.rs         # Model routing
│   │   │   └── reliable.rs       # Retry/fallback wrapper
│   │   ├── channels/             # Channel trait (selective)
│   │   │   ├── traits.rs
│   │   │   ├── webhook.rs        # HTTP webhook
│   │   │   └── websocket.rs      # WebSocket gateway
│   │   ├── memory/               # Memory system (ZeroClaw pattern)
│   │   │   ├── traits.rs
│   │   │   ├── sqlite.rs         # SQLite with hybrid search
│   │   │   ├── embeddings.rs
│   │   │   └── chunker.rs
│   │   ├── tools/                # Tool trait (ZeroClaw pattern)
│   │   │   ├── traits.rs
│   │   │   ├── registry.rs
│   │   │   ├── shell.rs
│   │   │   └── file_ops.rs
│   │   ├── security/             # Security layer (ZeroClaw pattern)
│   │   │   ├── policy.rs
│   │   │   └── secrets.rs        # OS keyring via keyring crate
│   │   └── config/               # Config system (PicoClaw + ZeroClaw)
│   │       ├── mod.rs
│   │       └── schema.rs         # TOML schema with env overrides
│   ├── Cargo.toml
│   └── tauri.conf.json
```

### 5.2 Design Patterns to Adopt

**From ZeroClaw (highest priority):**
- **Trait-based subsystem abstraction** - Define `Provider`, `Channel`, `Memory`, `Tool` traits. This is the single most impactful pattern across all four projects.
- **Factory pattern** for config-driven component creation
- **ReliableProvider wrapper** - Retry + fallback for all LLM calls
- **Multi-layered security** - Especially command validation and path traversal prevention
- **Atomic config updates** - Write-to-temp, fsync, rename pattern
- **Dual tool-calling parser** - Support both XML and OpenAI formats for broad LLM compatibility
- **Comprehensive testing** - Especially security edge cases (130+ tests as model)

**From PicoClaw (high priority):**
- **Minimal dependency footprint** - Only include what you use
- **Interface-driven config pattern** translated to Rust: struct tags → serde + env override
- **Thread-safe config with RwLock** - Already natural in Rust
- **Single-binary philosophy** - No external database dependencies
- **Cross-architecture builds** - Leverage Tauri's cross-platform nature

**From IronClaw (selective):**
- **Prelude module** for clean re-exports of common types
- **WASM sandbox concept** for future extensibility (not day-1)
- **FEATURE_PARITY.md** tracking document - excellent project management practice
- **Self-repair capability** - Stuck job detection and tool fixing

**From OpenClaw (conceptual only):**
- **Channel adapter pattern** - But implement in Rust traits, not JS plugins
- **Skills system** - But use TOML manifests (ZeroClaw) not JSON5
- **Plugin metadata pattern** - Standardized manifest for extensions

### 5.3 Dependencies to Prioritize

```toml
# KEEP (essential, lightweight)
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "1"
rusqlite = { version = "0.38", features = ["bundled"] }  # No external DB
reqwest = { version = "0.12", features = ["rustls-tls"] }  # No OpenSSL
axum = "0.8"                    # Only if HTTP gateway needed
clap = { version = "4", features = ["derive"] }
anyhow = "1"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
chacha20poly1305 = "0.10"       # Secret encryption
directories = "5"               # Platform paths
keyring = "3"                   # OS keyring (already in project)

# AVOID (too heavy for desktop app)
# postgresql/diesel - Use rusqlite instead (ZeroClaw lesson)
# playwright/fantoccini - Browser automation adds huge weight
# Full WASM runtime - Defer to later phase
# opentelemetry - Overkill for desktop; use tracing only
# sharp/image-heavy - Use platform-native image handling
```

### 5.4 Performance Optimization Strategies

**Build profile (from ZeroClaw):**
```toml
[profile.release]
opt-level = "z"       # Size optimization (critical for desktop)
lto = true            # Link-time optimization
codegen-units = 1     # Better optimization, slower build
strip = true          # Remove debug symbols
panic = "abort"       # Smaller binary, no unwind tables
```

**Runtime optimizations (combined learnings):**
- **Lazy initialization**: Don't create providers until first use
- **Connection warmup**: Pre-establish TLS for configured providers (ZeroClaw `warmup()`)
- **LRU embedding cache**: Avoid re-computing embeddings (ZeroClaw: 10,000 entry default)
- **History trimming**: Cap conversation at 50 messages (ZeroClaw) or 20 tool iterations (PicoClaw)
- **SQLite for everything**: Vector search, FTS5, session storage - zero network latency
- **Memory hygiene**: Auto-archive old sessions (ZeroClaw: 7-day archive, 30-day purge)

### 5.5 Frontend/Backend Separation (Tauri-Specific)

```
Frontend (WebView)              Backend (Rust Core)
┌─────────────────┐            ┌──────────────────────┐
│ React + Zustand  │──invoke──▶│ Tauri Commands Layer  │
│                  │◀─events──│ (thin IPC boundary)    │
│ • Chat UI        │           ├──────────────────────┤
│ • Settings       │           │ Agent Loop            │
│ • Memory Search  │           │ Provider Manager      │
│ • Channel Status │           │ Memory System         │
└─────────────────┘            │ Security Policy       │
                               │ Config Manager        │
                               └──────────────────────┘
```

**Key principle from all four projects**: The frontend should be a thin presentation layer. All business logic (agent loop, provider management, memory, security) lives in Rust. The Tauri IPC boundary is the **only** interface between them.

### 5.6 What NOT to Do

| Anti-Pattern | Seen In | Why to Avoid |
|--------------|---------|--------------|
| 60+ npm runtime deps | OpenClaw | Bundle bloat, security surface |
| External database requirement | IronClaw (PostgreSQL) | Deployment friction for desktop app |
| Monolithic config blob | PicoClaw | Hard to extend; prefer modular TOML sections |
| No security layer | PicoClaw | Desktop apps need path/command validation too |
| Docker as sandbox | OpenClaw | Not appropriate for desktop; too heavy |
| No test coverage for security | PicoClaw | Security code needs the most testing |
| Re-export everything | OpenClaw | Use focused prelude like IronClaw |

### 5.7 Implementation Priority Matrix

| Priority | Pattern | Source | Impact |
|----------|---------|--------|--------|
| P0 | Trait-based providers/tools/memory | ZeroClaw | Foundation of extensibility |
| P0 | SQLite-only storage (no external DB) | ZeroClaw + PicoClaw | Deployment simplicity |
| P0 | Release profile optimization | ZeroClaw | 3-5x smaller binary |
| P1 | ReliableProvider retry/fallback | ZeroClaw | Production robustness |
| P1 | Security policy (path + command) | ZeroClaw | Desktop app safety |
| P1 | Hybrid memory search (vector + BM25) | ZeroClaw + IronClaw | Quality of recall |
| P2 | Atomic config saves | ZeroClaw | Crash safety |
| P2 | Dual tool-call parsing | ZeroClaw | Broad LLM compatibility |
| P2 | WASM extensibility | IronClaw | Future plugin system |
| P3 | Channel adapter pattern | OpenClaw (concept) | Multi-platform messaging |
| P3 | Self-repair / stuck detection | IronClaw | Agent reliability |

---

## 6. Conclusion

The strongest architectural foundation comes from **ZeroClaw's trait-based, security-first Rust approach** combined with **PicoClaw's ruthless minimalism**. The existing TauriClaw project already has many of the right building blocks (provider pattern, commands layer, services layer). The key additions would be:

1. **ReliableProvider wrapper** for retry + fallback across all LLM providers
2. **Hybrid memory search** in SQLite (vector + FTS5/BM25)
3. **Multi-layered security policy** adapted for desktop context
4. **Trait-based extensibility** for providers, channels, tools, and memory backends
5. **Aggressive release profile** for minimal binary size

---

*Analysis conducted: February 2026*
*Sources: GitHub repositories, web search, code analysis*
