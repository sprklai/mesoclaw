# Zenii — AI Backend for Agents

> **20 MB binary. 114 API routes. 18 tools. Persistent memory. One shared brain.**
>
> Zenii is a local AI backend running at `localhost:18981`. Any tool on your machine — scripts, cron jobs, bots, other AI agents — can use it for persistent memory, AI chat, tool execution, scheduling, and more via simple HTTP calls.

**This file tells AI agents how to use Zenii.** If you're Claude Code, Cursor, Gemini CLI, Windsurf, Codex, or any AI coding agent: read this to learn what Zenii can do for you.

---

## Quick Check: Is Zenii Running?

```bash
curl -s http://localhost:18981/health | jq .
# → {"status":"ok","version":"0.1.8","uptime_secs":1234}
```

No auth required for `/health`. If this fails, Zenii isn't running. Start it with `zenii-daemon` or open the desktop app.

## Authentication

Optional. If `gateway_auth_token` is set in `config.toml`, include:

```bash
curl -H "Authorization: Bearer YOUR_TOKEN" http://localhost:18981/...
```

If not configured (default for local use), no auth needed. WebSocket auth uses `?token=YOUR_TOKEN` query parameter.

---

## For AI Coding Agents

### Core Capabilities

| Category | Key Endpoints | What It Does |
|----------|--------------|--------------|
| **Memory** | `POST /memory`, `GET /memory?q=` | Store and recall facts. Persists across sessions and restarts. Semantic search via FTS5 + vector embeddings. |
| **Chat** | `POST /chat`, `GET /ws/chat` | Send prompts to Zenii's AI with access to all tools and memory. Streaming via WebSocket. |
| **Tools** | `GET /tools`, `POST /tools/{name}/execute` | Discover and execute 18 built-in tools (web search, file ops, shell, process, patch, etc.). |
| **Sessions** | `POST /sessions`, `GET /sessions/{id}/messages` | Manage conversation context. Messages persist. |
| **Scheduler** | `POST /scheduler/jobs`, `GET /scheduler/jobs` | Create cron, interval, or one-time tasks that run AI agent turns autonomously. |
| **System** | `GET /health`, `GET /models`, `GET /config` | Health checks, available AI models, current configuration. |

### Memory — Store & Recall (Most Useful for Agents)

Zenii's persistent memory is its killer feature for agent integration. Store facts from one context, recall them from another — across sessions, tools, and interfaces.

```bash
# Store a memory
curl -X POST http://localhost:18981/memory \
  -H "Content-Type: application/json" \
  -d '{"key": "prod-db", "content": "Production DB is on port 5434, accessed via ssh prod", "category": "core"}'

# Recall by semantic search
curl "http://localhost:18981/memory?q=database+port&limit=3" | jq .

# Recall by exact key
curl http://localhost:18981/memory/prod-db | jq .

# Update a memory
curl -X PUT http://localhost:18981/memory/prod-db \
  -H "Content-Type: application/json" \
  -d '{"content": "Production DB moved to port 5435 on 2026-03-31", "category": "core"}'

# Forget a memory
curl -X DELETE http://localhost:18981/memory/prod-db
```

**Categories**: `core` (long-term facts), `daily` (date-stamped), `conversation` (session-specific), or any custom string.

### Chat — Delegate Reasoning

```bash
# Simple chat (non-streaming, returns full response)
curl -X POST http://localhost:18981/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt": "What port is prod DB on?", "session_id": "ops"}'
# → {"response": "Based on my memory, production DB is on port 5434...", "session_id": "ops"}

# With model override
curl -X POST http://localhost:18981/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Summarize this file", "model": "claude-sonnet-4-6"}'

# With agent delegation (multi-step tool use)
curl -X POST http://localhost:18981/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Search the web for Rust MCP crates and summarize the top 3", "delegation": true}'
```

**WebSocket streaming**: Connect to `ws://localhost:18981/ws/chat?token=TOKEN` and send:
```json
{"prompt": "Your question", "session_id": "optional-id"}
```
Receive streamed `{"type": "text", "content": "..."}` messages, ending with `{"type": "done"}`.

### Tools — Discover & Execute

```bash
# List all available tools with their JSON schemas
curl http://localhost:18981/tools | jq '.[].name'
# → ["system_info", "web_search", "file_read", "file_write", "file_list",
#    "file_search", "content_search", "shell", "process", "patch",
#    "learn", "propose_skill_change", "memory", "agent_notes", "config",
#    "channel_send", "scheduler", "workflows"]

# Execute a specific tool
curl -X POST http://localhost:18981/tools/web_search/execute \
  -H "Content-Type: application/json" \
  -d '{"args": {"query": "Rust async patterns 2026"}}'

# Execute shell command
curl -X POST http://localhost:18981/tools/shell/execute \
  -H "Content-Type: application/json" \
  -d '{"args": {"command": "df -h"}}'

# Read a file
curl -X POST http://localhost:18981/tools/file_read/execute \
  -H "Content-Type: application/json" \
  -d '{"args": {"path": "/etc/hostname"}}'
```

### Sessions

```bash
# Create a session
curl -X POST http://localhost:18981/sessions \
  -H "Content-Type: application/json" \
  -d '{"title": "Deploy planning"}'

# List sessions
curl http://localhost:18981/sessions | jq '.[].title'

# Get messages from a session
curl http://localhost:18981/sessions/SESSION_ID/messages | jq .
```

### Scheduler

```bash
# Create a cron job (daily at 9 AM)
curl -X POST http://localhost:18981/scheduler/jobs \
  -H "Content-Type: application/json" \
  -d '{"name": "morning-briefing", "schedule": {"Cron": {"expr": "0 9 * * *"}}, "payload": {"AgentTurn": {"prompt": "Summarize system status and send to Telegram"}}}'

# List active jobs
curl http://localhost:18981/scheduler/jobs | jq .

# Delete a job
curl -X DELETE http://localhost:18981/scheduler/jobs/JOB_ID
```

### Workflows

```bash
# Create a multi-step workflow
curl -X POST http://localhost:18981/workflows \
  -H "Content-Type: application/json" \
  -d '{
    "name": "research-and-notify",
    "description": "Search web, summarize, send to Telegram",
    "steps": [
      {"name": "search", "type": "tool", "tool_name": "web_search", "args": {"query": "{{input.topic}}"}},
      {"name": "summarize", "type": "llm", "prompt": "Summarize: {{steps.search.output}}"},
      {"name": "notify", "type": "tool", "tool_name": "channel_send", "args": {"action": "send", "channel": "telegram", "content": "{{steps.summarize.output}}"}}
    ]
  }'

# Run a workflow
curl -X POST http://localhost:18981/workflows/WORKFLOW_ID/run \
  -H "Content-Type: application/json" \
  -d '{"input": {"topic": "Rust MCP 2026"}}'
```

---

## A2A Protocol (Agent-to-Agent)

[A2A](https://github.com/a2aproject/A2A) is Google's open protocol (Linux Foundation, 22.9K stars) for AI agents to discover and collaborate with each other. MCP handles agent-to-tool communication; A2A handles agent-to-agent communication.

Zenii can advertise its capabilities via an **Agent Card** — a JSON document served at `/.well-known/agent.json` that tells other agents what Zenii can do.

### Zenii Agent Card

```json
{
  "name": "Zenii",
  "description": "Local AI backend with persistent memory, 18 tools, and 114 API routes. Every tool on your machine shares one AI brain. Store facts, recall them semantically, execute tools, schedule tasks, and chat with AI — all via localhost:18981.",
  "url": "http://localhost:18981",
  "version": "0.1.8",
  "provider": {
    "organization": "SprklAI",
    "url": "https://zenii.sprklai.com"
  },
  "capabilities": {
    "streaming": true,
    "pushNotifications": true,
    "stateTransitionHistory": false
  },
  "authentication": {
    "schemes": ["Bearer"],
    "credentials": "Optional. Set gateway_auth_token in config.toml. If unset, no auth required for local use."
  },
  "defaultInputModes": ["text/plain", "application/json"],
  "defaultOutputModes": ["text/plain", "application/json"],
  "skills": [
    {
      "id": "memory-store",
      "name": "Store Memory",
      "description": "Persistently store a fact, note, or context with a unique key. Survives restarts. Searchable via FTS5 full-text search and optional vector embeddings.",
      "tags": ["memory", "persistence", "knowledge"],
      "examples": ["Remember that prod DB is on port 5434", "Store the deploy checklist for project X"]
    },
    {
      "id": "memory-recall",
      "name": "Recall Memory",
      "description": "Search stored memories using natural language. Returns semantically relevant matches ranked by hybrid FTS5 + cosine vector similarity.",
      "tags": ["memory", "search", "recall", "semantic"],
      "examples": ["What port is the production database on?", "Find all memories about deploy procedures"]
    },
    {
      "id": "chat",
      "name": "AI Chat with Tools",
      "description": "Send a prompt to Zenii's AI agent. The agent has access to all 18 tools, persistent memory, and 6+ AI providers (OpenAI, Anthropic, Google Gemini, OpenRouter, Vercel AI Gateway, Ollama). Supports multi-step tool use via delegation.",
      "tags": ["chat", "reasoning", "delegation", "multi-provider"],
      "examples": ["Search the web for Rust async patterns and summarize", "What files changed in the last git commit?"]
    },
    {
      "id": "tool-execute",
      "name": "Execute Tool",
      "description": "Run any of 18 built-in tools directly: web_search, file_read, file_write, file_list, file_search, content_search, shell, process, patch, system_info, memory, learn, agent_notes, propose_skill_change, config, channel_send, scheduler, workflows.",
      "tags": ["tools", "execution", "automation", "shell", "files"],
      "examples": ["Execute a shell command", "Read a file", "Search the web"]
    },
    {
      "id": "schedule-task",
      "name": "Schedule Autonomous Task",
      "description": "Create cron, interval, or one-time scheduled jobs that execute AI agent turns autonomously. Jobs persist across restarts.",
      "tags": ["scheduling", "automation", "cron", "recurring"],
      "examples": ["Every morning at 9am, summarize system status", "In 30 minutes, check if the build passed"]
    },
    {
      "id": "channel-message",
      "name": "Send Channel Message",
      "description": "Send messages to Telegram, Slack, or Discord via configured channel integrations.",
      "tags": ["messaging", "channels", "notifications", "telegram", "slack", "discord"],
      "examples": ["Send 'deploy complete' to Telegram", "Post build results to Slack"]
    }
  ]
}
```

### A2A Task Lifecycle

When another agent delegates a task to Zenii via A2A:

```
Client Agent                         Zenii (A2A Server)
     |                                      |
     |-- POST /tasks/send ----------------->|  (task: "Remember that API key rotates monthly")
     |                                      |  → state: submitted → working
     |<-- 200 {state: "completed"} ---------|  (Zenii stores the memory, confirms)
     |                                      |
     |-- POST /tasks/send ----------------->|  (task: "What do you know about API keys?")
     |                                      |  → recalls from memory
     |<-- 200 {state: "completed", ...} ----|  (returns recalled facts)
```

**Status**: A2A Agent Card is defined above. Gateway endpoint for `/.well-known/agent.json` and task lifecycle routes (`/tasks/send`, `/tasks/sendSubscribe`) are planned for a future release.

---

## MCP Integration (Model Context Protocol)

[MCP](https://modelcontextprotocol.io/) is Anthropic's protocol for connecting AI agents to tools and data sources. Zenii can act as both an **MCP server** (exposing its tools to Claude Code, Cursor, etc.) and an **MCP client** (consuming tools from external MCP servers).

### Zenii as MCP Server

Zenii's 18 tools map directly to MCP tool definitions. The [`rmcp`](https://crates.io/crates/rmcp) crate (official Rust MCP SDK) provides the `#[tool]` macro for zero-boilerplate tool exposure over stdio and HTTP transports.

**MCP Tool Definitions** (what Zenii exposes to MCP clients):

```json
[
  {
    "name": "zenii_memory_store",
    "description": "Persistently store a fact in Zenii's memory. Survives restarts. Searchable later via semantic recall.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "key": {"type": "string", "description": "Unique identifier for this memory"},
        "content": {"type": "string", "description": "The content to remember"},
        "category": {"type": "string", "enum": ["core", "daily", "conversation"], "description": "Memory category (default: core)"}
      },
      "required": ["key", "content"]
    }
  },
  {
    "name": "zenii_memory_recall",
    "description": "Search Zenii's persistent memory using natural language. Returns semantically relevant matches.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "query": {"type": "string", "description": "Natural language search query"},
        "limit": {"type": "integer", "description": "Maximum results to return (default: 5)"}
      },
      "required": ["query"]
    }
  },
  {
    "name": "zenii_chat",
    "description": "Send a prompt to Zenii's AI agent. Has access to 18 tools, persistent memory, and 6+ AI providers.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "prompt": {"type": "string", "description": "The message to send"},
        "session_id": {"type": "string", "description": "Optional session ID for context continuity"},
        "model": {"type": "string", "description": "Optional model override (e.g. claude-sonnet-4-6, gpt-4o)"}
      },
      "required": ["prompt"]
    }
  },
  {
    "name": "zenii_web_search",
    "description": "Search the web via Zenii's cascading provider (Tavily, Brave, DuckDuckGo).",
    "inputSchema": {
      "type": "object",
      "properties": {
        "query": {"type": "string", "description": "Search query"}
      },
      "required": ["query"]
    }
  },
  {
    "name": "zenii_tool_execute",
    "description": "Execute any of Zenii's 18 built-in tools by name. Use GET /tools to discover available tools and their schemas.",
    "inputSchema": {
      "type": "object",
      "properties": {
        "tool_name": {"type": "string", "description": "Tool name (e.g. shell, file_read, file_search, process, patch, system_info)"},
        "args": {"type": "object", "description": "Tool-specific arguments (see GET /tools for schemas)"}
      },
      "required": ["tool_name", "args"]
    }
  }
]
```

### Zenii as MCP Client

[rig-core](https://crates.io/crates/rig-core) (Zenii's AI framework) already supports MCP via the `rmcp` feature flag. When enabled, Zenii can consume tools from any MCP server — GitHub, Postgres, Slack, filesystem servers, etc.

```toml
# In Cargo.toml (future):
rig-core = { version = "0.33", features = ["rmcp"] }
```

This turns Zenii into an MCP hub: a single agent that orchestrates tools from the entire MCP ecosystem (1000+ servers) alongside its own 18 built-in tools.

### Claude Code Configuration

To use Zenii as a tool source in Claude Code, add to your project's `.mcp.json`:

```json
{
  "mcpServers": {
    "zenii": {
      "command": "zenii-mcp-server",
      "args": ["--transport", "stdio"]
    }
  }
}
```

Or use Zenii's HTTP API directly via Claude Code hooks in `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "memory_store|memory_recall",
        "hooks": [
          {
            "type": "command",
            "command": "curl -s -X POST http://localhost:18981/memory -H 'Content-Type: application/json' -d '{\"key\": \"$KEY\", \"content\": \"$CONTENT\"}'"
          }
        ]
      }
    ]
  }
}
```

### Cursor Configuration

Add to `.cursorrules` in your project:

```
When you need to store information for later recall, use Zenii's memory API:
- Store: curl -X POST http://localhost:18981/memory -H "Content-Type: application/json" -d '{"key": "...", "content": "..."}'
- Recall: curl http://localhost:18981/memory?q=QUERY
- Tools: curl http://localhost:18981/tools | jq '.[].name'
- Chat: curl -X POST http://localhost:18981/chat -H "Content-Type: application/json" -d '{"prompt": "..."}'

Zenii runs locally at localhost:18981 with 18 tools, persistent memory, and 6+ AI providers.
```

---

## Example Workflows

### 1. Agent Stores Project Context, Recalls Later

```bash
# During code review — store a finding
curl -X POST http://localhost:18981/memory \
  -H "Content-Type: application/json" \
  -d '{"key": "review-auth-bug", "content": "Auth middleware skips validation for /api/internal/* routes. Needs fix before deploy.", "category": "core"}'

# Later, in a different session or tool:
curl -X POST http://localhost:18981/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Are there any known security issues I should fix before deploying?"}'
# → Zenii recalls the auth middleware finding from memory
```

### 2. Agent Delegates Web Research

```bash
# Ask Zenii to research and summarize (uses web_search + LLM)
curl -X POST http://localhost:18981/chat \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Search for the latest Rust MCP crates and compare rmcp vs mcp-core", "delegation": true}'
```

### 3. Multi-Agent via Shared Memory (A2A Pattern)

```bash
# Agent A (code reviewer) stores findings
curl -X POST http://localhost:18981/memory \
  -H "Content-Type: application/json" \
  -d '{"key": "perf-issue-db", "content": "N+1 query in user list endpoint. ~200ms per request.", "category": "core"}'

# Agent B (performance optimizer) recalls and acts
curl "http://localhost:18981/memory?q=performance+issues&limit=5"
# → Returns the N+1 finding. Agent B can now fix it with full context.
```

---

## Full API Reference

- **Interactive docs**: `http://localhost:18981/api-docs` (Scalar UI, requires `api-docs` feature)
- **OpenAPI 3.0 spec**: `http://localhost:18981/api-docs/openapi.json`
- **Documentation site**: [docs.zenii.sprklai.com](https://docs.zenii.sprklai.com)
- **GitHub**: [github.com/sprklai/zenii](https://github.com/sprklai/zenii)

### All 114 Routes (Summary)

| Group | Routes | Key Endpoints |
|-------|--------|---------------|
| System | 3 | `/health`, `/system/info`, `/setup/status` |
| Sessions | 6 | CRUD + generate-title |
| Messages | 3 | Get, send, delete-from |
| Chat | 1 | `POST /chat` |
| Memory | 5 | Store, recall, read, update, forget |
| Config | 3 | Get, update, get-file |
| Credentials | 5 | CRUD for API keys |
| Providers | 11 | Multi-provider management + test |
| Tools | 2 | List, execute |
| Permissions | 4 | Surface-level access control |
| Models | 1 | List available models |
| Identity | 4 | Soul/persona management |
| Skills | 6 | Prompt template CRUD + reload |
| Skill Proposals | 4 | Self-evolution proposals |
| User | 6 | Profile + observations |
| Embeddings | 5 | Status, test, embed, download, reindex |
| Plugins | 9 | Install, config, toggle |
| Channels | 10 | Telegram/Slack/Discord messaging |
| Scheduler | 7 | Cron job management |
| Workflows | 10 | DAG pipeline CRUD + execution |
| Agents | 2 | Active agent listing + cancel |
| Approvals | 3 | Human-in-the-loop approval |
| WebSocket | 2 | `/ws/chat`, `/ws/notifications` |
| API Docs | 2 | Scalar UI + OpenAPI JSON |
