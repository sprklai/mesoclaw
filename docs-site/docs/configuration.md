---
sidebar_position: 2
title: Configuration
slug: /configuration
---

# Zenii Configuration Reference

## Table of Contents

- [File Location](#file-location)
- [Configuration Sections](#configuration-sections)
  - [Gateway](#gateway)
  - [Database](#database)
  - [Memory](#memory)
  - [Security](#security)
  - [AI Agent](#ai-agent)
  - [Identity](#identity)
  - [Skills](#skills)
  - [User Learning](#user-learning)
  - [Tools](#tools)
  - [Web Search](#web-search)
  - [Context Injection](#context-injection)
  - [Prompt Strategy](#prompt-strategy)
  - [Context Management](#context-management)
  - [Embeddings](#embeddings)
  - [Reasoning](#reasoning)
  - [Plugins](#plugins)
  - [Tool Permissions](#tool-permissions)
  - [Channels](#channels)
  - [Scheduler](#scheduler)
  - [Credentials](#credentials)
  - [Self-Evolution](#self-evolution)
  - [Logging](#logging)
- [Environment Variable Overrides](#environment-variable-overrides)
- [Feature Flag Impact](#feature-flag-impact)
- [Example Full Config](#example-full-config)

---

## File Location

Zenii uses the `directories` crate with the reverse-domain identifier `com.sprklai.zenii` to determine platform-correct paths:

| Platform | Config File Path |
|---|---|
| Linux | `~/.config/zenii/config.toml` |
| macOS | `~/Library/Application Support/com.sprklai.zenii/config.toml` |
| Windows | `%APPDATA%\sprklai\zenii\config\config.toml` |

Data files (databases, identity, skills) default to:

| Platform | Data Directory |
|---|---|
| Linux | `~/.local/share/zenii/` |
| macOS | `~/Library/Application Support/com.sprklai.zenii/` |
| Windows | `%APPDATA%\sprklai\zenii\data\` |

If the config file does not exist on startup, Zenii uses all default values.

---

## Configuration Sections

All fields use `serde(default)`, so any field can be omitted to use its default value. The config file format is TOML.

### Gateway

| Field | Type | Default | Description |
|---|---|---|---|
| `gateway_host` | String | `"127.0.0.1"` | IP address the gateway listens on |
| `gateway_port` | u16 | `18981` | Port the gateway listens on |
| `gateway_auth_token` | Option\<String\> | `null` | Bearer token for API authentication. If unset, auth is disabled |
| `gateway_cors_origins` | Vec\<String\> | `["http://localhost:18971"]` | Allowed CORS origins. `["*"]` enables permissive CORS |
| `ws_max_connections` | usize | `32` | Maximum concurrent WebSocket connections |
| `event_bus_capacity` | usize | `256` | Capacity of the tokio broadcast event bus channel |

```toml
gateway_host = "127.0.0.1"
gateway_port = 18981
gateway_auth_token = "my-secret-token"
gateway_cors_origins = ["http://localhost:18971"]
ws_max_connections = 32
event_bus_capacity = 256
```

### Database

| Field | Type | Default | Description |
|---|---|---|---|
| `data_dir` | Option\<String\> | Platform default (see above) | Root directory for all data files |
| `db_path` | Option\<String\> | `{data_dir}/zenii.db` | Path to main SQLite database (app + FTS5) |
| `memory_db_path` | Option\<String\> | `{data_dir}/memory_vec.db` | Path to vector memory SQLite database (sqlite-vec) |
| `session_max_age_days` | u32 | `90` | Days before old sessions are automatically cleaned up on boot |

```toml
data_dir = "/home/user/.zenii"
db_path = "/home/user/.zenii/zenii.db"
memory_db_path = "/home/user/.zenii/memory_vec.db"
session_max_age_days = 90
```

### Memory

| Field | Type | Default | Description |
|---|---|---|---|
| `memory_fts_weight` | f32 | `0.4` | Weight for FTS5 full-text search scoring (0.0-1.0) |
| `memory_vector_weight` | f32 | `0.6` | Weight for vector similarity scoring (0.0-1.0) |
| `memory_default_limit` | usize | `10` | Default number of results for memory recall queries |
| `embedding_dim` | usize | `384` | Dimensionality of embedding vectors |
| `embedding_cache_size` | usize | `1000` | Number of embeddings to cache in memory |

```toml
memory_fts_weight = 0.4
memory_vector_weight = 0.6
memory_default_limit = 10
embedding_dim = 384
embedding_cache_size = 1000
```

### Security

| Field | Type | Default | Description |
|---|---|---|---|
| `security_autonomy_level` | String | `"supervised"` | Agent autonomy level (`supervised`, `semi-autonomous`, `autonomous`) |
| `max_tool_retries` | u32 | `3` | Maximum retry attempts for failed tool executions |
| `security_rate_limit_max` | u32 | `60` | Maximum requests per rate limit window |
| `security_rate_limit_window_secs` | u64 | `60` | Rate limit window duration in seconds |
| `security_audit_log_capacity` | usize | `1000` | Maximum number of audit log entries in memory |

```toml
security_autonomy_level = "supervised"
max_tool_retries = 3
security_rate_limit_max = 60
security_rate_limit_window_secs = 60
security_audit_log_capacity = 1000
```

### AI Agent

| Field | Type | Default | Description |
|---|---|---|---|
| `provider_name` | String | `"openai"` | Default AI provider name. Alias: `default_provider` |
| `provider_type` | String | `"openai"` | Provider type (used for API compatibility) |
| `provider_base_url` | Option\<String\> | `null` | Custom base URL for the provider API |
| `provider_model_id` | String | `"gpt-4o"` | Default model ID. Alias: `default_model` |
| `provider_api_key_env` | Option\<String\> | `null` | Environment variable name for the API key |
| `agent_max_turns` | usize | `8` | Maximum agent turns (tool call loops) per request. Range: 1-32 |
| `agent_max_tokens` | usize | `4096` | Maximum tokens for agent responses |
| `agent_timeout_secs` | u64 | `300` | Maximum seconds for agent execution before timeout. WebSocket chat aborts the agent task and returns an error on timeout |
| `agent_system_prompt` | Option\<String\> | `null` | Additional system prompt appended to identity (never replaces it) |

```toml
provider_name = "openai"
provider_type = "openai"
provider_base_url = "https://api.openai.com/v1"
provider_model_id = "gpt-4o"
provider_api_key_env = "OPENAI_API_KEY"
agent_max_turns = 8
agent_max_tokens = 4096
agent_timeout_secs = 300
agent_system_prompt = "Always respond concisely."
```

### Identity

| Field | Type | Default | Description |
|---|---|---|---|
| `identity_name` | String | `"Zenii"` | Display name of the AI assistant |
| `identity_description` | String | `"AI-powered assistant"` | Short description of the assistant |
| `identity_dir` | Option\<String\> | `{data_dir}/identity/` | Directory containing identity/persona markdown files |

```toml
identity_name = "Zenii"
identity_description = "AI-powered assistant"
identity_dir = "/home/user/.zenii/identity"
```

### Skills

| Field | Type | Default | Description |
|---|---|---|---|
| `skills_dir` | Option\<String\> | `{data_dir}/skills/` | Directory containing skill definition files |
| `skill_max_content_size` | usize | `100000` | Maximum size in bytes for a skill's content |
| `skill_proposal_expiry_days` | u32 | `7` | Days before pending skill proposals expire |

```toml
skills_dir = "/home/user/.zenii/skills"
skill_max_content_size = 100000
skill_proposal_expiry_days = 7
```

### User Learning

| Field | Type | Default | Description |
|---|---|---|---|
| `learning_enabled` | bool | `true` | Whether the user learning system is active |
| `learning_denied_categories` | Vec\<String\> | `[]` | Categories of observations the system must not learn |
| `learning_max_observations` | usize | `10000` | Maximum number of stored user observations |
| `learning_observation_ttl_days` | u32 | `365` | Days before observations expire |
| `learning_min_confidence` | f32 | `0.5` | Minimum confidence threshold to store an observation |

```toml
learning_enabled = true
learning_denied_categories = ["medical", "financial"]
learning_max_observations = 10000
learning_observation_ttl_days = 365
learning_min_confidence = 0.5
```

### Tools

| Field | Type | Default | Description |
|---|---|---|---|
| `tool_shell_timeout_secs` | u64 | `30` | Timeout in seconds for shell command execution |
| `tool_file_read_max_lines` | usize | `10000` | Maximum lines to read from a file |
| `tool_file_search_max_results` | usize | `100` | Maximum results for file search operations |
| `tool_process_list_limit` | usize | `200` | Maximum number of processes to list |

```toml
tool_shell_timeout_secs = 30
tool_file_read_max_lines = 10000
tool_file_search_max_results = 100
tool_process_list_limit = 200
```

### Web Search

| Field | Type | Default | Description |
|---|---|---|---|
| `web_search_timeout_secs` | u64 | `30` | Timeout for web search requests |
| `web_search_max_results` | usize | `20` | Maximum number of web search results |

```toml
web_search_timeout_secs = 30
web_search_max_results = 20
```

### Context Injection

| Field | Type | Default | Description |
|---|---|---|---|
| `context_injection_enabled` | bool | `true` | Whether context injection into agent prompts is active |
| `context_summary_model_id` | String | `"gpt-4o-mini"` | Model used for generating conversation summaries |
| `context_summary_provider_id` | String | `"openai"` | Provider used for summary generation |
| `context_reinject_gap_minutes` | u32 | `30` | Minutes of inactivity before reinjecting full context |
| `context_reinject_message_count` | u32 | `20` | Number of messages before triggering context reinjection |

```toml
context_injection_enabled = true
context_summary_model_id = "gpt-4o-mini"
context_summary_provider_id = "openai"
context_reinject_gap_minutes = 30
context_reinject_message_count = 20
```

### Prompt Strategy

| Field | Type | Default | Description |
|---|---|---|---|
| `prompt_compact_identity` | bool | `true` | Use compact axiom-based preamble instead of verbose prose. Reduces token usage by ~60-80% while maintaining response quality |
| `prompt_max_preamble_tokens` | usize | `1500` | Token budget for system preamble. Overflow trims lowest-priority dynamic context |

```toml
prompt_compact_identity = true
prompt_max_preamble_tokens = 1500
```

When `prompt_compact_identity` is `true` (default), Zenii uses a 4-layer compact format:
- **Layer 0**: Core identity (~80 tokens) -- name, version, location, OS, capabilities
- **Layer 1**: Runtime state (~60 tokens) -- date, model, session, compact reasoning axioms
- **Layer 2**: Dynamic context (variable) -- memories, user observations, skills, domain-specific details
- **Layer 3**: Overrides -- custom system prompt, conversation summary

When `false`, the legacy verbose prose mode is used (PromptComposer + ContextEngine).

The token budget (`prompt_max_preamble_tokens`) acts as overflow protection. When the assembled preamble exceeds the budget, lowest-priority dynamic context fragments are trimmed first.

### Context Management

| Field | Type | Default | Description |
|---|---|---|---|
| `context_strategy` | String | `"balanced"` | Context assembly strategy (`minimal`, `balanced`, `full`) |
| `context_max_history_messages` | usize | `20` | Maximum conversation history messages to include in context |
| `context_max_memory_results` | usize | `5` | Maximum memory recall results to include in context |
| `context_auto_extract` | bool | `true` | Whether to automatically extract key facts from conversations |
| `context_extract_interval` | usize | `3` | Extract facts every N messages |
| `context_summary_model` | String | `""` | Override model for context summarization (empty uses default) |

```toml
context_strategy = "balanced"
context_max_history_messages = 20
context_max_memory_results = 5
context_auto_extract = true
context_extract_interval = 3
context_summary_model = ""
```

### Embeddings

| Field | Type | Default | Description |
|---|---|---|---|
| `embedding_provider` | String | `"none"` | Embedding provider type: `none` (FTS5 only), `openai`, or `local` (FastEmbed) |
| `embedding_model` | String | `"BAAI/bge-small-en-v1.5"` | Model ID for embedding generation |
| `embedding_download_dir` | Option\<String\> | `null` | Directory for local embedding model downloads (defaults to data dir) |

```toml
embedding_provider = "local"
embedding_model = "BAAI/bge-small-en-v1.5"
# embedding_download_dir = "/custom/path/models"
```

### Reasoning

| Field | Type | Default | Description |
|---|---|---|---|
| `agent_max_continuations` | usize | `1` | Maximum autonomous continuation turns for the reasoning engine |
| `tool_dedup_enabled` | bool | `true` | Deduplicate identical tool calls within a single request. Uses a per-request cache keyed by `hash(tool_name + args)` |
| `agent_reasoning_guidance` | Option\<String\> | `null` | Custom reasoning instructions appended to agent system prompt |

```toml
agent_max_continuations = 1
tool_dedup_enabled = true
agent_reasoning_guidance = "Think step by step before taking actions."
```

### Plugins

| Field | Type | Default | Description |
|---|---|---|---|
| `plugins_dir` | Option\<String\> | `{data_dir}/plugins/` | Directory containing installed plugins |
| `plugin_idle_timeout_secs` | u64 | `300` | Seconds before idle plugin processes are stopped |
| `plugin_max_restart_attempts` | u32 | `3` | Maximum restart attempts for crashed plugin processes |
| `plugin_execute_timeout_secs` | u64 | `60` | Timeout for plugin tool execution |
| `plugin_auto_update` | bool | `false` | Whether to auto-update plugins on boot |

```toml
# plugins_dir = "/custom/path/plugins"
plugin_idle_timeout_secs = 300
plugin_max_restart_attempts = 3
plugin_execute_timeout_secs = 60
plugin_auto_update = false
```

### Tool Permissions

Risk-based, per-surface tool permission system. See [Architecture: Tool Permission System](./architecture#tool-permission-system-phase-19) for details.

| Field | Type | Default | Description |
|---|---|---|---|
| `tool_permissions.low_risk_default` | String | `"allowed"` | Default permission for low-risk tools |
| `tool_permissions.medium_risk_default` | String | `"allowed"` | Default permission for medium-risk tools |
| `tool_permissions.high_risk_default` | String | `"denied"` | Default permission for high-risk tools |
| `tool_permissions.overrides` | HashMap | desktop/cli/tui: all high-risk allowed | Per-surface, per-tool overrides |

```toml
[tool_permissions]
low_risk_default = "allowed"
medium_risk_default = "allowed"
high_risk_default = "denied"

[tool_permissions.overrides.telegram]
memory = "denied"
web_search = "allowed"

[tool_permissions.overrides.desktop]
shell = "allowed"
file_read = "allowed"
file_write = "allowed"
```

Permission states: `allowed`, `denied`, `ask_once` (future), `ask_always` (future).

### Channels

| Field | Type | Default | Description |
|---|---|---|---|
| `channels_enabled` | Vec\<String\> | `[]` | List of channel names to enable on startup |
| `channel_tool_policy` | HashMap\<String, Vec\<String\>\> | `{}` | Legacy per-channel tool allowlists (superseded by `tool_permissions`) |
| `telegram_polling_timeout_secs` | u32 | `30` | Telegram long-polling timeout |
| `telegram_dm_policy` | String | `"allowlist"` | Telegram DM policy (`allowlist`, `open`, `deny`) |
| `telegram_retry_min_ms` | u64 | `1000` | Minimum retry delay for Telegram API errors (milliseconds) |
| `telegram_retry_max_ms` | u64 | `60000` | Maximum retry delay for Telegram API errors (milliseconds) |
| `telegram_require_group_mention` | bool | `true` | Whether the bot must be @mentioned in group chats to respond |

```toml
channels_enabled = ["telegram", "slack"]

# Tool permissions for channels are now managed via [tool_permissions]
# See the Tool Permissions section above

telegram_polling_timeout_secs = 30
telegram_dm_policy = "allowlist"
telegram_retry_min_ms = 1000
telegram_retry_max_ms = 60000
telegram_require_group_mention = true
```

### Scheduler

| Field | Type | Default | Description |
|---|---|---|---|
| `scheduler_tick_interval_secs` | u64 | `1` | How often the scheduler checks for due jobs (seconds) |
| `scheduler_stuck_threshold_secs` | u64 | `120` | Seconds before a running job is considered stuck |
| `scheduler_error_backoff_secs` | Vec\<u64\> | `[30, 60, 300, 900, 3600]` | Exponential backoff delays for failed jobs (seconds) |
| `scheduler_max_history_per_job` | usize | `100` | Maximum execution history entries per job |
| `scheduler_agent_turn_timeout_secs` | u64 | `120` | Timeout for agent turns within scheduled jobs |
| `scheduler_heartbeat_file` | Option\<String\> | `null` | Path to heartbeat file (updated each tick for external monitoring) |

```toml
scheduler_tick_interval_secs = 1
scheduler_stuck_threshold_secs = 120
scheduler_error_backoff_secs = [30, 60, 300, 900, 3600]
scheduler_max_history_per_job = 100
scheduler_agent_turn_timeout_secs = 120
scheduler_heartbeat_file = "/tmp/zenii-heartbeat"
```

### Credentials

| Field | Type | Default | Description |
|---|---|---|---|
| `keyring_service_id` | String | `"com.sprklai.zenii"` | OS keyring service identifier for credential storage |
| `credential_file_path` | Optional\<String\> | `None` (auto: `{data_dir}/credentials.enc`) | Override path for the encrypted credential file (used when OS keyring is unavailable) |

```toml
keyring_service_id = "com.sprklai.zenii"
# credential_file_path = "/custom/path/credentials.enc"  # Optional override
```

### Self-Evolution

| Field | Type | Default | Description |
|---|---|---|---|
| `self_evolution_enabled` | bool | `true` | Whether the self-evolution system (skill proposals) is active |
| `learning_archive_threshold` | f64 | `0.3` | Confidence threshold below which observations are archived |
| `learning_archive_after_days` | u32 | `30` | Days before low-confidence observations are archived |

```toml
self_evolution_enabled = true
learning_archive_threshold = 0.3
learning_archive_after_days = 30
```

### User Profile

| Field | Type | Default | Description |
|---|---|---|---|
| `user_name` | Option\<String\> | `null` | User's display name (e.g., "John"). Used in greetings and personalization |
| `user_timezone` | Option\<String\> | `null` | IANA timezone (e.g., "America/New_York"). Auto-detected on first run |
| `user_location` | Option\<String\> | `null` | Location/region description (e.g., "New York, US"). Used for context injection |

```toml
user_name = "John"
user_timezone = "America/New_York"
user_location = "New York, US"
```

### Logging

| Field | Type | Default | Description |
|---|---|---|---|
| `log_level` | String | `"info"` | Log level for the `tracing` framework (`trace`, `debug`, `info`, `warn`, `error`) |
| `log_dir` | String | `""` (platform default) | Override log directory. Empty uses `{data_dir}/logs/` |
| `log_keep_days` | u32 | `30` | Days to retain log files before automatic cleanup |
| `usage_tracking_enabled` | bool | `true` | Enable date-rotated JSONL usage tracking |

```toml
log_level = "info"
# log_dir = "/custom/logs"       # Override log directory (default: {data_dir}/logs/)
# log_keep_days = 30             # Days to keep log files
# usage_tracking_enabled = true  # JSONL usage tracking
```

All binaries write daily-rotated diagnostic logs to the OS-appropriate data directory:

| OS | Log Directory |
|---|---|
| **Linux** | `~/.local/share/zenii/logs/` |
| **macOS** | `~/Library/Application Support/com.sprklai.zenii/logs/` |
| **Windows** | `C:\Users\{user}\AppData\Roaming\sprklai\zenii\logs\` |

Log files per binary: `daemon.log.YYYY-MM-DD`, `desktop.log.YYYY-MM-DD`, `cli.log.YYYY-MM-DD`, `tui.log.YYYY-MM-DD`. Old files are automatically cleaned up based on `log_keep_days`.

---

## Environment Variable Overrides

| Variable | Description | Maps To |
|---|---|---|
| `ZENII_TOKEN` | Gateway authentication token | `gateway_auth_token` |
| `ZENII_GATEWAY_URL` | Gateway URL override (used by CLI and desktop app to connect to an external daemon instead of starting an embedded one) | N/A (runtime override, not a config field) |

Environment variables take precedence over config file values when supported.

---

## Feature Flag Impact

Some configuration fields are only relevant when specific feature flags are enabled at compile time:

| Feature Flag | Relevant Config Fields |
|---|---|
| `local-embeddings` | `embedding_provider` (when set to `"local"`), `embedding_model`, `embedding_download_dir` |
| `channels` | `channels_enabled`, `tool_permissions` (channel surface overrides) |
| `channels-telegram` | `telegram_polling_timeout_secs`, `telegram_dm_policy`, `telegram_retry_min_ms`, `telegram_retry_max_ms`, `telegram_require_group_mention` |
| `channels-slack` | (uses `tool_permissions` for Slack surface overrides) |
| `channels-discord` | (uses `tool_permissions` for Discord surface overrides) |
| `scheduler` | `scheduler_tick_interval_secs`, `scheduler_stuck_threshold_secs`, `scheduler_error_backoff_secs`, `scheduler_max_history_per_job`, `scheduler_agent_turn_timeout_secs`, `scheduler_heartbeat_file` |

Fields can always be set in the config file regardless of feature flags -- they are simply ignored at runtime if the corresponding feature is not compiled in.

---

## Example Full Config

```toml
# Gateway
gateway_host = "127.0.0.1"
gateway_port = 18981
gateway_auth_token = "my-secret-token"
gateway_cors_origins = ["http://localhost:18971"]
ws_max_connections = 32
event_bus_capacity = 256

# Logging
log_level = "info"
# log_dir = ""              # Override log directory (default: {data_dir}/logs/)
# log_keep_days = 30        # Days to keep log files before auto-cleanup

# Database
# data_dir = "~/.local/share/zenii"  # uses platform default if unset
session_max_age_days = 90

# AI Agent
provider_name = "openai"
provider_model_id = "gpt-4o"
agent_max_turns = 8
agent_max_tokens = 4096
agent_timeout_secs = 300

# Identity
identity_name = "Zenii"
identity_description = "AI-powered assistant"

# Memory
memory_fts_weight = 0.4
memory_vector_weight = 0.6
memory_default_limit = 10
embedding_dim = 384

# Security
security_autonomy_level = "supervised"
max_tool_retries = 3
security_rate_limit_max = 60
security_rate_limit_window_secs = 60

# Tools
tool_shell_timeout_secs = 30
tool_file_read_max_lines = 10000

# Web Search
web_search_timeout_secs = 30
web_search_max_results = 20

# Embeddings
embedding_provider = "none"
embedding_model = "BAAI/bge-small-en-v1.5"

# Reasoning
agent_max_continuations = 1
tool_dedup_enabled = true

# Context
context_injection_enabled = true
context_strategy = "balanced"
context_max_history_messages = 20
context_auto_extract = true

# User Learning
learning_enabled = true
learning_max_observations = 10000
learning_min_confidence = 0.5

# User Profile
# user_name = "John"
# user_timezone = "America/New_York"
# user_location = "New York, US"

# Tool Permissions
[tool_permissions]
low_risk_default = "allowed"
medium_risk_default = "allowed"
high_risk_default = "denied"

# Channels (requires --features channels)
channels_enabled = []
telegram_dm_policy = "allowlist"

# Scheduler (requires --features scheduler)
scheduler_tick_interval_secs = 1

# Credentials
keyring_service_id = "com.sprklai.zenii"
# credential_file_path = "/custom/path/credentials.enc"  # Optional override

# Self-Evolution
self_evolution_enabled = true
skill_proposal_expiry_days = 7
```
