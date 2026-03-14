# Zenii API Reference

> **Note**: This document was generated with AI assistance and may contain inaccuracies. If you find errors, please [report an issue](https://github.com/sprklai/zenii/issues).

## Table of Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Error Format](#error-format)
- [Error Codes](#error-codes)
- [Routes](#routes)
  - [Health](#health)
  - [Sessions](#sessions)
  - [Messages](#messages)
  - [Chat](#chat)
  - [Memory](#memory)
  - [Config](#config)
  - [Credentials](#credentials)
  - [Providers](#providers)
  - [Models](#models)
  - [Tools](#tools)
  - [System](#system)
  - [Identity](#identity)
  - [Skills](#skills)
  - [Skill Proposals](#skill-proposals)
  - [User](#user)
  - [Embeddings](#embeddings)
  - [Plugins](#plugins)
  - [Channels (Always Available)](#channels-always-available)
  - [Channels (Feature-Gated)](#channels-feature-gated)
  - [Scheduler (Feature-Gated)](#scheduler-feature-gated)
  - [WebSocket](#websocket)
- [WebSocket Protocol](#websocket-protocol)
- [Rate Limiting](#rate-limiting)

---

## Overview

**Base URL:** `http://127.0.0.1:18981`

The Zenii gateway is an axum HTTP+WebSocket server. All routes accept and return JSON unless otherwise noted. CORS is configured via the `gateway_cors_origins` config field; an empty list or `["*"]` enables permissive CORS.

### Interactive API Documentation

When built with the `api-docs` feature (enabled by default in daemon and desktop), Zenii serves interactive API documentation:

- **Scalar UI:** `http://127.0.0.1:18981/api-docs` -- interactive API explorer with try-it-out functionality
- **OpenAPI JSON:** `http://127.0.0.1:18981/api-docs/openapi.json` -- raw OpenAPI 3.1 spec

Both endpoints bypass authentication and are always accessible.

```bash
# Open interactive API docs in browser
open http://127.0.0.1:18981/api-docs

# Download the OpenAPI spec
curl http://127.0.0.1:18981/api-docs/openapi.json -o openapi.json
```

To disable API docs (reduces binary size), build without the feature:
```bash
cargo build -p zenii-daemon --no-default-features --features gateway,ai,keyring
```

## Authentication

Authentication uses bearer tokens via the `Authorization` header:

```
Authorization: Bearer <token>
```

- If `gateway_auth_token` is not set in config, all requests pass through without authentication.
- `GET /health` always bypasses authentication.
- WebSocket endpoints (`/ws/*`) also accept a `?token=<token>` query parameter as an alternative to the header.

Set the token via `ZENII_TOKEN` environment variable or `gateway_auth_token` in `config.toml`.

## Error Format

All errors return JSON with the following structure:

```json
{
  "error_code": "ZENII_*",
  "message": "Human-readable error description"
}
```

## Error Codes

| Error Code | HTTP Status | ZeniiError Variant | Description |
|---|---|---|---|
| `ZENII_NOT_FOUND` | 404 | `NotFound` | Resource not found |
| `ZENII_POLICY_DENIED` | 403 | `PolicyDenied` | Security policy blocked the action |
| `ZENII_RATE_LIMITED` | 429 | `RateLimited` | Rate limit exceeded |
| `ZENII_AUTH_REQUIRED` | 401 | `Auth` | Missing or invalid authentication |
| `ZENII_BAD_REQUEST` | 400 | `Serialization` | Invalid JSON in request body |
| `ZENII_TOML_PARSE_ERROR` | 400 | `TomlParse` | Invalid TOML syntax |
| `ZENII_CONFIG_ERROR` | 422 | `Config` | Invalid configuration value |
| `ZENII_DB_ERROR` | 503 | `Database` | Database operation failed |
| `ZENII_SQLITE_ERROR` | 503 | `Sqlite` | SQLite-level error |
| `ZENII_AGENT_ERROR` | 502 | `Agent` | AI agent call failed |
| `ZENII_HTTP_ERROR` | 502 | `Http` | Upstream HTTP request failed |
| `ZENII_TOOL_ERROR` | 500 | `Tool` | Tool execution failed |
| `ZENII_MEMORY_ERROR` | 500 | `Memory` | Memory store operation failed |
| `ZENII_EMBEDDING_ERROR` | 500 | `Embedding` | Embedding generation failed |
| `ZENII_CREDENTIAL_ERROR` | 500 | `Credential` | Credential store operation failed |
| `ZENII_GATEWAY_ERROR` | 500 | `Gateway` | Gateway internal error |
| `ZENII_IO_ERROR` | 500 | `Io` | File system I/O error |
| `ZENII_EVENT_ERROR` | 500 | `EventBus` | Event bus broadcast error |
| `ZENII_CHANNEL_ERROR` | 500 | `Channel` | Messaging channel error |
| `ZENII_CONTEXT` | 500 | `Context` | Context engine error |
| `ZENII_TOML_SERIALIZE_ERROR` | 500 | `TomlSerialize` | TOML serialization failed |
| `ZENII_P4_IDENTITY` | 500 | `Identity` | Identity system error |
| `ZENII_P4_IDENTITY_NOT_FOUND` | 404 | `IdentityNotFound` | Identity file not found |
| `ZENII_P4_SKILL` | 500 | `Skill` | Skill system error |
| `ZENII_P4_SKILL_NOT_FOUND` | 404 | `SkillNotFound` | Skill not found |
| `ZENII_P4_USER` | 500 | `User` | User learning system error |
| `ZENII_YAML_PARSE_ERROR` | 400 | `Yaml` | Invalid YAML syntax |
| `ZENII_VALIDATION` | 400 | `Validation` | Input validation failed |
| `ZENII_SCHEDULER_ERROR` | 500 | `Scheduler` | Scheduler operation failed |
| `ZENII_PLUGIN_ERROR` | 500 | `Plugin` | Plugin system error |
| `ZENII_PLUGIN_NOT_FOUND` | 404 | `PluginNotFound` | Plugin not found |
| `ZENII_INTERNAL_ERROR` | 500 | `Other` | Unclassified internal error |

---

## Routes

### Health

#### GET /health

Returns server health status. Bypasses authentication.

**Response:**
```json
{ "status": "ok" }
```

**Example:**
```bash
curl http://127.0.0.1:18981/health
```

---

### Sessions

#### POST /sessions

Create a new chat session.

**Request Body:**
```json
{ "title": "My Session" }
```

**Response (201):**
```json
{
  "id": "uuid-string",
  "title": "My Session",
  "created_at": "2026-03-08T12:00:00Z",
  "updated_at": "2026-03-08T12:00:00Z"
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:18981/sessions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title": "My Session"}'
```

#### GET /sessions

List all sessions.

**Response:**
```json
[
  {
    "id": "uuid-string",
    "title": "My Session",
    "created_at": "2026-03-08T12:00:00Z",
    "updated_at": "2026-03-08T12:00:00Z"
  }
]
```

#### GET /sessions/{id}

Get a session by ID.

**Response:** Same as single session object above.

#### PUT /sessions/{id}

Update a session title.

**Request Body:**
```json
{ "title": "New Title" }
```

**Response:** Updated session object.

#### DELETE /sessions/{id}

Delete a session and its messages.

**Response:** `204 No Content`

#### POST /sessions/{id}/generate-title

Auto-generate a title for a session based on its messages.

**Request Body:**
```json
{ "model": "gpt-4o" }
```

The `model` field is optional. If omitted, uses the default model.

**Response:** Updated session object with generated title.

---

### Messages

#### GET /sessions/{id}/messages

Get all messages in a session, including tool call records for assistant messages.

**Response:**
```json
[
  {
    "id": "msg-uuid",
    "session_id": "session-uuid",
    "role": "user",
    "content": "Hello",
    "created_at": "2026-03-08T12:00:00Z"
  },
  {
    "id": "msg-uuid-2",
    "session_id": "session-uuid",
    "role": "assistant",
    "content": "Hi there!",
    "created_at": "2026-03-08T12:00:01Z",
    "tool_calls": [
      {
        "call_id": "call-uuid",
        "tool_name": "web_search",
        "args": {"query": "example"},
        "output": "results...",
        "success": true,
        "duration_ms": 1200
      }
    ]
  }
]
```

**Example:**
```bash
curl http://127.0.0.1:18981/sessions/SESSION_ID/messages \
  -H "Authorization: Bearer $TOKEN"
```

#### POST /sessions/{id}/messages

Add a message to a session (manual insertion, no AI response).

**Request Body:**
```json
{
  "role": "user",
  "content": "Hello, world"
}
```

**Response (201):** Created message object.

---

### Chat

#### POST /chat

Send a prompt and receive an AI response. Optionally associates with a session.

**Request Body:**
```json
{
  "prompt": "What is the weather today?",
  "session_id": "optional-session-uuid",
  "model": "optional-model-id"
}
```

**Response:**
```json
{
  "response": "I don't have access to real-time weather data...",
  "session_id": "session-uuid"
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:18981/chat \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Hello!", "session_id": null}'
```

---

### Memory

#### POST /memory

Store a new memory entry.

**Request Body:**
```json
{
  "key": "user_preference_theme",
  "content": "User prefers dark mode",
  "category": "core"
}
```

The `category` field is optional and defaults to `"core"`.

**Response:** `201 Created`

**Example:**
```bash
curl -X POST http://127.0.0.1:18981/memory \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"key": "pref_theme", "content": "User likes dark mode"}'
```

#### GET /memory

Recall memories matching a query.

**Query Parameters:**
| Param | Type | Default | Description |
|---|---|---|---|
| `q` | string | `""` | Search query (FTS5 + vector hybrid) |
| `limit` | integer | `memory_default_limit` (10) | Max results |
| `offset` | integer | 0 | Pagination offset |

**Response:**
```json
[
  {
    "key": "pref_theme",
    "content": "User likes dark mode",
    "category": "core",
    "created_at": "2026-03-08T12:00:00Z",
    "score": 0.85
  }
]
```

#### GET /memory/{key}

Get a specific memory entry by exact key.

**Response:** Single memory entry object.

#### PUT /memory/{key}

Update (upsert) a memory entry.

**Request Body:**
```json
{
  "content": "Updated content",
  "category": "core"
}
```

#### DELETE /memory/{key}

Delete a memory entry by key.

**Response:** `204 No Content`

---

### Config

#### GET /config

Get current configuration with secrets redacted and default paths resolved.

**Response:** Full `AppConfig` object as JSON. The `gateway_auth_token` field is always `null` in responses.

**Example:**
```bash
curl http://127.0.0.1:18981/config \
  -H "Authorization: Bearer $TOKEN"
```

#### PUT /config

Update configuration. Accepts a partial or full `AppConfig` object.

**Request Body:** Any subset of `AppConfig` fields as JSON.

**Response:** `200 OK` with the updated config.

---

### Setup

#### GET /setup/status

Check first-run onboarding status. **No authentication required.**

Returns whether the user needs to complete onboarding (missing provider API key, name, or location).

**Response:**
```json
{
  "needs_setup": true,
  "missing": ["user_name", "user_location", "api_key"],
  "detected_timezone": "America/New_York",
  "has_usable_model": false
}
```

| Field | Type | Description |
|-------|------|-------------|
| `needs_setup` | bool | `true` if any required field is missing |
| `missing` | string[] | List of missing fields (e.g., `"user_name"`, `"user_location"`, `"api_key"`) |
| `detected_timezone` | string or null | Auto-detected IANA timezone from the server OS |
| `has_usable_model` | bool | `true` if at least one provider has a stored API key |

**Example:**
```bash
curl http://127.0.0.1:18981/setup/status
```

---

### Credentials

#### POST /credentials

Store a credential (key-value pair) in the secure store (OS keyring or in-memory fallback).

**Request Body:**
```json
{
  "key": "api_key:openai",
  "value": "sk-..."
}
```

**Response:**
```json
{ "ok": true }
```

**Example:**
```bash
curl -X POST http://127.0.0.1:18981/credentials \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"key": "api_key:openai", "value": "sk-your-key-here"}'
```

#### GET /credentials

List stored credential keys (names only, values are never exposed in this endpoint).

**Response:**
```json
["api_key:openai", "api_key:anthropic", "channel:telegram:token"]
```

#### DELETE /credentials/{key}

Delete a credential.

**Response:**
```json
{ "deleted": true }
```

#### GET /credentials/{key}/value

Get the actual credential value. Use with caution -- this reveals the secret.

**Response:**
```json
{
  "key": "api_key:openai",
  "value": "sk-..."
}
```

Returns `404 ZENII_NOT_FOUND` if the key does not exist.

#### GET /credentials/{key}/exists

Check whether a credential exists without revealing its value.

**Response:**
```json
{ "exists": true }
```

---

### Providers

#### GET /providers

List all AI providers with their models.

**Response:**
```json
[
  {
    "id": "openai",
    "name": "OpenAI",
    "base_url": "https://api.openai.com/v1",
    "requires_api_key": true,
    "is_builtin": true,
    "models": [
      { "model_id": "gpt-4o", "display_name": "GPT-4o" }
    ]
  }
]
```

**Example:**
```bash
curl http://127.0.0.1:18981/providers \
  -H "Authorization: Bearer $TOKEN"
```

#### POST /providers

Create a user-defined provider.

**Request Body:**
```json
{
  "id": "local-ollama",
  "name": "Local Ollama",
  "base_url": "http://localhost:11434/v1",
  "requires_api_key": false,
  "models": [
    { "model_id": "llama3", "display_name": "Llama 3" }
  ]
}
```

**Response:**
```json
{ "ok": true }
```

#### GET /providers/with-key-status

List all providers with a `has_api_key` boolean indicating whether a credential is stored.

**Response:** Array of provider objects with an additional `has_api_key` field.

#### GET /providers/default

Get the global default model.

**Response:**
```json
{
  "provider_id": "openai",
  "model_id": "gpt-4o"
}
```

Returns `null` if no default is set.

#### PUT /providers/default

Set the global default model.

**Request Body:**
```json
{
  "provider_id": "openai",
  "model_id": "gpt-4o"
}
```

**Response:**
```json
{ "ok": true }
```

#### GET /providers/{id}

Get a specific provider by ID.

**Response:** Single provider object.

#### PUT /providers/{id}

Update a provider's base URL.

**Request Body:**
```json
{ "base_url": "https://new-url.example.com/v1" }
```

#### DELETE /providers/{id}

Delete a user-defined provider. Built-in providers cannot be deleted.

#### POST /providers/{id}/test

Test connectivity to a provider's API.

**Response:** Connection test result.

#### POST /providers/{id}/models

Add a model to a provider.

**Request Body:**
```json
{
  "model_id": "gpt-4o-mini",
  "display_name": "GPT-4o Mini"
}
```

#### DELETE /providers/{id}/models/{model_id}

Delete a model from a provider.

---

### Models

#### GET /models

List available models from the current config.

**Response:**
```json
[
  {
    "id": "gpt-4o",
    "provider": "openai"
  }
]
```

**Example:**
```bash
curl http://127.0.0.1:18981/models \
  -H "Authorization: Bearer $TOKEN"
```

---

### Tools

#### GET /tools

List all registered tools.

**Response:** Array of tool descriptors (name, description, parameters schema).

**Example:**
```bash
curl http://127.0.0.1:18981/tools \
  -H "Authorization: Bearer $TOKEN"
```

#### POST /tools/{name}/execute

Execute a tool by name.

**Request Body:**
```json
{
  "args": {
    "query": "rust async programming"
  }
}
```

**Response:** Tool execution result (varies by tool).

---

### System

#### GET /system/info

Get host system information.

**Response:**
```json
{
  "os": "Ubuntu",
  "os_version": "24.04",
  "hostname": "workstation",
  "cpu_count": 8,
  "total_memory_bytes": 17179869184,
  "used_memory_bytes": 8589934592
}
```

**Example:**
```bash
curl http://127.0.0.1:18981/system/info \
  -H "Authorization: Bearer $TOKEN"
```

---

### Identity

#### GET /identity

List all identity/persona files.

**Response:**
```json
{
  "files": [
    { "name": "SOUL", "description": "Core personality", "is_default": true },
    { "name": "IDENTITY", "description": "Role definition", "is_default": true },
    { "name": "USER", "description": "User context", "is_default": true }
  ]
}
```

**Example:**
```bash
curl http://127.0.0.1:18981/identity \
  -H "Authorization: Bearer $TOKEN"
```

#### POST /identity/reload

Reload identity files from disk.

**Response:** `200 OK`

#### GET /identity/{name}

Get an identity file's content.

**Response:**
```json
{
  "name": "SOUL",
  "content": "# Soul\n\nYou are Zenii...",
  "is_default": true
}
```

#### PUT /identity/{name}

Update an identity file's content.

**Request Body:**
```json
{ "content": "# Soul\n\nUpdated personality..." }
```

---

### Skills

#### GET /skills

List all skills. Optional `?category=` query parameter to filter.

**Response:**
```json
{
  "skills": [
    { "id": "system-prompt", "category": "system", "description": "..." },
    { "id": "summarize", "category": "utility", "description": "..." }
  ]
}
```

**Example:**
```bash
curl http://127.0.0.1:18981/skills \
  -H "Authorization: Bearer $TOKEN"
```

#### POST /skills

Create a user-defined skill.

**Request Body:**
```json
{
  "id": "my-custom-skill",
  "content": "---\ncategory: utility\ndescription: My skill\n---\n\nSkill content..."
}
```

**Response:** Created skill object.

#### POST /skills/reload

Reload skills from disk.

**Response:** `200 OK`

#### GET /skills/{id}

Get a full skill definition by ID.

**Response:** Complete skill object with content.

#### PUT /skills/{id}

Update a skill's content.

**Request Body:**
```json
{ "content": "Updated skill content..." }
```

**Response:** Updated skill object.

#### DELETE /skills/{id}

Delete a user-defined skill.

**Response:**
```json
{ "status": "deleted" }
```

---

### Skill Proposals

Skill proposals are generated by the self-evolution system. They suggest creating, updating, or deleting skills and require human approval.

#### GET /skills/proposals

List pending skill proposals.

**Response:**
```json
[
  {
    "id": "proposal-uuid",
    "action": "create",
    "skill_name": "new-skill",
    "content": "Proposed skill content...",
    "rationale": "Observed recurring pattern...",
    "status": "pending",
    "created_at": "2026-03-08T12:00:00Z",
    "resolved_at": null
  }
]
```

**Example:**
```bash
curl http://127.0.0.1:18981/skills/proposals \
  -H "Authorization: Bearer $TOKEN"
```

#### POST /skills/proposals/{id}/approve

Approve and execute a proposal (creates, updates, or deletes the skill).

**Response:** Result of the applied action.

#### POST /skills/proposals/{id}/reject

Reject a proposal.

**Response:** Updated proposal with `status: "rejected"`.

#### DELETE /skills/proposals/{id}

Delete a proposal record.

**Response:** `200 OK`

---

### User

#### GET /user/observations

List user observations (learned preferences and facts). Optional `?category=` filter.

**Response:**
```json
{
  "observations": [
    {
      "category": "preference",
      "key": "theme",
      "value": "dark",
      "confidence": 0.8,
      "created_at": "2026-03-08T12:00:00Z"
    }
  ]
}
```

**Example:**
```bash
curl http://127.0.0.1:18981/user/observations \
  -H "Authorization: Bearer $TOKEN"
```

#### POST /user/observations

Add a user observation.

**Request Body:**
```json
{
  "category": "preference",
  "key": "language",
  "value": "English",
  "confidence": 0.9
}
```

The `confidence` field defaults to `0.5` if omitted.

**Response:**
```json
{ "status": "observed" }
```

#### DELETE /user/observations

Clear all user observations.

#### GET /user/observations/{key}

Get a specific observation by key.

**Response:** Single observation object. Returns `404` if not found.

#### DELETE /user/observations/{key}

Delete a specific observation by key.

#### GET /user/profile

Get the composed user profile context string (used for agent prompts).

**Response:**
```json
{ "context": "User preferences: theme=dark, language=English..." }
```

---

### Embeddings

#### GET /embeddings/status

Get the current embedding provider configuration and status.

**Response:**
```json
{
  "provider": "local",
  "model": "BAAI/bge-small-en-v1.5",
  "active": true
}
```

If no provider is configured:
```json
{
  "provider": "none",
  "model": null,
  "active": false
}
```

**Example:**
```bash
curl http://127.0.0.1:18981/embeddings/status \
  -H "Authorization: Bearer $TOKEN"
```

#### POST /embeddings/test

Test embedding generation with the current provider.

**Request Body:**
```json
{
  "text": "Test embedding generation"
}
```

**Response:**
```json
{
  "success": true,
  "dimensions": 384,
  "provider": "local"
}
```

#### POST /embeddings/embed

Generate an embedding vector for the given text.

**Request Body:**
```json
{
  "text": "Text to embed"
}
```

**Response:**
```json
{
  "vector": [0.123, -0.456, ...],
  "dimensions": 384
}
```

#### POST /embeddings/download

Download a local embedding model (for `local` provider only).

**Request Body:**
```json
{
  "model": "BAAI/bge-small-en-v1.5"
}
```

**Response:**
```json
{
  "status": "downloaded",
  "model": "BAAI/bge-small-en-v1.5"
}
```

#### POST /embeddings/reindex

Re-embed all stored memories with the current provider. Useful after switching providers.

**Response:**
```json
{
  "reindexed": 42,
  "provider": "local"
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:18981/embeddings/reindex \
  -H "Authorization: Bearer $TOKEN"
```

---

### Plugins

#### GET /plugins

Returns all installed plugins with their status.

**Response:**
```json
[
  {
    "name": "weather",
    "version": "1.0.0",
    "description": "Weather forecast tool",
    "enabled": true,
    "tools": ["get_weather"],
    "skills": ["weather-prompt"]
  }
]
```

#### POST /plugins/install

Install a plugin from a git URL or local path.

**Request Body:**
```json
{
  "source": "https://github.com/user/weather-plugin",
  "local": false
}
```

**Response (201):**
```json
{
  "name": "weather",
  "version": "1.0.0",
  "installed": true
}
```

#### GET /plugins/{name}

Returns details for a specific plugin.

**Response:**
```json
{
  "name": "weather",
  "version": "1.0.0",
  "description": "Weather forecast tool",
  "author": "example",
  "enabled": true,
  "tools": ["get_weather"],
  "skills": ["weather-prompt"],
  "permissions": {
    "network": true,
    "filesystem": false
  }
}
```

#### DELETE /plugins/{name}

Removes an installed plugin and unregisters its tools and skills.

**Response:** `204 No Content`

#### PUT /plugins/{name}/toggle

Enable or disable a plugin.

**Request Body:**
```json
{
  "enabled": true
}
```

**Response:** `200 OK`

#### POST /plugins/{name}/update

Update a git-sourced plugin to the latest version.

**Response:**
```json
{
  "name": "weather",
  "version": "1.1.0",
  "updated": true
}
```

#### GET /plugins/{name}/config

Returns the plugin's configuration values.

**Response:**
```json
{
  "api_key_source": "env",
  "cache_ttl": 300
}
```

#### PUT /plugins/{name}/config

Update a plugin's configuration.

**Request Body:**
```json
{
  "cache_ttl": 600
}
```

**Response:** `200 OK`

---

### Channels (Always Available)

This endpoint is always available regardless of the `channels` feature flag.

#### POST /channels/{name}/test

Test channel credentials by calling the provider API. Works without the channels feature compiled in.

Supported channel names: `telegram`, `slack`, `discord`, `matrix`.

**Response:**
```json
{
  "channel": "telegram",
  "healthy": true,
  "latency_ms": 245
}
```

On failure:
```json
{
  "channel": "telegram",
  "healthy": false,
  "error": "Bot token not configured"
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:18981/channels/telegram/test \
  -H "Authorization: Bearer $TOKEN"
```

---

### Channels (Feature-Gated)

These routes require the `channels` feature flag to be enabled at compile time.

#### GET /channels

List registered channels with their status.

**Response:**
```json
[
  { "name": "telegram", "status": "connected" },
  { "name": "slack", "status": "disconnected" }
]
```

**Example:**
```bash
curl http://127.0.0.1:18981/channels \
  -H "Authorization: Bearer $TOKEN"
```

#### GET /channels/{name}/status

Get a single channel's status.

**Response:**
```json
{ "name": "telegram", "status": "connected" }
```

#### POST /channels/{name}/send

Send a message through a channel.

**Request Body:**
```json
{
  "content": "Hello from Zenii!",
  "recipient": "optional-recipient-id"
}
```

**Response:** `200 OK`

#### POST /channels/{name}/connect

Connect (start) a channel.

**Response:** `200 OK` or `501 Not Implemented`

#### POST /channels/{name}/disconnect

Disconnect (stop) a channel.

**Response:** `200 OK` or `501 Not Implemented`

#### GET /channels/{name}/health

Health check for a specific channel.

**Response:**
```json
{ "name": "telegram", "healthy": true }
```

#### POST /channels/{name}/message

Webhook endpoint for receiving inbound messages from a channel connector.

**Request Body:** `ChannelMessage` object (varies by channel implementation).

#### GET /channels/sessions

List all channel-originated sessions.

**Response:**
```json
[
  {
    "session_id": "uuid",
    "channel": "telegram",
    "thread_id": "chat-123",
    "created_at": "2026-03-10T12:00:00Z"
  }
]
```

#### GET /channels/sessions/{id}/messages

Get messages for a channel session.

**Response:** Array of message objects (same format as `/sessions/{id}/messages`).

---

### Scheduler (Feature-Gated)

These routes require the `scheduler` feature flag to be enabled at compile time.

#### GET /scheduler/jobs

List all scheduled jobs.

**Response:**
```json
[
  {
    "id": "job-uuid",
    "name": "daily-summary",
    "schedule": "0 9 * * *",
    "enabled": true,
    "prompt": "Generate a daily summary",
    "last_run": "2026-03-08T09:00:00Z",
    "next_run": "2026-03-09T09:00:00Z"
  }
]
```

**Example:**
```bash
curl http://127.0.0.1:18981/scheduler/jobs \
  -H "Authorization: Bearer $TOKEN"
```

#### POST /scheduler/jobs

Create a new scheduled job.

**Request Body:**
```json
{
  "name": "daily-summary",
  "schedule": "0 9 * * *",
  "prompt": "Generate a daily summary of system events",
  "enabled": true
}
```

**Response (201):**
```json
{ "id": "generated-job-uuid" }
```

#### PUT /scheduler/jobs/{id}/toggle

Toggle a job's enabled/disabled state.

**Response:**
```json
{ "id": "job-uuid", "enabled": false }
```

#### DELETE /scheduler/jobs/{id}

Delete a scheduled job.

**Response:** `204 No Content`

#### GET /scheduler/jobs/{id}/history

Get execution history for a job.

**Response:** Array of `JobExecution` objects with timestamps, status, and output.

#### GET /scheduler/status

Get scheduler status.

**Response:**
```json
{
  "running": true,
  "job_count": 3
}
```

---

### WebSocket

#### GET /ws/chat

WebSocket endpoint for streaming chat. See [WebSocket Protocol](#websocket-protocol) below.

**Authentication:** Use `?token=<token>` query parameter or `Authorization: Bearer <token>` header.

**Example (websocat):**
```bash
websocat "ws://127.0.0.1:18981/ws/chat?token=$TOKEN"
```

#### GET /ws/notifications

WebSocket endpoint for real-time notifications (scheduler events, system alerts).

**Authentication:** Same as `/ws/chat`.

---

## WebSocket Protocol

### Connection

```
ws://127.0.0.1:18981/ws/chat?token=<auth_token>
```

### Client-to-Server Message

Send a JSON message to start a chat:

```json
{
  "prompt": "Tell me about Rust",
  "session_id": "optional-session-uuid",
  "model": "optional-model-id"
}
```

### Server-to-Client Messages

All outbound messages are tagged with a `type` field:

#### `text` -- Streaming token

```json
{
  "type": "text",
  "content": "partial response text..."
}
```

#### `tool_call` -- Agent is calling a tool

```json
{
  "type": "tool_call",
  "call_id": "call-uuid",
  "tool_name": "web_search",
  "args": {"query": "example"}
}
```

#### `tool_result` -- Tool execution completed

```json
{
  "type": "tool_result",
  "call_id": "call-uuid",
  "tool_name": "web_search",
  "output": "search results...",
  "success": true,
  "duration_ms": 1200
}
```

#### `done` -- Response complete

```json
{
  "type": "done"
}
```

#### `error` -- Error occurred

```json
{
  "type": "error",
  "error": "Something went wrong"
}
```

#### `notification` -- Scheduler/system notification (via `/ws/notifications`)

```json
{
  "type": "notification",
  "event_type": "scheduler_notification",
  "job_id": "job-uuid",
  "job_name": "daily-summary",
  "message": "Job completed successfully",
  "status": null,
  "error": null
}
```

## Rate Limiting

Rate limiting is configurable via `config.toml`:

- `security_rate_limit_max`: Maximum requests per window (default: 60)
- `security_rate_limit_window_secs`: Window duration in seconds (default: 60)

When rate-limited, the server returns `429 Too Many Requests` with error code `ZENII_RATE_LIMITED`.
