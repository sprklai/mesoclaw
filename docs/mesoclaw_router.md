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

## Prepopulated Models by Provider

### OpenAI
| Model | Cost Tier | Context | Best For |
|-------|-----------|---------|----------|
| `gpt-4o` | Medium | 128K | General purpose |
| `gpt-4o-mini` | Low | 128K | Fast, cost-effective |
| `gpt-4-turbo` | Medium | 128K | Legacy compatibility |
| `o3` | High | 200K | Reasoning, analysis |
| `o4-mini` | Medium | 128K | Reasoning (cheaper) |
| `gpt-3.5-turbo` | Low | 16K | Simple tasks |

### Anthropic
| Model | Cost Tier | Context | Best For |
|-------|-----------|---------|----------|
| `claude-opus-4-5-20250219` | High | 200K | Complex reasoning |
| `claude-sonnet-4-5-20250219` | Medium | 200K | Code, general |
| `claude-haiku-4-5-20251001` | Low | 200K | Fast responses |

### Google AI
| Model | Cost Tier | Context | Best For |
|-------|-----------|---------|----------|
| `gemini-2.0-flash` | Low | 1M | Fast, cost-effective |
| `gemini-1.5-pro` | Medium | 2M | General purpose |
| `gemini-1.5-flash` | Low | 1M | Quick tasks |

### Groq
| Model | Cost Tier | Context | Best For |
|-------|-----------|---------|----------|
| `llama-3.3-70b-versatile` | Low | 128K | Fast inference |
| `llama-3.1-8b-instant` | Low | 128K | Ultra-fast |
| `mixtral-8x7b-32768` | Low | 32K | Balanced |

### Ollama (Local)
| Model | Cost Tier | Context | Best For |
|-------|-----------|---------|----------|
| `llama3.2:3b` | Low | 128K | Fast local |
| `llama3.1:70b` | Medium | 128K | Quality local |
| `codellama:34b` | Medium | 16K | Code local |
| `deepseek-coder:33b` | Medium | 16K | Code local |

### OpenRouter (Multi-Provider)
| Model | Cost Tier | Context | Best For |
|-------|-----------|---------|----------|
| `anthropic/claude-sonnet-4` | Medium | 200K | Via OpenRouter |
| `openai/gpt-4o` | Medium | 128K | Via OpenRouter |
| `google/gemini-2.0-flash-exp` | Low | 1M | Via OpenRouter |
| `deepseek/deepseek-chat` | Low | 64K | Budget option |

### Vercel AI Gateway
| Model | Cost Tier | Context | Best For |
|-------|-----------|---------|----------|
| `anthropic/claude-sonnet-4-5` | Medium | 200K | Via Gateway |
| `openai/gpt-4o` | Medium | 128K | Via Gateway |
| `google/gemini-2.0-flash` | Low | 1M | Via Gateway |

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
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### Migration File

```sql
-- migrations/YYYY-MM-DD-HHMMSS-add_router_config/up.sql
CREATE TABLE router_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    active_profile TEXT NOT NULL DEFAULT 'balanced',
    custom_routes TEXT,
    task_overrides TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO router_config (id, active_profile) VALUES (1, 'balanced');
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

# Model listing
mesoclaw router models               # List all routable models
mesoclaw router models --provider <name>  # Filter by provider

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

### Phase 1: Core Infrastructure (Backend)

#### Task 1.1: Database Migration
- [ ] Create migration `add_router_config`
- [ ] Add `router_config` table to schema.rs
- [ ] Create Diesel model for RouterConfig
- [ ] Add CRUD operations in database/models/router_config.rs

**Files:**
- `src-tauri/migrations/YYYY-MM-DD-HHMMSS-add_router_config/`
- `src-tauri/src/database/schema.rs`
- `src-tauri/src/database/models/router_config.rs`

#### Task 1.2: Routing Profile Enum
- [ ] Add `RoutingProfile` enum to router.rs
- [ ] Implement profile → cost tier mapping
- [ ] Create profile-specific routing rules
- [ ] Add serialization/deserialization

**Files:**
- `src-tauri/src/ai/providers/router.rs`

#### Task 1.3: Model Registry
- [ ] Create `ModelRegistry` struct with prepopulated models
- [ ] Add provider-specific model lists
- [ ] Implement cost tier assignment per model
- [ ] Add model availability checking

**Files:**
- `src-tauri/src/ai/model_registry.rs` (new)

#### Task 1.4: Router Service
- [ ] Create `RouterService` to manage routing state
- [ ] Integrate with database for persistence
- [ ] Add profile switching logic
- [ ] Implement task override handling

**Files:**
- `src-tauri/src/services/router.rs` (new)

#### Task 1.5: Router Commands
- [ ] Implement `get_router_config_command`
- [ ] Implement `set_router_profile_command`
- [ ] Implement `get_routing_for_task_command`
- [ ] Implement `set_task_model_override_command`
- [ ] Implement `get_available_models_for_routing_command`
- [ ] Register commands in lib.rs

**Files:**
- `src-tauri/src/commands/router.rs` (new)
- `src-tauri/src/lib.rs`

### Phase 2: CLI Support

#### Task 2.1: CLI Router Subcommand
- [ ] Add `router` subcommand to CLI
- [ ] Implement `profile` subcommands (show, set)
- [ ] Implement `task` subcommands (show, set-override)
- [ ] Implement `models` subcommand (list, filter)
- [ ] Implement `status` command

**Files:**
- `src-tauri/src/cli.rs`

#### Task 2.2: CLI Output Formatting
- [ ] Add table formatting for model lists
- [ ] Add color-coded cost tier display
- [ ] Add JSON output option for scripting

**Files:**
- `src-tauri/src/cli.rs`

### Phase 3: Frontend Integration

#### Task 3.1: Router Store (Zustand)
- [ ] Create `routerStore.ts`
- [ ] Implement profile state management
- [ ] Implement task route fetching
- [ ] Implement override management

**Files:**
- `src/stores/routerStore.ts` (new)

#### Task 3.2: Router Settings UI
- [ ] Create `RouterSettings.tsx` component
- [ ] Implement profile selection radio group
- [ ] Implement task routing table
- [ ] Implement model override dropdowns
- [ ] Add to Settings page tabs

**Files:**
- `src/components/settings/RouterSettings.tsx` (new)
- `src/routes/settings.tsx`

#### Task 3.3: Chat Integration
- [ ] Add task classification to chat input
- [ ] Update chat to use router for model selection
- [ ] Add visual indicator of selected model
- [ ] Add manual model override option in chat

**Files:**
- `src/routes/chat.tsx`
- `src/components/chat/ChatInput.tsx`

### Phase 4: App-Wide Integration

#### Task 4.1: Agent Integration
- [ ] Update agent executor to use router
- [ ] Add task classification to agent commands
- [ ] Add router-based model selection

**Files:**
- `src-tauri/src/agent/executor.rs`
- `src-tauri/src/agent/commands.rs`

#### Task 4.2: Scheduler Integration
- [ ] Update scheduler executor to use router
- [ ] Add router-based model selection for jobs
- [ ] Update job creation to support task type

**Files:**
- `src-tauri/src/scheduler/executor.rs`
- `src-tauri/src/commands/scheduler.rs`

#### Task 4.3: Channel Integration
- [ ] Update channel handlers to use router
- [ ] Add task classification for incoming messages
- [ ] Add router-based model selection

**Files:**
- `src-tauri/src/channel/handlers.rs`

### Phase 5: Testing & Documentation

#### Task 5.1: Backend Tests
- [ ] Unit tests for RoutingProfile
- [ ] Unit tests for ModelRegistry
- [ ] Unit tests for RouterService
- [ ] Integration tests for router commands

**Files:**
- `src-tauri/src/ai/providers/router.rs` (tests)
- `src-tauri/src/services/router.rs` (tests)
- `src-tauri/src/commands/router.rs` (tests)

#### Task 5.2: Frontend Tests
- [ ] Tests for routerStore
- [ ] Tests for RouterSettings component
- [ ] Tests for task classification

**Files:**
- `src/stores/routerStore.test.ts`
- `src/components/settings/RouterSettings.test.tsx`

#### Task 5.3: Documentation
- [ ] Update README with router feature
- [ ] Add router section to docs/app_usage.md
- [ ] Add CLI reference to docs

**Files:**
- `README.md`
- `docs/app_usage.md`

## File Summary

### New Files

| File | Purpose |
|------|---------|
| `src-tauri/migrations/...add_router_config/` | Database migration |
| `src-tauri/src/database/models/router_config.rs` | Diesel model |
| `src-tauri/src/ai/model_registry.rs` | Prepopulated model list |
| `src-tauri/src/services/router.rs` | Router service |
| `src-tauri/src/commands/router.rs` | Tauri commands |
| `src/stores/routerStore.ts` | Frontend state |
| `src/components/settings/RouterSettings.tsx` | Settings UI |
| `src/lib/taskClassifier.ts` | Task classification |

### Modified Files

| File | Changes |
|------|---------|
| `src-tauri/src/database/schema.rs` | Add router_config table |
| `src-tauri/src/ai/providers/router.rs` | Add RoutingProfile, enhance routing |
| `src-tauri/src/cli.rs` | Add router CLI commands |
| `src-tauri/src/lib.rs` | Register router commands |
| `src/routes/settings.tsx` | Add Router tab |
| `src/routes/chat.tsx` | Integrate router for model selection |
| `src-tauri/src/agent/executor.rs` | Use router for agents |
| `src-tauri/src/scheduler/executor.rs` | Use router for jobs |

## Success Criteria

1. **Profile Switching**: User can switch between eco/balanced/premium profiles
2. **Task Routing**: Different tasks (code, analysis, creative) route to appropriate models
3. **CLI Support**: All router functions accessible from command line
4. **UI Control**: Full router configuration in Settings UI
5. **App-Wide**: Router used consistently across chat, agents, scheduler
6. **Fallback**: Graceful fallback when primary model unavailable
7. **Customization**: Users can override per-task model selection

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| Task misclassification | Simple heuristics + manual override option |
| Provider unavailability | Fallback chain in routing rules |
| Model list outdated | Configurable model list + user additions |
| Performance overhead | Cache routing decisions, async loading |

## Future Enhancements (Out of Scope)

- ML-based task classification
- Cost tracking and optimization
- A/B testing for model selection
- ClawRouter integration (x402 payment)
- Multi-model parallel execution
