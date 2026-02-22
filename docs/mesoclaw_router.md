# MesoClaw Router - Implementation Plan

## Overview

A simplified LLM routing system inspired by ClawRouter that provides:
- **Routing profiles** (eco/balanced/premium) for automatic model selection
- **Per-provider model prepopulation** with sensible defaults
- **App-wide integration** - works in chat, agents, scheduler
- **Dual interface** - UI settings and CLI commands
- **Task-based routing** - route to optimal model based on task type

## Design Philosophy

Unlike ClawRouter's complex 15-dimension scoring and cryptocurrency payment system, MesoClaw Router focuses on:

1. **Simplicity** - 3 routing profiles (eco/balanced/premium)
2. **User Control** - Full customization via UI/CLI
3. **Local-First** - All routing happens client-side, no external services
4. **Provider Flexibility** - Works with any configured provider (OpenAI, Anthropic, Groq, Ollama, etc.)

## Architecture

### Current State

The codebase already has routing infrastructure in `src-tauri/src/ai/providers/router.rs`:
- `CostTier` enum (Low/Medium/High)
- `TaskType` enum (Code/General/Fast/Creative/Analysis/Other)
- `ModelTarget` struct (provider_id + model + cost_tier)
- `ModelRouter` with task-based routing and fallback logic

**What's Missing:**
- Routing profile concept (eco/balanced/premium)
- Prepopulated model lists per provider
- CLI commands for router management
- Database persistence of router config
- Frontend UI for routing settings
- Integration into main chat/agent flows

### Target Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                    MesoClaw Router System                           │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌─────────────┐    ┌─────────────────┐    ┌───────────────────┐  │
│  │   Profile   │───▶│  Routing Rules  │───▶│   Model Target    │  │
│  │  (eco/bal/  │    │  (Task→Models)  │    │ (Provider+Model)  │  │
│  │   premium)  │    │                 │    │                   │  │
│  └─────────────┘    └─────────────────┘    └───────────────────┘  │
│         │                   │                       │              │
│         ▼                   ▼                       ▼              │
│  ┌─────────────┐    ┌─────────────────┐    ┌───────────────────┐  │
│  │   Settings  │    │   Model Registry │   │   Provider API    │  │
│  │    Store    │    │   (Prepopulated) │    │   (OpenAI, etc)   │  │
│  └─────────────┘    └─────────────────┘    └───────────────────┘  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Routing Profiles

### Profile Definitions

| Profile | Description | Cost Tier Preference | Use Case |
|---------|-------------|---------------------|----------|
| **eco** | Cost-optimized | Low (fallback: Medium) | Development, testing, budget-conscious |
| **balanced** | Quality/cost balance | Medium (fallback: Low/High) | General use (default) |
| **premium** | Maximum quality | High (fallback: Medium) | Production, critical tasks |

### Profile → Task → Model Mapping

```
Profile: ECO
├── Code: gemini-2.0-flash → gpt-4o-mini → llama-3.1-70b (Ollama)
├── General: gpt-4o-mini → gemini-2.0-flash
├── Fast: gemini-2.0-flash → gpt-4o-mini
├── Creative: gemini-2.0-flash → gpt-4o-mini
├── Analysis: gemini-2.0-flash → gpt-4o-mini
└── Fallback: gpt-4o-mini

Profile: BALANCED (default)
├── Code: claude-sonnet-4.5 → gpt-4o → gemini-2.0-flash
├── General: gpt-4o → claude-haiku-4.5
├── Fast: gemini-2.0-flash → gpt-4o-mini
├── Creative: claude-sonnet-4.5 → gpt-4o
├── Analysis: claude-sonnet-4.5 → gpt-4o
└── Fallback: gpt-4o-mini

Profile: PREMIUM
├── Code: claude-opus-4.5 → gpt-4o → o3
├── General: claude-opus-4.5 → gpt-4o
├── Fast: claude-sonnet-4.5 → gpt-4o
├── Creative: claude-opus-4.5 → gpt-4o
├── Analysis: claude-opus-4.5 → o3
└── Fallback: claude-sonnet-4.5
```

## Dynamic Model Discovery

Unlike hardcoded model lists, MesoClaw Router dynamically discovers available models from each provider at runtime. This ensures users always see up-to-date model lists including newly released models.

### Official Model Documentation (User-Configurable)

The following documentation pages are used as reference for model discovery. Users can update these URLs in settings if providers change their documentation locations:

| Provider | Documentation URL | Discovery Method |
|----------|-------------------|------------------|
| **OpenAI** | `https://developers.openai.com/api/docs/models` | API + docs parsing |
| **Anthropic** | `https://platform.claude.com/docs/en/about-claude/models/overview` | Docs parsing (no API) |
| **Google AI** | `https://ai.google.dev/gemini-api/docs/models` | Docs parsing |
| **Groq** | `https://console.groq.com/docs/models` | API + docs parsing |
| **Vercel AI Gateway** | `https://vercel.com/ai-gateway/models` | API + docs parsing |
| **OpenRouter** | `https://openrouter.ai/models` | API + docs parsing |
| **Ollama** | Local: `http://localhost:11434/api/tags` | **Existing discovery command** |

### Provider Discovery Endpoints

| Provider | Discovery Method | Endpoint | Notes |
|----------|------------------|----------|-------|
| **Ollama** | Local API | `GET http://localhost:11434/api/tags` | **Repurpose existing `discover_ollama_models_command`** |
| **Vercel AI Gateway** | OpenAI-compatible API | `GET /v1/models` | Requires API key, returns provider-prefixed models |
| **OpenRouter** | OpenAI-compatible API | `GET https://openrouter.ai/api/v1/models` | 300+ models from multiple providers |
| **OpenAI** | OpenAI API | `GET https://api.openai.com/v1/models` | Native OpenAI models |
| **Anthropic** | Static list + manual | N/A | Anthropic doesn't have a list endpoint |
| **Google AI** | Static list + manual | N/A | Google AI SDK provides model constants |
| **Groq** | OpenAI-compatible API | `GET https://api.groq.com/openai/v1/models` | OpenAI-compatible |

### User Configuration for Documentation URLs

Users can customize the documentation URLs in settings:

```typescript
// src/types/router.ts
interface ProviderDocConfig {
  providerId: string;
  docUrl: string;
  lastUpdated: string;
  autoRefresh: boolean;
}

// Default configuration
const DEFAULT_DOC_URLS: Record<string, string> = {
  openai: 'https://developers.openai.com/api/docs/models',
  anthropic: 'https://platform.claude.com/docs/en/about-claude/models/overview',
  google_ai: 'https://ai.google.dev/gemini-api/docs/models',
  groq: 'https://console.groq.com/docs/models',
  vercel_ai_gateway: 'https://vercel.com/ai-gateway/models',
  openrouter: 'https://openrouter.ai/models',
};
```

### Model Refresh UI

```
┌─────────────────────────────────────────────────────────────────────┐
│  Settings > Router > Model Sources                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Model Documentation Sources                                        │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Provider      │ Documentation URL                │ Refresh │   │
│  ├─────────────────────────────────────────────────────────────┤   │
│  │  OpenAI        │ developers.openai.com/.../models │ [Sync]  │   │
│  │  Anthropic     │ platform.claude.com/.../models   │ [Sync]  │   │
│  │  Google AI     │ ai.google.dev/.../models         │ [Sync]  │   │
│  │  Groq          │ console.groq.com/docs/models     │ [Sync]  │   │
│  │  Vercel Gateway│ vercel.com/ai-gateway/models     │ [Sync]  │   │
│  │  OpenRouter    │ openrouter.ai/models             │ [Sync]  │   │
│  │  Ollama (local)│ localhost:11434                  │ [Sync]  │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  [Sync All Models]  [Edit URLs...]  [Reset to Defaults]            │
│                                                                     │
│  Last synced: 2026-02-21 14:32:00                                  │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Model Metadata Schema (Multi-Modality Support)

```rust
/// Extended model metadata supporting multi-modality
pub struct DiscoveredModel {
    /// Unique model identifier (e.g., "anthropic/claude-sonnet-4-5")
    pub id: String,
    /// Display name for UI
    pub display_name: String,
    /// Provider that owns this model
    pub provider_id: String,
    /// Cost tier for routing decisions
    pub cost_tier: CostTier,
    /// Maximum context window in tokens
    pub context_limit: i32,
    /// Modalities supported by this model
    pub modalities: Vec<ModelModality>,
    /// Model capabilities
    pub capabilities: ModelCapabilities,
    /// Last discovery timestamp
    pub discovered_at: DateTime<Utc>,
}

/// Supported modalities for future extensibility
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelModality {
    /// Text generation (default)
    Text,
    /// Image understanding (vision)
    Image,
    /// Image generation
    ImageGeneration,
    /// Audio transcription
    AudioTranscription,
    /// Audio generation (TTS)
    AudioGeneration,
    /// Video understanding
    Video,
    /// Embedding generation
    Embedding,
}

/// Model capabilities for routing decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Supports function/tool calling
    pub tool_calling: bool,
    /// Supports structured JSON output
    pub structured_output: bool,
    /// Supports streaming responses
    pub streaming: bool,
    /// Supports system prompts
    pub system_prompt: bool,
    /// Maximum output tokens
    pub max_output_tokens: Option<i32>,
}
```

### Discovery Implementation

```rust
// src-tauri/src/ai/model_discovery.rs

/// Trait for provider-specific model discovery
#[async_trait]
pub trait ModelDiscovery: Send + Sync {
    /// Discover available models from this provider
    async fn discover_models(&self, config: &ProviderConfig) -> Result<Vec<DiscoveredModel>, DiscoveryError>;

    /// Check if discovery is available (e.g., API accessible)
    async fn is_available(&self, config: &ProviderConfig) -> bool;
}

/// Ollama discovery - uses existing implementation
pub struct OllamaDiscovery;

#[async_trait]
impl ModelDiscovery for OllamaDiscovery {
    async fn discover_models(&self, config: &ProviderConfig) -> Result<Vec<DiscoveredModel>, DiscoveryError> {
        // Calls existing discover_ollama_models_command logic
        // GET http://localhost:11434/api/tags
        // Transforms OllamaModel -> DiscoveredModel
    }
}

/// Vercel AI Gateway discovery
pub struct VercelAIGatewayDiscovery;

#[async_trait]
impl ModelDiscovery for VercelAIGatewayDiscovery {
    async fn discover_models(&self, config: &ProviderConfig) -> Result<Vec<DiscoveredModel>, DiscoveryError> {
        // GET /v1/models with API key
        // Response: { "data": [{ "id": "anthropic/claude-sonnet-4", ... }] }
        // Parses provider prefix to determine original provider
    }
}

/// OpenRouter discovery
pub struct OpenRouterDiscovery;

#[async_trait]
impl ModelDiscovery for OpenRouterDiscovery {
    async fn discover_models(&self, config: &ProviderConfig) -> Result<Vec<DiscoveredModel>, DiscoveryError> {
        // GET https://openrouter.ai/api/v1/models
        // 300+ models with pricing and capability info
        // Rich metadata including context limits and pricing
    }
}
```

### Default Model Fallbacks

When discovery fails or before discovery runs, use sensible defaults:

```rust
// Static fallback models per provider (used when API unavailable)
pub static FALLBACK_MODELS: LazyLock<HashMap<&'static str, Vec<DiscoveredModel>>> = LazyLock::new(|| {
    HashMap::from([
        ("openai", vec![
            DiscoveredModel::text_only("gpt-4o", "OpenAI", CostTier::Medium, 128_000),
            DiscoveredModel::text_only("gpt-4o-mini", "OpenAI", CostTier::Low, 128_000),
        ]),
        ("anthropic", vec![
            DiscoveredModel::text_only("claude-sonnet-4-5-20250219", "Anthropic", CostTier::Medium, 200_000),
            DiscoveredModel::text_only("claude-haiku-4-5-20251001", "Anthropic", CostTier::Low, 200_000),
        ]),
        ("google_ai", vec![
            DiscoveredModel::multimodal("gemini-2.0-flash", "Google AI", CostTier::Low, 1_000_000, vec![ModelModality::Text, ModelModality::Image]),
            DiscoveredModel::multimodal("gemini-1.5-pro", "Google AI", CostTier::Medium, 2_000_000, vec![ModelModality::Text, ModelModality::Image]),
        ]),
        // Ollama uses discovered models only (no fallback)
    ])
});
```

### Discovery Schedule

1. **On Startup**: Discover models for all active providers
2. **On Provider Add**: Discover models when new provider configured
3. **Manual Refresh**: User-triggered refresh via UI or CLI
4. **Periodic Refresh**: Optional background refresh (default: disabled)

## Database Schema

### New Table: `router_config`

```sql
CREATE TABLE router_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),  -- Singleton row
    active_profile TEXT NOT NULL DEFAULT 'balanced',
    -- JSON-stored routing rules for customization
    custom_routes TEXT,  -- JSON: { "code": ["model1", "model2"], ... }
    -- Per-task overrides
    task_overrides TEXT, -- JSON: { "code": "claude-opus-4", ... }
    -- Last model discovery timestamp
    last_discovery TEXT, -- ISO timestamp
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### New Table: `discovered_models`

Stores dynamically discovered models with multi-modality support:

```sql
CREATE TABLE discovered_models (
    id TEXT PRIMARY KEY,              -- e.g., "anthropic/claude-sonnet-4-5"
    display_name TEXT NOT NULL,       -- Human-readable name
    provider_id TEXT NOT NULL,        -- e.g., "vercel-ai-gateway"
    model_id TEXT NOT NULL,           -- Original model ID from provider
    cost_tier TEXT NOT NULL DEFAULT 'medium',  -- 'low', 'medium', 'high'
    context_limit INTEGER DEFAULT 4096,
    -- Multi-modality support
    modalities TEXT NOT NULL DEFAULT '["text"]',  -- JSON array: ["text", "image", "audio"]
    capabilities TEXT,                -- JSON: { "tool_calling": true, "streaming": true, ... }
    -- Metadata
    discovered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    UNIQUE(provider_id, model_id)     -- Prevent duplicates per provider
);

CREATE INDEX idx_discovered_models_provider ON discovered_models(provider_id);
CREATE INDEX idx_discovered_models_tier ON discovered_models(cost_tier);
CREATE INDEX idx_discovered_models_modalities ON discovered_models(modalities);
```

### Migration File

```sql
-- migrations/YYYY-MM-DD-HHMMSS-add_router_config/up.sql
CREATE TABLE router_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    active_profile TEXT NOT NULL DEFAULT 'balanced',
    custom_routes TEXT,
    task_overrides TEXT,
    last_discovery TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO router_config (id, active_profile) VALUES (1, 'balanced');

CREATE TABLE discovered_models (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    cost_tier TEXT NOT NULL DEFAULT 'medium',
    context_limit INTEGER DEFAULT 4096,
    modalities TEXT NOT NULL DEFAULT '["text"]',
    capabilities TEXT,
    discovered_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT TRUE,
    UNIQUE(provider_id, model_id)
);

CREATE INDEX idx_discovered_models_provider ON discovered_models(provider_id);
CREATE INDEX idx_discovered_models_tier ON discovered_models(cost_tier);
CREATE INDEX idx_discovered_models_modalities ON discovered_models(modalities);
```

## API Design

### Backend Commands (Rust)

```rust
// src-tauri/src/commands/router.rs

#[tauri::command]
pub async fn get_router_config_command() -> Result<RouterConfigResponse, String>;

#[tauri::command]
pub async fn set_router_profile_command(profile: String) -> Result<(), String>;

#[tauri::command]
pub async fn get_routing_for_task_command(task_type: String) -> Result<ModelTarget, String>;

#[tauri::command]
pub async fn set_task_model_override_command(task_type: String, model: String) -> Result<(), String>;

#[tauri::command]
pub async fn clear_task_override_command(task_type: String) -> Result<(), String>;

#[tauri::command]
pub async fn get_available_models_for_routing_command() -> Result<Vec<RoutableModel>, String>;
```

### CLI Commands

```bash
# Profile management
mesoclaw router profile              # Show current profile
mesoclaw router profile set <eco|balanced|premium>  # Set profile

# Task routing
mesoclaw router task <task>          # Show model for task
mesoclaw router task <task> --set <model>  # Override task model

# Model discovery
mesoclaw router discover             # Discover models for all active providers
mesoclaw router discover --provider <name>  # Discover for specific provider
mesoclaw router models               # List all discovered models
mesoclaw router models --provider <name>  # Filter by provider
mesoclaw router models --modality <text|image|audio|video>  # Filter by modality
mesoclaw router models --refresh     # Force refresh from provider APIs

# Documentation URL management
mesoclaw router urls                 # Show documentation URLs
mesoclaw router urls set <provider> <url>  # Update documentation URL
mesoclaw router urls reset           # Reset all URLs to defaults

# Routing info
mesoclaw router status               # Full routing status
mesoclaw router test "query"         # Test routing decision
```

### Frontend API (TypeScript)

```typescript
// src/stores/routerStore.ts
interface RouterStore {
  profile: 'eco' | 'balanced' | 'premium';
  taskRoutes: Record<TaskType, ModelTarget[]>;
  overrides: Record<TaskType, string>;

  // Actions
  setProfile(profile: string): Promise<void>;
  getRoutingForTask(task: TaskType): ModelTarget;
  setTaskOverride(task: TaskType, model: string): Promise<void>;
  clearTaskOverride(task: TaskType): Promise<void>;
}
```

## UI Design

### Settings Page: Router Tab

```
┌─────────────────────────────────────────────────────────────────────┐
│  Settings > Router                                                  │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Routing Profile                                                    │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  ◉ Balanced (Recommended)                                    │   │
│  │    Quality/cost balance for general use                      │   │
│  │                                                              │   │
│  │  ○ Eco                                                      │   │
│  │    Cost-optimized, uses cheaper models                       │   │
│  │                                                              │   │
│  │  ○ Premium                                                  │   │
│  │    Maximum quality, uses most capable models                 │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  Task Routing                                                       │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  Task          │ Primary Model          │ Override          │   │
│  ├─────────────────────────────────────────────────────────────┤   │
│  │  Code          │ claude-sonnet-4-5      │ [Select...]       │   │
│  │  General       │ gpt-4o                 │ [Select...]       │   │
│  │  Fast          │ gemini-2.0-flash       │ [Select...]       │   │
│  │  Creative      │ claude-sonnet-4-5      │ [Select...]       │   │
│  │  Analysis      │ claude-sonnet-4-5      │ [Select...]       │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
│  Available Models                                                   │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │  [Filter by provider ▼]           [Search models...]         │   │
│  │                                                              │   │
│  │  ● claude-opus-4-5 (Anthropic) - High tier                  │   │
│  │  ● claude-sonnet-4-5 (Anthropic) - Medium tier              │   │
│  │  ● claude-haiku-4-5 (Anthropic) - Low tier                  │   │
│  │  ● gpt-4o (OpenAI) - Medium tier                            │   │
│  │  ● gpt-4o-mini (OpenAI) - Low tier                          │   │
│  │  ...                                                        │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Integration Points

### 1. Chat Interface

```typescript
// src/routes/chat.tsx
// When user sends message, route selects model automatically

const handleSendMessage = async (message: string) => {
  const taskType = classifyTask(message);  // Simple classification
  const model = routerStore.getRoutingForTask(taskType);

  await streamChat({
    message,
    providerId: model.providerId,
    modelId: model.modelId,
  });
};
```

### 2. Agent Execution

```rust
// src-tauri/src/agent/executor.rs
// Agent uses router for model selection

pub async fn execute_agent_turn(&self, task: &str) -> Result<String> {
    let task_type = TaskType::from_type_str(task);
    let router = self.router.read().await;
    let target = router.primary_for(task_type);

    let provider = self.provider_factory.create(&target.provider_id)?;
    provider.complete(task, &target.model).await
}
```

### 3. Scheduler Jobs

```rust
// src-tauri/src/scheduler/executor.rs
// Scheduled jobs use router

pub async fn execute_job(&self, job: &ScheduledJob) -> Result<()> {
    let router = self.get_router().await;
    let target = router.primary_for(TaskType::General);

    match job.job_type {
        JobType::AgentTurn => self.run_agent(&target).await,
        JobType::Heartbeat => self.run_heartbeat(&target).await,
        // ...
    }
}
```

## Task Classification

Simple heuristic-based classification (no ML required):

```rust
pub fn classify_task(input: &str) -> TaskType {
    let lower = input.to_lowercase();

    // Code indicators
    if lower.contains("code") || lower.contains("debug") ||
       lower.contains("implement") || lower.contains("function") ||
       lower.contains("class") || lower.contains("bug") {
        return TaskType::Code;
    }

    // Analysis indicators
    if lower.contains("analyze") || lower.contains("compare") ||
       lower.contains("summarize") || lower.contains("explain") ||
       lower.contains("why") || lower.contains("how does") {
        return TaskType::Analysis;
    }

    // Creative indicators
    if lower.contains("write") || lower.contains("create") ||
       lower.contains("design") || lower.contains("brainstorm") ||
       lower.contains("idea") || lower.contains("story") {
        return TaskType::Creative;
    }

    // Fast indicators (short queries)
    if input.len() < 50 && !lower.contains("?") {
        return TaskType::Fast;
    }

    TaskType::General
}
```

## Implementation Tasks

### Phase 1: Core Infrastructure (Backend) ✅ COMPLETE

#### Task 1.1: Database Migration ✅
- [x] Create migration `add_router_config`
- [x] Add `router_config` table to schema.rs
- [x] Add `discovered_models` table to schema.rs
- [x] Create Diesel model for RouterConfig
- [x] Create Diesel model for DiscoveredModel with multi-modality support
- [x] Add CRUD operations in database/models/router_config.rs
- [x] Add CRUD operations in database/models/discovered_model.rs

**Files:**
- `src-tauri/migrations/2026-02-21-124207-add_router_config/`
- `src-tauri/src/database/schema.rs`
- `src-tauri/src/database/models/router_config.rs`
- `src-tauri/src/database/models/discovered_model.rs`

#### Task 1.2: Routing Profile Enum ✅
- [x] Add `RoutingProfile` enum to router.rs
- [x] Implement profile → cost tier mapping
- [x] Create profile-specific routing rules
- [x] Add serialization/deserialization
- [x] Add `ModelModality` enum (Text, Image, Audio, Video, Embedding)
- [x] Add `ModelCapabilities` struct

**Files:**
- `src-tauri/src/ai/providers/router.rs`

#### Task 1.3: Model Discovery Infrastructure ✅
- [x] Create `ModelDiscovery` trait for provider-specific discovery
- [x] Implement `OllamaDiscovery` (repurposed from existing discovery)
- [x] Implement `VercelAIGatewayDiscovery` (GET /v1/models)
- [x] Implement `OpenRouterDiscovery` (GET /api/v1/models)
- [x] Implement `OpenAIDiscovery` (GET /v1/models)
- [x] Implement `GroqDiscovery` (OpenAI-compatible)
- [x] Add cost tier, context limit, and vision support inference heuristics

**Files:**
- `src-tauri/src/ai/discovery/mod.rs`
- `src-tauri/src/ai/discovery/ollama.rs`
- `src-tauri/src/ai/discovery/vercel_gateway.rs`
- `src-tauri/src/ai/discovery/openrouter.rs`
- `src-tauri/src/ai/discovery/openai.rs`
- `src-tauri/src/ai/discovery/groq.rs`

#### Task 1.4: Model Registry Service ✅
- [x] Create `ModelRegistry` to manage discovered models
- [x] Implement model discovery on demand
- [x] Implement model sync to database
- [x] Add modality filtering for routing decisions
- [x] Add cost tier filtering
- [x] Add profile-based model selection

**Files:**
- `src-tauri/src/services/model_registry.rs`

#### Task 1.5: Router Service ✅
- [x] Create `RouterService` to manage routing state
- [x] Integrate with database for persistence
- [x] Integrate with ModelRegistry for model availability
- [x] Add profile switching logic
- [x] Implement task override handling
- [x] Implement modality-aware routing

**Files:**
- `src-tauri/src/services/router.rs`

#### Task 1.6: Router Commands ✅
- [x] Implement `get_router_config`
- [x] Implement `set_router_profile`
- [x] Implement `get_discovered_models`
- [x] Implement `get_discovered_models_by_provider`
- [x] Implement `discover_models`
- [x] Implement `set_task_override`
- [x] Implement `clear_task_override`
- [x] Implement `route_message`
- [x] Implement `route_message_with_modalities`
- [x] Implement `get_available_models`
- [x] Implement `is_provider_available`
- [x] Implement `reload_models`
- [x] Implement `get_model_count`
- [x] Implement `initialize_router`
- [x] Register commands in lib.rs
- [x] Add RouterState management

**Files:**
- `src-tauri/src/commands/router.rs`
- `src-tauri/src/lib.rs`

### Phase 2: CLI Support ✅ COMPLETE

#### Task 2.1: CLI Router Subcommand ✅
- [x] Add `router` subcommand to CLI
- [x] Implement `profile` subcommands (show, set)
- [x] Implement `task` subcommands (show, set-override)
- [x] Implement `models` subcommand (list, filter, refresh)
- [x] Implement `discover` subcommand (trigger discovery)
- [x] Implement `status` command

**Files:**
- `src-tauri/src/cli.rs`

### Phase 3: Frontend Integration ✅ COMPLETE

#### Task 3.1: Router Store (Zustand) ✅
- [x] Create `routerStore.ts`
- [x] Implement profile state management
- [x] Implement task route fetching
- [x] Implement override management
- [x] Implement model discovery functions
- [x] Implement routing functions
- [x] Add `initialize()` function with proper loading state management

**Files:**
- `src/stores/routerStore.ts` ✅ CREATED

#### Task 3.2: Router Settings UI ✅
- [x] Create `RouterSettings.tsx` component
- [x] Implement profile selection radio group
- [x] Implement task routing table
- [x] Implement model override dropdowns
- [x] Implement model discovery UI
- [x] Implement discovered models list with filtering
- [x] Add to Settings page tabs
- [x] Call `initialize()` on component mount

**Files:**
- `src/components/settings/RouterSettings.tsx` ✅ CREATED
- `src/routes/settings.tsx` ✅ MODIFIED

#### Task 3.3: Chat Integration (Optional Future Enhancement)
- [x] Add task classification utility (`taskClassifier.ts`)
- [ ] Update chat to use router for model selection (optional - user can enable)
- [ ] Add visual indicator of selected model
- [ ] Add manual model override option in chat

**Files:**
- `src/lib/taskClassifier.ts` ✅ CREATED
- `src/routes/chat.tsx` (future enhancement)

### Phase 4: App-Wide Integration ✅ COMPLETE

#### Task 4.1: Agent Integration ✅
- [x] Update agent executor to use router
- [x] Add task classification to agent commands
- [x] Add router-based model selection
- [x] Implement `resolve_routed_provider()` function
- [x] Implement `resolve_provider_for_model()` function
- [x] Add `start_routed_agent_session_command`

**Files:**
- `src-tauri/src/agent/agent_commands.rs` ✅ MODIFIED
- `src-tauri/src/lib.rs` ✅ MODIFIED (command registered)

#### Task 4.2: Scheduler Integration ✅
- [x] Update scheduler executor to use router
- [x] Add router-based model selection for jobs
- [x] Update job creation to support task type
- [x] Add `AgentComponents.router` and `AgentComponents.pool` fields
- [x] Wire router state in lib.rs scheduler setup

**Files:**
- `src-tauri/src/scheduler/tokio_scheduler.rs` ✅ MODIFIED
- `src-tauri/src/lib.rs` ✅ MODIFIED (router state wired)

#### Task 4.3: Channel Integration ✅ (via Agent System)
- [x] Channel messages processed through agent system
- [x] Agent system already has router integration
- [x] No separate channel handler changes needed

**Notes:**
Channels (Telegram, Discord, Slack, Matrix) receive messages and process them
through the agent system, which already has full router support via
`start_routed_agent_session_command`.

### Phase 5: Testing & Documentation ✅ COMPLETE

#### Task 5.1: Backend Tests
- [x] Unit tests for RoutingProfile
- [x] Unit tests for ModelRegistry
- [x] Unit tests for RouterService
- [x] Integration tests for router commands

**Files:**
- `src-tauri/src/ai/providers/router.rs` (76 tests passing)
- `src-tauri/src/services/router.rs` (tests)
- `src-tauri/src/commands/router.rs` (tests)

#### Task 5.2: Frontend Tests
- [x] Tests for routerStore (9 tests passing)
- [ ] Tests for RouterSettings component (optional - UI testing)
- [x] Tests for task classification (33 tests passing)

**Files:**
- `src/stores/routerStore.test.ts`
- `src/lib/taskClassifier.test.ts`

#### Task 5.3: Documentation
- [x] Update README with router feature
- [x] Add router section to docs/app_usage.md
- [x] Add CLI reference to docs

**Files:**
- `README.md` - Updated feature list with smart model router
- `docs/app_usage.md` - Added Section 5: Smart Model Router with all subsections

## File Summary

### New Files

| File | Purpose |
|------|---------|
| `src-tauri/migrations/...add_router_config/` | Database migration |
| `src-tauri/src/database/models/router_config.rs` | Diesel model for router config |
| `src-tauri/src/database/models/discovered_model.rs` | Diesel model for discovered models |
| `src-tauri/src/ai/model_discovery.rs` | Discovery trait and types |
| `src-tauri/src/ai/discovery/mod.rs` | Discovery module |
| `src-tauri/src/commands/ollama.rs` | Modified to use shared discovery types |
| `src-tauri/src/ai/discovery/vercel_gateway.rs` | Vercel AI Gateway discovery |
| `src-tauri/src/ai/discovery/openrouter.rs` | OpenRouter discovery |
| `src-tauri/src/ai/discovery/openai.rs` | OpenAI discovery |
| `src-tauri/src/ai/discovery/groq.rs` | Groq discovery |
| `src-tauri/src/ai/model_registry.rs` | Model registry service |
| `src-tauri/src/services/router.rs` | Router service |
| `src-tauri/src/commands/router.rs` | Tauri commands |
| `src/stores/routerStore.ts` | Frontend state |
| `src/components/settings/RouterSettings.tsx` | Settings UI |
| `src/lib/taskClassifier.ts` | Task classification |

### Modified Files

| File | Changes |
|------|---------|
| `src-tauri/src/database/schema.rs` | Add router_config and discovered_models tables |
| `src-tauri/src/ai/providers/router.rs` | Add RoutingProfile, multi-modality support |
| `src-tauri/src/commands/ollama.rs` | Update to use shared discovery infrastructure |
| `src-tauri/src/cli.rs` | Add router CLI commands |
| `src-tauri/src/lib.rs` | Register router commands |
| `src/routes/settings.tsx` | Add Router tab |
| `src/routes/chat.tsx` | Integrate router for model selection |
| `src-tauri/src/agent/executor.rs` | Use router for agents |
| `src-tauri/src/scheduler/executor.rs` | Use router for jobs |

## Success Criteria

1. **Profile Switching**: User can switch between eco/balanced/premium profiles
2. **Task Routing**: Different tasks (code, analysis, creative) route to appropriate models
3. **Dynamic Discovery**: Models are discovered from provider APIs and documentation, not hardcoded
4. **Multi-Modality Support**: Router can select models based on required modalities (text, image, audio, video)
5. **CLI Support**: All router functions accessible from command line
6. **UI Control**: Full router configuration in Settings UI
7. **App-Wide**: Router used consistently across chat, agents, scheduler
8. **Fallback**: Graceful fallback when primary model unavailable
9. **Customization**: Users can override per-task model selection
10. **Ollama Integration**: Uses existing discovered local models, not hardcoded list
11. **User-Configurable URLs**: Users can update documentation URLs for model discovery

## Multi-Modality Routing

### Modality Detection

The router can detect required modalities from the request context:

```rust
pub fn detect_required_modalities(request: &ChatRequest) -> Vec<ModelModality> {
    let mut modalities = vec![ModelModality::Text]; // Always need text

    // Check for images in messages
    if request.messages.iter().any(|m| m.has_images()) {
        modalities.push(ModelModality::Image);
    }

    // Check for audio attachments
    if request.attachments.iter().any(|a| a.is_audio()) {
        modalities.push(ModelModality::AudioTranscription);
    }

    // Check for video attachments
    if request.attachments.iter().any(|a| a.is_video()) {
        modalities.push(ModelModality::Video);
    }

    modalities
}
```

### Modality-Aware Routing

```rust
impl ModelRouter {
    /// Route to a model that supports all required modalities
    pub fn route_for_modalities(
        &self,
        task: TaskType,
        required_modalities: &[ModelModality],
    ) -> Option<&ModelTarget> {
        self.targets_for(task)
            .into_iter()
            .find(|target| {
                let model = self.registry.get_model(&target.model)?;
                model.supports_all_modalities(required_modalities)
            })
    }
}
```

### UI Indication

When a request requires specific modalities, the UI shows which models support them:

```
┌─────────────────────────────────────────────────────────────────────┐
│  Select Model for Image Analysis                                    │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Required modalities: 📷 Image + 📝 Text                           │
│                                                                     │
│  ● claude-sonnet-4-5 (Anthropic) - Supports 📷 📝                  │
│  ○ gpt-4o (OpenAI) - Supports 📷 📝 🔊                             │
│  ○ gemini-2.0-flash (Google) - Supports 📷 📝 🔊 🎬                │
│  ○ gpt-4o-mini (OpenAI) - Text only (incompatible)                 │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Task misclassification | Simple heuristics + manual override option |
| Provider unavailability | Fallback chain in routing rules |
| Model list outdated | Dynamic discovery from provider APIs |
| Discovery API failure | Static fallback models per provider |
| Performance overhead | Cache routing decisions, async discovery |
| Multi-modality mismatch | Modality-aware routing with UI feedback |

## Future Enhancements (Out of Scope)

- ML-based task classification
- Cost tracking and optimization
- A/B testing for model selection
- ClawRouter integration (x402 payment)
- Multi-model parallel execution
- Audio generation (TTS) routing
- Embedding model routing
- Fine-tuned model support
