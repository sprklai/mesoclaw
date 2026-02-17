# Design: Sidecar Modularity Architecture

> Modular sidecar system for Docker/Podman, Python, Node.js integration with MCP protocol support.
> Brainstormed and approved: February 16, 2026

---

## Decisions

| Decision                | Choice                           | Rationale                                                                        |
| ----------------------- | -------------------------------- | -------------------------------------------------------------------------------- |
| Sidecar role            | Both Tools + Services            | Lightweight sidecars as on-demand Tools, heavy sidecars as long-lived Services   |
| Primary protocol        | Stdin/Stdout JSON                | Simple, no networking needed, works with any language                            |
| Standard protocol       | MCP (JSON-RPC over stdin/stdout) | Industry standard for AI tool servers; enables Composio.dev integration for free |
| Service protocol        | HTTP REST                        | Long-lived services expose REST endpoints on localhost                           |
| Container scope         | Container as Tool runtime        | Isolated execution environment, not full orchestration                           |
| Container preference    | Podman > Docker > Native         | Auto-detect, prefer rootless Podman, fallback chain                              |
| Module discovery        | Manifest-based (TOML)            | Filesystem convention + typed manifests in `~/.mesoclaw/modules/`                |
| Architecture modularity | Cargo feature flags              | Compile-time module selection, minimal to full builds                            |
| Trait design            | Extends existing Tool trait      | SidecarModule extends Tool, registers in ToolRegistry — no agent loop changes    |

---

## 1. Module Type Hierarchy

Three module types, all unified under the existing Tool system:

```
┌─────────────────────────────────────────────────────────────────┐
│                     ToolRegistry (existing)                     │
├─────────────────────────────────────────────────────────────────┤
│  Built-in Tools          │  Sidecar Modules                    │
│  ├─ shell                │  ├─ SidecarTool (on-demand)         │
│  ├─ file_read            │  │   spawn → execute → exit         │
│  ├─ file_write           │  │   Protocol: stdin/stdout JSON    │
│  ├─ file_list            │  ├─ SidecarService (long-lived)     │
│  ├─ memory_store         │  │   start → HTTP calls → stop      │
│  ├─ memory_recall        │  │   Protocol: HTTP REST             │
│  ├─ memory_forget        │  └─ McpServer (MCP protocol)        │
│  └─ spawn_agent          │      start → JSON-RPC → stop        │
│                          │      Protocol: MCP over stdin/stdout │
└─────────────────────────────────────────────────────────────────┘
```

| Module Type      | Lifecycle                  | Protocol                         | Use Case                                |
| ---------------- | -------------------------- | -------------------------------- | --------------------------------------- |
| `SidecarTool`    | On-demand (spawn per call) | Stdin/Stdout JSON                | Python script, Node.js script, CLI tool |
| `SidecarService` | Long-lived (start once)    | HTTP REST                        | Python API server, Node.js service      |
| `McpServer`      | Long-lived (start once)    | MCP (JSON-RPC over stdin/stdout) | Composio tools, any MCP server          |

---

## 2. Trait Architecture

```rust
/// Extends existing Tool trait for sidecar-specific behavior
#[async_trait]
pub trait SidecarModule: Tool + Send + Sync {
    /// What kind of module: Tool, Service, or McpServer
    fn module_type(&self) -> ModuleType;

    /// Execution runtime: Native, Docker, or Podman
    fn runtime(&self) -> Runtime;

    /// Check if the module is healthy and responsive
    async fn health_check(&self) -> Result<HealthStatus>;

    /// Start the module (no-op for on-demand SidecarTool)
    async fn start(&self) -> Result<()>;

    /// Stop the module (no-op for on-demand SidecarTool)
    async fn stop(&self) -> Result<()>;
}

/// Container runtime abstraction — Docker and Podman are interchangeable
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Check if this runtime is installed and accessible
    async fn is_available(&self) -> bool;

    /// Pull a container image
    async fn pull_image(&self, image: &str) -> Result<()>;

    /// Run a container with the given configuration
    async fn run(&self, config: &ContainerConfig) -> Result<ContainerHandle>;

    /// Stop a running container
    async fn stop(&self, handle: &ContainerHandle) -> Result<()>;

    /// Execute a command inside a running container
    async fn exec(&self, handle: &ContainerHandle, cmd: &[&str]) -> Result<Output>;
}

// Implementations: DockerRuntime, PodmanRuntime
// Auto-detection: prefer Podman (rootless) → Docker → native fallback
```

### Key Types

```rust
pub enum ModuleType {
    Tool,       // On-demand, spawn per invocation
    Service,    // Long-lived HTTP server
    McpServer,  // Long-lived MCP protocol server
}

pub enum Runtime {
    Native,     // Direct process spawn
    Docker,     // Docker container
    Podman,     // Podman container (preferred)
}

pub enum HealthStatus {
    Healthy,
    Degraded(String),   // Working but with warnings
    Unhealthy(String),  // Not functioning
    NotStarted,         // Service/MCP not yet started
}

pub struct ContainerConfig {
    pub image: String,
    pub command: Vec<String>,
    pub volumes: Vec<(PathBuf, String)>,  // host:container
    pub environment: HashMap<String, String>,
    pub network: bool,          // Enable/disable networking
    pub memory_limit_mb: u64,
    pub timeout_seconds: u64,
}
```

---

## 3. Module Manifest (TOML)

External modules defined via filesystem convention with typed manifests:

```
~/.mesoclaw/modules/
├── python-analyst/
│   ├── manifest.toml       ← Module definition
│   ├── main.py             ← Entry point
│   └── requirements.txt    ← Dependencies
├── node-transformer/
│   ├── manifest.toml
│   ├── index.js
│   └── package.json
└── composio-tools/
    └── manifest.toml       ← MCP server config (no code needed)
```

### SidecarTool Manifest

```toml
[module]
name = "python-analyst"
version = "0.1.0"
type = "tool"                    # "tool" | "service" | "mcp"
description = "Analyze data with Python pandas"

[runtime]
type = "native"                  # "native" | "docker" | "podman"
command = "python3"
args = ["main.py"]
working_dir = "."                # Relative to module directory

[tool]
# JSON Schema for the tool (what the agent sees)
input_schema = '''
{
  "type": "object",
  "properties": {
    "data_path": { "type": "string", "description": "Path to CSV/JSON data" },
    "query": { "type": "string", "description": "Analysis question" }
  },
  "required": ["query"]
}
'''

[security]
allowed_paths = ["~/data", "/tmp"]
network = false
timeout_seconds = 120
max_memory_mb = 512
```

### SidecarTool with Container Runtime

```toml
[module]
name = "python-analyst"
version = "0.1.0"
type = "tool"
description = "Analyze data with Python pandas (containerized)"

[runtime]
type = "docker"                  # or "podman"
image = "python:3.12-slim"
volumes = ["./:/app", "~/data:/data:ro"]
command = "python3"
args = ["/app/main.py"]

[tool]
input_schema = '''
{
  "type": "object",
  "properties": {
    "data_path": { "type": "string", "description": "Path to data file" },
    "query": { "type": "string", "description": "Analysis question" }
  },
  "required": ["query"]
}
'''

[security]
network = false
timeout_seconds = 120
max_memory_mb = 512
```

### SidecarService Manifest

```toml
[module]
name = "ml-inference"
version = "0.1.0"
type = "service"
description = "ML model inference service"

[runtime]
type = "docker"
image = "my-ml-service:latest"
ports = [8081]                    # Exposed port

[service]
health_endpoint = "/health"
execute_endpoint = "/predict"
startup_timeout_seconds = 30

[tool]
input_schema = '''
{
  "type": "object",
  "properties": {
    "input_data": { "type": "string", "description": "JSON input for prediction" }
  },
  "required": ["input_data"]
}
'''

[security]
network = true                   # Services typically need network
timeout_seconds = 60
max_memory_mb = 2048
```

### McpServer Manifest

```toml
[module]
name = "composio-tools"
version = "0.1.0"
type = "mcp"
description = "Composio MCP tools (Gmail, GitHub, Slack, etc.)"

[runtime]
type = "native"
command = "npx"
args = ["-y", "composio-core", "mcp-server"]

[mcp]
# Expose specific tools or all discovered tools
expose_tools = ["gmail_send", "github_create_issue", "slack_post"]
# Alternative: expose_all = true

[security]
network = true                   # MCP servers need network for external APIs
timeout_seconds = 300
```

---

## 4. Communication Protocols

### Stdin/Stdout JSON (SidecarTool)

Simple request/response over process stdin/stdout:

```
Mesoclaw → Sidecar (stdin):
{"id": "req-1", "method": "execute", "params": {"data_path": "sales.csv", "query": "top 5 products"}}

Sidecar → Mesoclaw (stdout):
{"id": "req-1", "result": {"summary": "Top 5 products by revenue: ...", "chart_data": [...]}}

Error response:
{"id": "req-1", "error": {"code": -1, "message": "File not found"}}

Progress (optional):
{"id": "req-1", "progress": {"percent": 45, "message": "Processing rows..."}}
```

Sidecar authors implement a simple loop:

```python
# Python sidecar template
import json, sys

for line in sys.stdin:
    request = json.loads(line)
    try:
        result = handle(request["params"])
        response = json.dumps({"id": request["id"], "result": result})
        print(response, flush=True)
    except Exception as e:
        error_resp = json.dumps({"id": request["id"], "error": {"code": -1, "message": str(e)}})
        print(error_resp, flush=True)
```

### MCP (JSON-RPC over stdin/stdout)

Standard Model Context Protocol. Mesoclaw acts as MCP client:

1. `initialize` → negotiate capabilities
2. `tools/list` → discover available tools
3. `tools/call` → execute a specific tool
4. Tools auto-registered in ToolRegistry with `mcp:{module_name}:{tool_name}` naming

This means any MCP-compatible server (Composio, custom servers) works without modification.

### HTTP REST (SidecarService)

Long-lived services expose REST endpoints:

```
POST http://localhost:{port}/execute
Content-Type: application/json

{"params": {"input_data": "..."}}

Response:
{"result": {"prediction": 0.95, "confidence": "high"}}
```

Health check: `GET http://localhost:{port}/health`

---

## 5. Container Runtime Integration

### Execution Flow

```
Agent requests "python-analyst" tool
        │
        ▼
   ToolRegistry.get("python-analyst")
        │
        ▼
   Check runtime type from manifest
        │
   ┌────┴────────────┐
   │ container        │ native
   │                  │
   ▼                  ▼
ContainerRuntime     tokio::process::Command
   .run(config)         .spawn()
   │                    │
   ▼                    ▼
Container with:      Direct process with:
- volume mounts      - Working directory
- memory limits      - Environment vars
- network policy     - Path restrictions
- timeout            - Timeout (tokio::time)
```

### Auto-Detection Priority

1. Check if `podman` binary exists and works → use `PodmanRuntime`
2. Check if `docker` binary exists and works → use `DockerRuntime`
3. Neither available → fall back to native runtime (log warning)
4. User override in `config.toml`:

```toml
[modules]
preferred_runtime = "podman"   # "podman" | "docker" | "native"
```

### Container Lifecycle for SidecarTool (on-demand)

```
1. Agent calls tool
2. ContainerRuntime.run(config)  — creates + starts container
3. Write request to container stdin
4. Read response from container stdout
5. ContainerRuntime.stop(handle) — stops + removes container
6. Return result to agent
```

### Container Lifecycle for SidecarService (long-lived)

```
1. On startup: ContainerRuntime.run(config) — starts service container
2. Wait for health_endpoint to respond 200
3. Register tool in ToolRegistry
4. Agent calls: HTTP POST to execute_endpoint
5. On shutdown: ContainerRuntime.stop(handle)
```

---

## 6. Module Registry & Lifecycle

### Startup Sequence

```
1. Scan ~/.mesoclaw/modules/ for directories with manifest.toml
2. Parse and validate each manifest (reject malformed)
3. Classify by type:
   a. SidecarTool → register in ToolRegistry (lazy, not started)
   b. SidecarService → start process/container, wait for health, register tools
   c. McpServer → start process, run initialize + tools/list, register discovered tools
4. Log registered modules and any failures
5. Emit ModulesReady event on EventBus
```

### Runtime Execution

- **SidecarTool**: Agent calls tool → spawn process/container → send request → receive response → kill process/container → return to agent
- **SidecarService**: Agent calls tool → HTTP POST to running service → return response
- **McpServer**: Agent calls tool → JSON-RPC `tools/call` to running server → return response

### Shutdown Sequence

```
1. Stop all SidecarService instances (graceful shutdown with timeout)
2. Stop all McpServer instances
3. Kill any lingering SidecarTool processes
4. Clean up container resources
```

### Hot Reload (v2)

Watch `~/.mesoclaw/modules/` for changes:

- New module directory → register
- Manifest change → restart module
- Directory removed → deregister and stop

---

## 7. Architecture Modularity (Feature Flags)

The entire Mesoclaw system is modular via Cargo feature flags:

```toml
[features]
default = ["core", "cli", "desktop"]

# Core (always required)
core = []                              # Agent, tools, memory, security, config

# Entry points
cli = ["dep:clap", "dep:rustyline"]    # CLI binary
desktop = ["dep:tauri"]                # Tauri GUI binary
gateway = ["dep:axum"]                 # HTTP/WebSocket gateway

# Sidecar system
sidecars = []                          # Module system, manifest parsing, stdio protocol
containers = ["sidecars"]              # Docker/Podman container runtime
mcp-client = ["sidecars"]             # MCP protocol client

# Channels
channels-telegram = []                 # Telegram integration
channels-webhook = []                  # Webhook integration

# Optional systems
scheduler = []                         # Cron/heartbeat scheduler
memory-vector = []                     # Vector search (qdrant)
```

### Build Profiles

| Profile         | Features                                                            | Use Case                   |
| --------------- | ------------------------------------------------------------------- | -------------------------- |
| Minimal CLI     | `core, cli, gateway`                                                | Lightweight CLI agent      |
| Full Desktop    | `default, sidecars, containers, mcp-client, scheduler`              | Full-featured desktop app  |
| Headless Server | `core, gateway, sidecars, mcp-client, scheduler, channels-telegram` | Server deployment          |
| Developer       | `default, sidecars, mcp-client`                                     | Development with MCP tools |

### Module Dependency Tree

```
core (always)
├── gateway (required by cli + desktop for HTTP/WS communication)
│   ├── cli
│   └── desktop
├── sidecars (optional)
│   ├── containers (optional, adds Docker/Podman)
│   └── mcp-client (optional, adds MCP protocol)
├── channels-telegram (optional)
├── channels-webhook (optional)
├── scheduler (optional)
└── memory-vector (optional)
```

---

## 8. File Structure

### New Rust Modules

```
src-tauri/src/
├── modules/                          ← NEW: Sidecar module system
│   ├── mod.rs                       ← ModuleRegistry, discovery, lifecycle management
│   ├── manifest.rs                  ← TOML manifest parsing and validation
│   ├── sidecar_tool.rs              ← SidecarTool: on-demand process spawning
│   ├── sidecar_service.rs           ← SidecarService: long-lived HTTP service management
│   ├── mcp_client.rs                ← McpServer: MCP protocol client
│   ├── container/                   ← Container runtime abstraction
│   │   ├── mod.rs                   ← ContainerRuntime trait, auto-detection
│   │   ├── docker.rs                ← DockerRuntime implementation
│   │   └── podman.rs                ← PodmanRuntime implementation
│   └── protocol/                    ← Communication protocol implementations
│       ├── mod.rs                   ← Protocol trait
│       ├── stdio_json.rs            ← Stdin/Stdout JSON protocol
│       ├── mcp.rs                   ← MCP JSON-RPC protocol
│       └── http.rs                  ← HTTP client for services
```

### User-Facing Directory

```
~/.mesoclaw/
├── modules/                         ← User-installed sidecar modules
│   ├── python-analyst/
│   │   ├── manifest.toml
│   │   └── main.py
│   └── composio-tools/
│       └── manifest.toml
├── module-cache/                    ← Container image cache metadata
└── config.toml                      ← [modules] section for global settings
```

---

## 9. Security Model

| Boundary         | Enforcement                        | Mechanism                                                            |
| ---------------- | ---------------------------------- | -------------------------------------------------------------------- |
| Filesystem       | `allowed_paths` in manifest        | Path validation before process spawn; container volume mounts        |
| Network          | `network = false` by default       | Container: `--network=none`; Native: advisory warning                |
| Timeout          | Hard kill after `timeout_seconds`  | `tokio::time::timeout` wrapping process/container                    |
| Memory           | `max_memory_mb` limit              | Container: `--memory` flag; Native: advisory via `ulimit`            |
| Execution        | SecurityPolicy gates module access | Per-identity `tools.toml` controls which modules agents can invoke   |
| Audit            | All invocations logged             | EventBus `ModuleToolStart`/`ModuleToolResult` events → `audit.jsonl` |
| Input validation | Schema enforcement                 | Manifest `input_schema` validated before forwarding to sidecar       |

### Principle of Least Privilege

- Sidecars run with minimal permissions by default
- Container modules: no network, no host filesystem access beyond explicit volumes
- Native modules: restricted to `allowed_paths`, process isolation via OS
- Agent identity profiles control which modules are available per session

---

## 10. Gateway API Extensions

New endpoints for module management:

```
Modules:
  GET    /api/v1/modules                    List registered modules + status
  GET    /api/v1/modules/{name}             Module details + health
  POST   /api/v1/modules/{name}/start       Start a service/MCP module
  POST   /api/v1/modules/{name}/stop        Stop a service/MCP module
  POST   /api/v1/modules/{name}/restart     Restart a module
  GET    /api/v1/modules/{name}/health      Health check
  POST   /api/v1/modules/reload             Re-scan modules directory
```

New WebSocket events:

```json
{"type": "module.started", "name": "composio-tools", "tools_registered": 3}
{"type": "module.stopped", "name": "ml-inference", "reason": "shutdown"}
{"type": "module.health_changed", "name": "composio-tools", "status": "degraded", "message": "..."}
{"type": "module.tool_start", "name": "python-analyst", "session_id": "...", "params": {}}
{"type": "module.tool_result", "name": "python-analyst", "session_id": "...", "success": true}
```

### CLI Commands

```bash
mesoclaw module list                      # List all modules + status
mesoclaw module install <path-or-url>     # Copy module to ~/.mesoclaw/modules/
mesoclaw module remove <name>             # Remove module
mesoclaw module start <name>              # Start service/MCP module
mesoclaw module stop <name>               # Stop module
mesoclaw module health <name>             # Check health
mesoclaw module reload                    # Re-scan modules directory
mesoclaw module create <name> --type tool --runtime python  # Scaffold new module
```

---

## 11. Composio.dev Integration Path

Composio exposes tools via MCP servers. Mesoclaw's MCP client support enables Composio integration with zero custom code:

```
1. User installs Composio CLI: npm install -g composio-core
2. Creates manifest in ~/.mesoclaw/modules/composio/manifest.toml
3. Mesoclaw starts MCP server, discovers tools via tools/list
4. Agent can now call gmail_send, github_create_issue, slack_post, etc.
5. Composio handles OAuth, token management, API calls
```

Any MCP-compatible tool server works the same way — the protocol is standardized.

---

## 12. What Changes in Existing Plan

### New Phase/Tasks Required

| Item                 | Phase                         | Description                                   |
| -------------------- | ----------------------------- | --------------------------------------------- |
| Module system core   | Phase 2 (after Tool registry) | ModuleRegistry, manifest parsing, SidecarTool |
| Container runtime    | Phase 2                       | ContainerRuntime trait + Docker/Podman        |
| MCP client           | Phase 2                       | MCP protocol client, tool discovery           |
| SidecarService       | Phase 4                       | Long-lived service management                 |
| Module CLI commands  | Phase 5                       | `mesoclaw module` subcommands                 |
| Module management UI | Phase 6                       | Frontend UI for module status/management      |
| Cargo feature flags  | Phase 0                       | Restructure crate into feature-gated modules  |

### Modified Existing Items

| Existing Item                     | Change                                         |
| --------------------------------- | ---------------------------------------------- |
| Tool trait + registry (Phase 2.2) | Extended with SidecarModule trait              |
| SecurityPolicy (Phase 2.3)        | Adds module-level access control               |
| EventBus (Phase 2.1)              | New module lifecycle events                    |
| Gateway API                       | New `/api/v1/modules/*` endpoints              |
| CLI commands                      | New `mesoclaw module` subcommand group         |
| Cargo.toml                        | Feature flags for modular compilation          |
| `~/.mesoclaw/`                    | New `modules/` and `module-cache/` directories |

### Tauri v2 Sidecar Compliance Notes

For bundled trusted sidecars shipped with the app, follow Tauri's sidecar model:

1. Declare binaries in `bundle.externalBin` in `src-tauri/tauri.conf.json`.
2. Enable and initialize `tauri-plugin-shell`.
3. Restrict shell capability permissions to explicit sidecar commands/scopes.

User-installed modules in `~/.mesoclaw/modules/` remain backend-managed via process/container runtimes and should not require broad shell capability grants.

### New Dependencies

```toml
# Module system
toml = "1"                      # Manifest parsing (may already exist)
jsonschema = "0.28"             # Input schema validation

# Container runtime
bollard = "0.18"                # Docker API client (optional, feature-gated)
# Podman uses Docker-compatible API, so bollard works for both

# MCP protocol
# JSON-RPC is simple enough to implement with serde_json + tokio::process
# No additional dependency needed
```

---

_Design approved: February 16, 2026_
_References: docs/plans/2026-02-16-cli-gateway-agents-design.md, docs/claw-ecosystem-analysis.md_
